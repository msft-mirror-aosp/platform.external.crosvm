// Copyright 2021 The ChromiumOS Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

mod sys;

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use anyhow::anyhow;
use anyhow::bail;
use anyhow::Context;
use base::warn;
use base::Event;
use base::Timer;
use cros_async::sync::Mutex as AsyncMutex;
use cros_async::AsyncTube;
use cros_async::EventAsync;
use cros_async::Executor;
use cros_async::ExecutorKind;
use cros_async::TimerAsync;
use data_model::DataInit;
use futures::future::AbortHandle;
use futures::future::Abortable;
use sync::Mutex;
pub use sys::start_device as run_block_device;
pub use sys::Options;
use vm_memory::GuestMemory;
use vmm_vhost::message::*;

use crate::virtio;
use crate::virtio::block::asynchronous::flush_disk;
use crate::virtio::block::asynchronous::handle_queue;
use crate::virtio::block::asynchronous::handle_vhost_user_command_tube;
use crate::virtio::block::asynchronous::BlockAsync;
use crate::virtio::block::DiskState;
use crate::virtio::copy_config;
use crate::virtio::vhost::user::device::handler::sys::Doorbell;
use crate::virtio::vhost::user::device::handler::VhostBackendReqConnection;
use crate::virtio::vhost::user::device::handler::VhostBackendReqConnectionState;
use crate::virtio::vhost::user::device::handler::VhostUserBackend;
use crate::virtio::vhost::user::device::VhostUserDevice;

const QUEUE_SIZE: u16 = 256;
const NUM_QUEUES: u16 = 16;

struct BlockBackend {
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
    backend_req_conn: Arc<Mutex<VhostBackendReqConnectionState>>,
    workers: [Option<AbortHandle>; NUM_QUEUES as usize],
}

impl VhostUserDevice for BlockAsync {
    fn max_queue_num(&self) -> usize {
        NUM_QUEUES as usize
    }

    fn into_backend(
        mut self: Box<Self>,
        ex: &Executor,
    ) -> anyhow::Result<Box<dyn VhostUserBackend>> {
        let avail_features =
            self.avail_features | VhostUserVirtioFeatures::PROTOCOL_FEATURES.bits();

        let disk_image = match self.disk_image.take() {
            Some(disk_image) => disk_image,
            None => bail!("cannot create a vhost-user backend from an empty disk image"),
        };
        let async_image = disk_image.to_async_disk(ex)?;

        let disk_state = Rc::new(AsyncMutex::new(DiskState::new(
            async_image,
            Arc::clone(&self.disk_size),
            self.read_only,
            self.sparse,
            self.id,
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
            .and_then(|t| TimerAsync::new(t, ex).context("Failed to create an async timer"))?;
        let flush_timer_armed = Rc::new(RefCell::new(false));
        ex.spawn_local(flush_disk(
            Rc::clone(&disk_state),
            flush_timer_read,
            Rc::clone(&flush_timer_armed),
        ))
        .detach();

        let backend_req_conn = Arc::new(Mutex::new(VhostBackendReqConnectionState::NoConnection));
        if let Some(control_tube) = self.control_tube.take() {
            let async_tube = AsyncTube::new(ex, control_tube)?;
            ex.spawn_local(handle_vhost_user_command_tube(
                async_tube,
                Arc::clone(&backend_req_conn),
                Rc::clone(&disk_state),
            ))
            .detach();
        }

        Ok(Box::new(BlockBackend {
            ex: ex.clone(),
            disk_state,
            disk_size: Arc::clone(&self.disk_size),
            block_size: self.block_size,
            seg_max: self.seg_max,
            avail_features,
            acked_features: 0,
            acked_protocol_features: VhostUserProtocolFeatures::empty(),
            flush_timer: flush_timer_write,
            backend_req_conn: Arc::clone(&backend_req_conn),
            flush_timer_armed,
            workers: Default::default(),
        }))
    }

    fn executor_kind(&self) -> Option<ExecutorKind> {
        Some(self.executor_kind)
    }
}

impl VhostUserBackend for BlockBackend {
    fn max_queue_num(&self) -> usize {
        NUM_QUEUES as usize
    }

    fn max_vring_len(&self) -> u16 {
        QUEUE_SIZE
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
        VhostUserProtocolFeatures::CONFIG
            | VhostUserProtocolFeatures::MQ
            | VhostUserProtocolFeatures::SLAVE_REQ
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
            BlockAsync::build_config_space(disk_size, self.seg_max, self.block_size, NUM_QUEUES)
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

    fn set_backend_req_connection(&mut self, conn: VhostBackendReqConnection) {
        let mut backend_req_conn = self.backend_req_conn.lock();

        if let VhostBackendReqConnectionState::Connected(_) = &*backend_req_conn {
            warn!("Backend Request Connection already established. Overwriting");
        }

        *backend_req_conn = VhostBackendReqConnectionState::Connected(conn);
    }
}
