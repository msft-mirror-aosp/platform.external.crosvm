// Copyright 2021 The ChromiumOS Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

pub mod sys;

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use anyhow::anyhow;
use anyhow::bail;
use anyhow::Context;
use base::error;
use base::warn;
use base::Event;
use base::Tube;
use cros_async::EventAsync;
use cros_async::Executor;
use futures::future::AbortHandle;
use futures::future::Abortable;
use sync::Mutex;
pub use sys::run_gpu_device;
pub use sys::Options;
use vm_memory::GuestMemory;
use vmm_vhost::message::VhostUserProtocolFeatures;
use vmm_vhost::message::VhostUserVirtioFeatures;

use crate::virtio::gpu;
use crate::virtio::vhost::user::device::handler::sys::Doorbell;
use crate::virtio::vhost::user::device::handler::VhostBackendReqConnection;
use crate::virtio::vhost::user::device::handler::VhostBackendReqConnectionState;
use crate::virtio::vhost::user::device::handler::VhostUserBackend;
use crate::virtio::DescriptorChain;
use crate::virtio::Gpu;
use crate::virtio::Queue;
use crate::virtio::QueueReader;
use crate::virtio::SharedMemoryRegion;
use crate::virtio::VirtioDevice;

const MAX_QUEUE_NUM: usize = gpu::QUEUE_SIZES.len();
const MAX_VRING_LEN: u16 = gpu::QUEUE_SIZES[0];

#[derive(Clone)]
struct SharedReader {
    queue: Arc<Mutex<Queue>>,
    doorbell: Doorbell,
}

impl gpu::QueueReader for SharedReader {
    fn pop(&self, mem: &GuestMemory) -> Option<DescriptorChain> {
        self.queue.lock().pop(mem)
    }

    fn add_used(&self, mem: &GuestMemory, desc_index: u16, len: u32) {
        self.queue.lock().add_used(mem, desc_index, len)
    }

    fn signal_used(&self, mem: &GuestMemory) {
        self.queue.lock().trigger_interrupt(mem, &self.doorbell);
    }
}

async fn run_ctrl_queue(
    reader: SharedReader,
    mem: GuestMemory,
    kick_evt: EventAsync,
    state: Rc<RefCell<gpu::Frontend>>,
) {
    loop {
        if let Err(e) = kick_evt.next_val().await {
            error!("Failed to read kick event for ctrl queue: {}", e);
            break;
        }

        let mut state = state.borrow_mut();
        let needs_interrupt = state.process_queue(&mem, &reader);

        if needs_interrupt {
            reader.signal_used(&mem);
        }
    }
}

struct GpuBackend {
    ex: Executor,
    gpu: Rc<RefCell<Gpu>>,
    resource_bridges: Arc<Mutex<Vec<Tube>>>,
    acked_protocol_features: u64,
    state: Option<Rc<RefCell<gpu::Frontend>>>,
    fence_state: Arc<Mutex<gpu::FenceState>>,
    queue_workers: [Option<AbortHandle>; MAX_QUEUE_NUM],
    platform_workers: Rc<RefCell<Vec<AbortHandle>>>,
    backend_req_conn: VhostBackendReqConnectionState,
}

impl VhostUserBackend for GpuBackend {
    fn max_queue_num(&self) -> usize {
        MAX_QUEUE_NUM
    }

    fn max_vring_len(&self) -> u16 {
        MAX_VRING_LEN
    }

    fn features(&self) -> u64 {
        self.gpu.borrow().features() | VhostUserVirtioFeatures::PROTOCOL_FEATURES.bits()
    }

    fn ack_features(&mut self, value: u64) -> anyhow::Result<()> {
        self.gpu.borrow_mut().ack_features(value);
        Ok(())
    }

    fn acked_features(&self) -> u64 {
        self.features()
    }

    fn protocol_features(&self) -> VhostUserProtocolFeatures {
        VhostUserProtocolFeatures::CONFIG
            | VhostUserProtocolFeatures::SLAVE_REQ
            | VhostUserProtocolFeatures::MQ
            | VhostUserProtocolFeatures::SHARED_MEMORY_REGIONS
    }

