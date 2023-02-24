// Copyright 2022 The ChromiumOS Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

//! VirtioDevice implementation for the VMM side of a vhost-user connection.

use std::cell::RefCell;
use std::thread;

use anyhow::Context;
use base::error;
use base::Event;
use base::RawDescriptor;
use vm_memory::GuestMemory;
use vmm_vhost::message::VhostUserProtocolFeatures;
use vmm_vhost::message::VhostUserVirtioFeatures;

use crate::virtio::copy_config;
use crate::virtio::vhost::user::vmm::Connection;
use crate::virtio::vhost::user::vmm::Result;
use crate::virtio::vhost::user::vmm::VhostUserHandler;
use crate::virtio::DeviceType;
use crate::virtio::Interrupt;
use crate::virtio::Queue;
use crate::virtio::SharedMemoryMapper;
use crate::virtio::SharedMemoryRegion;
use crate::virtio::VirtioDevice;
use crate::Suspendable;

pub struct VhostUserVirtioDevice {
    device_type: DeviceType,
    kill_evt: Option<Event>,
    worker_thread: Option<thread::JoinHandle<()>>,
    handler: RefCell<VhostUserHandler>,
    queue_sizes: Vec<u16>,
    cfg: Option<Vec<u8>>,
    expose_shmem_descriptors_with_viommu: bool,
}

/// Method for determining the number of queues and the size of each queue.
///
/// For device types that have a fixed number of queues defined in the specification, use
/// `QueueSizes::Fixed` to specify a vector containing queue sizes with one element per queue.
///
/// Otherwise, use `QueueSizes::AskDevice`, which will query the backend using the vhost-user
/// `VHOST_USER_GET_QUEUE_NUM` message if `VHOST_USER_PROTOCOL_F_MQ` has been negotiated. If the
/// MQ feature is supported, the `queue_size` field will be used as the size of each queue up to
/// the number indicated by the backend's `GET_QUEUE_NUM` response. If the MQ feature is not
/// supported, `default_queues` will be used as the number of queues instead, and again
/// `queue_size` will be used as the size of each of these queues.
pub enum QueueSizes {
    /// Use a fixed number of queues. Each element in the `Vec` represents the size of the
    /// corresponding queue with the same index. The number of queues is determined by the length
    /// of the `Vec`.
    Fixed(Vec<u16>),
    /// Query the backend device to determine how many queues it supports.
    AskDevice {
        /// Size of each queue (number of elements in each ring).
        queue_size: u16,
        /// Default number of queues to use if the backend does not support the
        /// `VHOST_USER_PROTOCOL_F_MQ` feature.
        default_queues: usize,
    },
}

impl VhostUserVirtioDevice {
    /// Create a new VirtioDevice for a vhost-user device frontend.
    ///
    /// # Arguments
    ///
    /// - `connection`: connection to the device backend
    /// - `device_type`: virtio device type
    /// - `queue_sizes`: per-device queue size configuration
    /// - `max_queues`: maximum number of queues supported by this implementation
    /// - `allow_features`: allowed virtio device features
    /// - `allow_protocol_features`: allowed vhost-user protocol features
    /// - `base_features`: base virtio device features (e.g. `VIRTIO_F_VERSION_1`)
    /// - `cfg`: bytes to return for the virtio configuration space (queried from device if not
    ///   specified)
    pub fn new(
        connection: Connection,
        device_type: DeviceType,
        queue_sizes: QueueSizes,
        max_queues: usize,
        allow_features: u64,
        allow_protocol_features: VhostUserProtocolFeatures,
        base_features: u64,
        cfg: Option<&[u8]>,
        expose_shmem_descriptors_with_viommu: bool,
    ) -> Result<VhostUserVirtioDevice> {
        let allow_features =
            allow_features | base_features | VhostUserVirtioFeatures::PROTOCOL_FEATURES.bits();
        let init_features = base_features | VhostUserVirtioFeatures::PROTOCOL_FEATURES.bits();

        let mut handler = VhostUserHandler::new_from_connection(
            connection,
            max_queues as u64,
            allow_features,
            init_features,
            allow_protocol_features,
        )?;

        let queue_sizes = match queue_sizes {
            QueueSizes::Fixed(v) => v,
            QueueSizes::AskDevice {
                queue_size,
                default_queues,
            } => handler.queue_sizes(queue_size, default_queues)?,
        };

        Ok(VhostUserVirtioDevice {
            device_type,
            kill_evt: None,
            worker_thread: None,
            handler: RefCell::new(handler),
            queue_sizes,
            cfg: cfg.map(|cfg| cfg.to_vec()),
            expose_shmem_descriptors_with_viommu,
        })
    }
}

impl VirtioDevice for VhostUserVirtioDevice {
    fn keep_rds(&self) -> Vec<RawDescriptor> {
        Vec::new()
    }

    fn device_type(&self) -> DeviceType {
        self.device_type
    }

    fn queue_max_sizes(&self) -> &[u16] {
        &self.queue_sizes
    }

    fn features(&self) -> u64 {
        self.handler.borrow().avail_features
    }

    fn ack_features(&mut self, features: u64) {
        if let Err(e) = self.handler.borrow_mut().ack_features(features) {
            error!("failed to enable features 0x{:x}: {}", features, e);
        }
    }

    fn read_config(&self, offset: u64, data: &mut [u8]) {
        if let Some(cfg) = &self.cfg {
            copy_config(data, 0, cfg, offset);
        } else if let Err(e) = self.handler.borrow_mut().read_config(offset, data) {
            error!("failed to read config: {}", e);
        }
    }

    fn write_config(&mut self, offset: u64, data: &[u8]) {
        if let Err(e) = self.handler.borrow_mut().write_config(offset, data) {
            error!("failed to write config: {}", e);
        }
    }

    fn activate(
        &mut self,
        mem: GuestMemory,
        interrupt: Interrupt,
        queues: Vec<(Queue, Event)>,
    ) -> anyhow::Result<()> {
        let (join_handle, kill_evt) = self
            .handler
            .borrow_mut()
            .activate(mem, interrupt, queues, &format!("{}", self.device_type))
            .context("failed to activate queues")?;
        self.worker_thread = Some(join_handle);
        self.kill_evt = Some(kill_evt);
        Ok(())
    }

    fn reset(&mut self) -> bool {
        if let Err(e) = self.handler.borrow_mut().reset(self.queue_sizes.len()) {
            error!("Failed to reset device: {}", e);
            false
        } else {
            true
        }
    }

    fn get_shared_memory_region(&self) -> Option<SharedMemoryRegion> {
        match self.handler.borrow_mut().get_shared_memory_region() {
            Ok(r) => r,
            Err(e) => {
                error!("Failed to get shared memory regions {}", e);
                None
            }
        }
    }

    fn set_shared_memory_mapper(&mut self, mapper: Box<dyn SharedMemoryMapper>) {
        if let Err(e) = self.handler.borrow_mut().set_shared_memory_mapper(mapper) {
            error!("Error setting shared memory mapper {}", e);
        }
    }

    fn expose_shmem_descriptors_with_viommu(&self) -> bool {
        self.expose_shmem_descriptors_with_viommu
    }
}

impl Suspendable for VhostUserVirtioDevice {}

impl Drop for VhostUserVirtioDevice {
    fn drop(&mut self) {
        if let Some(kill_evt) = self.kill_evt.take() {
            if let Some(worker_thread) = self.worker_thread.take() {
                if let Err(e) = kill_evt.signal() {
                    error!("failed to write to kill_evt: {}", e);
                    return;
                }
                let _ = worker_thread.join();
            }
        }
    }
}
