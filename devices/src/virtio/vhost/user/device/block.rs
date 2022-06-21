// Copyright 2021 The Chromium OS Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

mod sys;

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{atomic::AtomicU64, atomic::Ordering, Arc};

use anyhow::{anyhow, bail, Context};
use base::{warn, Event, Timer};
use cros_async::{sync::Mutex as AsyncMutex, EventAsync, Executor, TimerAsync};
use data_model::DataInit;
use disk::AsyncDisk;
use futures::future::{AbortHandle, Abortable};
use sync::Mutex;
use vm_memory::GuestMemory;
use vmm_vhost::message::*;

use crate::virtio::block::asynchronous::{flush_disk, handle_queue};
use crate::virtio::block::*;
use crate::virtio::vhost::user::device::handler::{Doorbell, VhostUserBackend};
use crate::virtio::{self, block::sys::*, copy_config};

pub use sys::{start_device as run_block_device, Options};

const QUEUE_SIZE: u16 = 256;
const NUM_QUEUES: u16 = 16;

pub(crate) struct BlockBackend {
    ex: Executor,
    disk_state: Rc<AsyncMutex<DiskState>>,
    disk_size: Arc<AtomicU64>,
    block_size: u32,
    seg_max: u32,
    avail_features: u64,
    acked_features: u64,
    acked_protocol_features: VhostUserProtocolFeatures,
    flush_timer: Rc<RefCell<TimerAsync>>,
    flush_timer_armed: Rc<RefCell<bool>>,
    workers: [Option<AbortHandle>; Self::MAX_QUEUE_NUM],
}

impl BlockBackend {
    pub fn new_from_async_disk(
        ex: &Executor,
        async_image: Box<dyn AsyncDisk>,
        base_features: u64,
        read_only: bool,
        sparse: bool,
        block_size: u32,
    ) -> anyhow::Result<BlockBackend> {
        if block_size % SECTOR_SIZE as u32 != 0 {
            bail!(
                "Block size {} is not a multiple of {}.",
                block_size,
                SECTOR_SIZE,
            );
        }
        let disk_size = async_image.get_len()?;
        if disk_size % block_size as u64 != 0 {
            warn!(
                "Disk size {} is not a multiple of block size {}; \
                 the remainder will not be visible to the guest.",
                disk_size, block_size,
            );
        }

        let avail_features = build_avail_features(base_features, read_only, sparse, true)
            | VhostUserVirtioFeatures::PROTOCOL_FEATURES.bits();

        let seg_max = get_seg_max(QUEUE_SIZE);

        let disk_size = Arc::new(AtomicU64::new(disk_size));

        let disk_state = Rc::new(AsyncMutex::new(DiskState::new(
            async_image,
            Arc::clone(&disk_size),
            read_only,
            sparse,
            None, // id: Option<BlockId>,
        )));

        let timer = Timer::new().context("Failed to create a timer")?;
        let flush_timer_write = Rc::new(RefCell::new(
            TimerAsync::new(
                // Call try_clone() to share the same underlying FD with the `flush_disk` task.
                timer.try_clone().context("Failed to clone flush_timer")?,
                ex,
            )
            .context("Failed to create an async timer")?,
        ));
        // Create a separate TimerAsync with the same backing kernel timer. This allows the
        // `flush_disk` task to borrow its copy waiting for events while the queue handlers can
        // still borrow their copy momentarily to set timeouts.
        // Call try_clone() to share the same underlying FD with the `flush_disk` task.
        let flush_timer_read = timer
            .try_clone()
            .context("Failed to clone flush_timer")
            .and_then(|t| {
                // TODO(b/228645507): Update code below to match B* once B* Timer is upstreamed.
                TimerAsync::new(t, ex).context("Failed to create an async timer")
            })?;
        let flush_timer_armed = Rc::new(RefCell::new(false));
        ex.spawn_local(flush_disk(
            Rc::clone(&disk_state),
            flush_timer_read,
            Rc::clone(&flush_timer_armed),
        ))
        .detach();

        Ok(BlockBackend {
            ex: ex.clone(),
            disk_state,
            disk_size,
            block_size,
            seg_max,
            avail_features,
            acked_features: 0,
            acked_protocol_features: VhostUserProtocolFeatures::empty(),
            flush_timer: flush_timer_write,
            flush_timer_armed,
            workers: Default::default(),
        })
    }
}

impl VhostUserBackend for BlockBackend {
    const MAX_QUEUE_NUM: usize = NUM_QUEUES as usize;
    const MAX_VRING_LEN: u16 = QUEUE_SIZE;

    type Error = anyhow::Error;

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
        let config_space = {
            let disk_size = self.disk_size.load(Ordering::Relaxed);
            build_config_space(disk_size, self.seg_max, self.block_size, NUM_QUEUES)
        };
        copy_config(data, 0, config_space.as_slice(), offset);
    }

    fn reset(&mut self) {
        panic!("Unsupported call to reset");
    }

    fn start_queue(
        &mut self,
        idx: usize,
        mut queue: virtio::Queue,
        mem: GuestMemory,
        doorbell: Arc<Mutex<Doorbell>>,
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

        let disk_state = Rc::clone(&self.disk_state);
        let timer = Rc::clone(&self.flush_timer);
        let timer_armed = Rc::clone(&self.flush_timer_armed);
        self.ex
            .spawn_local(Abortable::new(
                handle_queue(
                    self.ex.clone(),
                    mem,
                    disk_state,
                    Rc::new(RefCell::new(queue)),
                    kick_evt,
                    doorbell,
                    timer,
                    timer_armed,
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