    fn ack_protocol_features(&mut self, features: u64) -> anyhow::Result<()> {
        let unrequested_features = features & !self.protocol_features().bits();
        if unrequested_features != 0 {
            bail!("Unexpected protocol features: {:#x}", unrequested_features);
        }

        self.acked_protocol_features |= features;
        Ok(())
    }

    fn acked_protocol_features(&self) -> u64 {
        self.acked_protocol_features
    }

    fn read_config(&self, offset: u64, dst: &mut [u8]) {
        self.gpu.borrow().read_config(offset, dst)
    }

    fn write_config(&self, offset: u64, data: &[u8]) {
        self.gpu.borrow_mut().write_config(offset, data)
    }

    fn start_queue(
        &mut self,
        idx: usize,
        mut queue: Queue,
        mem: GuestMemory,
        doorbell: Doorbell,
        kick_evt: Event,
    ) -> anyhow::Result<()> {
        if let Some(handle) = self.queue_workers.get_mut(idx).and_then(Option::take) {
            warn!("Starting new queue handler without stopping old handler");
            handle.abort();
        }

        match idx {
            // ctrl queue.
            0 => {}
            // We don't currently handle the cursor queue.
            1 => return Ok(()),
            _ => bail!("attempted to start unknown queue: {}", idx),
        }

        // Enable any virtqueue features that were negotiated (like VIRTIO_RING_F_EVENT_IDX).
        queue.ack_features(self.acked_features());

        let kick_evt = EventAsync::new(kick_evt, &self.ex)
            .context("failed to create EventAsync for kick_evt")?;

        let reader = SharedReader {
            queue: Arc::new(Mutex::new(queue)),
            doorbell,
        };

        let state = if let Some(s) = self.state.as_ref() {
            s.clone()
        } else {
            let fence_handler =
                gpu::create_fence_handler(mem.clone(), reader.clone(), self.fence_state.clone());

            let mapper = {
                match &mut self.backend_req_conn {
                    VhostBackendReqConnectionState::Connected(request) => {
                        request.take_shmem_mapper()?
                    }
                    VhostBackendReqConnectionState::NoConnection => {
                        bail!("No backend request connection found")
                    }
                }
            };

            let state = Rc::new(RefCell::new(
                self.gpu
                    .borrow_mut()
                    .initialize_frontend(self.fence_state.clone(), fence_handler, mapper)
                    .ok_or_else(|| anyhow!("failed to initialize gpu frontend"))?,
            ));
            self.state = Some(state.clone());
            state
        };

        // Start handling platform-specific workers.
        self.start_platform_workers()?;

        // Start handling the control queue.
        let (handle, registration) = AbortHandle::new_pair();
        self.ex
            .spawn_local(Abortable::new(
                run_ctrl_queue(reader, mem, kick_evt, state),
                registration,
            ))
            .detach();

        self.queue_workers[idx] = Some(handle);
        Ok(())
    }

    fn stop_queue(&mut self, idx: usize) {
        if let Some(handle) = self.queue_workers.get_mut(idx).and_then(Option::take) {
            handle.abort();
        }
    }

    fn reset(&mut self) {
        for handle in self.platform_workers.borrow_mut().drain(..) {
            handle.abort();
        }

        for queue_num in 0..self.max_queue_num() {
            self.stop_queue(queue_num);
        }
    }

    fn get_shared_memory_region(&self) -> Option<SharedMemoryRegion> {
        self.gpu.borrow().get_shared_memory_region()
    }

    fn set_backend_req_connection(&mut self, conn: VhostBackendReqConnection) {
        if let VhostBackendReqConnectionState::Connected(_) = &self.backend_req_conn {
            warn!("connection already established. overwriting");
        }

        self.backend_req_conn = VhostBackendReqConnectionState::Connected(conn);
    }
}

impl Drop for GpuBackend {
    fn drop(&mut self) {
        // Workers are detached and will leak unless they are aborted. Aborting marks the
        // Abortable task, then wakes it up. This means the executor should be asked to continue
        // running for one more step after the backend is destroyed.
        self.reset();
    }
}
