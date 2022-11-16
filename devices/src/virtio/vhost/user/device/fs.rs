// Copyright 2021 The ChromiumOS Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

mod sys;

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::anyhow;
use anyhow::bail;
use anyhow::Context;
use argh::FromArgs;
use base::error;
use base::warn;
use base::AsRawDescriptors;
use base::Event;
use base::RawDescriptor;
use base::Tube;
use cros_async::EventAsync;
use cros_async::Executor;
use data_model::DataInit;
use data_model::Le32;
use fuse::Server;
use futures::future::AbortHandle;
use futures::future::Abortable;
use hypervisor::ProtectionType;
use sync::Mutex;
pub use sys::start_device as run_fs_device;
use vm_memory::GuestMemory;
use vmm_vhost::message::VhostUserProtocolFeatures;
use vmm_vhost::message::VhostUserVirtioFeatures;

use crate::virtio;
use crate::virtio::copy_config;
use crate::virtio::fs::passthrough::PassthroughFs;
use crate::virtio::fs::process_fs_queue;
use crate::virtio::fs::virtio_fs_config;
use crate::virtio::fs::FS_MAX_TAG_LEN;
use crate::virtio::vhost::user::device::handler::sys::Doorbell;
use crate::virtio::vhost::user::device::handler::VhostUserBackend;

const MAX_QUEUE_NUM: usize = 2; /* worker queue and high priority queue */
const MAX_VRING_LEN: u16 = 1024;

async fn handle_fs_queue(
    mut queue: virtio::Queue,
    mem: GuestMemory,
    doorbell: Doorbell,
    kick_evt: EventAsync,
    server: Arc<fuse::Server<PassthroughFs>>,
    tube: Arc<Mutex<Tube>>,
) {
    // Slot is always going to be 0 because we do not support DAX
    let slot: u32 = 0;

    loop {
        if let Err(e) = kick_evt.next_val().await {
            error!("Failed to read kick event for fs queue: {}", e);
            break;
        }
        if let Err(e) = process_fs_queue(&mem, &doorbell, &mut queue, &server, &tube, slot) {
            error!("Process FS queue failed: {}", e);
            break;
        }
    }
}

struct FsBackend {
    ex: Executor,
    server: Arc<fuse::Server<PassthroughFs>>,
    tag: [u8; FS_MAX_TAG_LEN],
    avail_features: u64,
    acked_features: u64,
    acked_protocol_features: VhostUserProtocolFeatures,
    workers: [Option<AbortHandle>; MAX_QUEUE_NUM],
    keep_rds: Vec<RawDescriptor>,
}

impl FsBackend {
    pub fn new(ex: &Executor, tag: &str) -> anyhow::Result<Self> {
        if tag.len() > FS_MAX_TAG_LEN {
            bail!(
                "fs tag is too long: {} (max supported: {})",
                tag.len(),
                FS_MAX_TAG_LEN
            );
        }
        let mut fs_tag = [0u8; FS_MAX_TAG_LEN];
        fs_tag[..tag.len()].copy_from_slice(tag.as_bytes());

        let avail_features = virtio::base_features(ProtectionType::Unprotected)
            | VhostUserVirtioFeatures::PROTOCOL_FEATURES.bits();

        // Use default passthroughfs config
        let fs = PassthroughFs::new(Default::default())?;

        let mut keep_rds: Vec<RawDescriptor> = [0, 1, 2].to_vec();
        keep_rds.append(&mut fs.keep_rds());

        let ex = ex.clone();
        keep_rds.extend(ex.as_raw_descriptors());

        let server = Arc::new(Server::new(fs));

        Ok(FsBackend {
            ex,
            server,
            tag: fs_tag,
            avail_features,
            acked_features: 0,
            acked_protocol_features: VhostUserProtocolFeatures::empty(),
            workers: Default::default(),
            keep_rds,
        })
    }
}

impl VhostUserBackend for FsBackend {
    fn max_queue_num(&self) -> usize {
        MAX_QUEUE_NUM
    }

    fn max_vring_len(&self) -> u16 {
        MAX_VRING_LEN
    }

    fn features(&self) -> u64 {
        self.avail_features
    }

    fn ack_features(&mut self, value: u64) -> anyhow::Result<()> {
        let unrequested_features = value & !self.avail_features;
        if unrequested_features != 0 {
            bail!("invalid features are given: {:#x}", unrequested_features);
        }

        self.acked_features |= value;

        Ok(())
    }

    fn acked_features(&self) -> u64 {
        self.acked_features
    }

    fn protocol_features(&self) -> VhostUserProtocolFeatures {
        VhostUserProtocolFeatures::CONFIG | VhostUserProtocolFeatures::MQ
    }

    fn ack_protocol_features(&mut self, features: u64) -> anyhow::Result<()> {
        let features = VhostUserProtocolFeatures::from_bits(features)
            .ok_or_else(|| anyhow!("invalid protocol features are given: {:#x}", features))?;
        let supported = self.protocol_features();
        self.acked_protocol_features = features & supported;
        Ok(())
    }

    fn acked_protocol_features(&self) -> u64 {
        self.acked_protocol_features.bits()
    }

    fn read_config(&self, offset: u64, data: &mut [u8]) {
        let config = virtio_fs_config {
            tag: self.tag,
            num_request_queues: Le32::from(1),
        };
        copy_config(data, 0, config.as_slice(), offset);
    }

    fn reset(&mut self) {
        for handle in self.workers.iter_mut().filter_map(Option::take) {
            handle.abort();
        }
    }

    fn start_queue(
        &mut self,
        idx: usize,
        mut queue: virtio::Queue,
        mem: GuestMemory,
        doorbell: Doorbell,
        kick_evt: Event,
    ) -> anyhow::Result<()> {
        if let Some(handle) = self.workers.get_mut(idx).and_then(Option::take) {
            warn!("Starting new queue handler without stopping old handler");
            handle.abort();
        }

        // Enable any virtqueue features that were negotiated (like VIRTIO_RING_F_EVENT_IDX).
        queue.ack_features(self.acked_features);

        let kick_evt = EventAsync::new(kick_evt, &self.ex)
            .context("failed to create EventAsync for kick_evt")?;
        let (handle, registration) = AbortHandle::new_pair();
        let (_, fs_device_tube) = Tube::pair()?;

        self.ex
            .spawn_local(Abortable::new(
                handle_fs_queue(
                    queue,
                    mem,
                    doorbell,
                    kick_evt,
                    self.server.clone(),
                    Arc::new(Mutex::new(fs_device_tube)),
                ),
                registration,
            ))
            .detach();

        self.workers[idx] = Some(handle);
        Ok(())
    }

    fn stop_queue(&mut self, idx: usize) {
        if let Some(handle) = self.workers.get_mut(idx).and_then(Option::take) {
            handle.abort();
        }
    }
}

#[derive(FromArgs)]
#[argh(subcommand, name = "fs")]
/// FS Device
pub struct Options {
    #[argh(option, arg_name = "PATH")]
    /// path to a vhost-user socket
    socket: Option<String>,
    #[argh(option, arg_name = "STRING")]
    /// VFIO-PCI device name (e.g. '0000:00:07.0')
    vfio: Option<String>,
    #[argh(option, arg_name = "TAG")]
    /// the virtio-fs tag
    tag: String,
    #[argh(option, arg_name = "DIR")]
    /// path to a directory to share
    shared_dir: PathBuf,
    #[argh(option, arg_name = "UIDMAP")]
    /// uid map to use
    uid_map: Option<String>,
    #[argh(option, arg_name = "GIDMAP")]
    /// gid map to use
    gid_map: Option<String>,
}
