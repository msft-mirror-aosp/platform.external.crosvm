// Copyright 2021 The ChromiumOS Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use data_model::DataInit;
use data_model::Le32;
use vmm_vhost::message::VhostUserProtocolFeatures;

use crate::virtio::fs::virtio_fs_config;
use crate::virtio::fs::FS_MAX_TAG_LEN;
use crate::virtio::fs::QUEUE_SIZE;
use crate::virtio::vhost::user::vmm::Connection;
use crate::virtio::vhost::user::vmm::Error;
use crate::virtio::vhost::user::vmm::QueueSizes;
use crate::virtio::vhost::user::vmm::Result;
use crate::virtio::vhost::user::vmm::VhostUserVirtioDevice;
use crate::virtio::DeviceType;

impl VhostUserVirtioDevice {
    pub fn new_fs(
        base_features: u64,
        connection: Connection,
        tag: &str,
    ) -> Result<VhostUserVirtioDevice> {
        if tag.len() > FS_MAX_TAG_LEN {
            return Err(Error::TagTooLong {
                len: tag.len(),
                max: FS_MAX_TAG_LEN,
            });
        }

        // The spec requires a minimum of 2 queues: one worker queue and one high priority queue
        let default_queues = 2;

        let mut cfg_tag = [0u8; FS_MAX_TAG_LEN];
        cfg_tag[..tag.len()].copy_from_slice(tag.as_bytes());

        let cfg = virtio_fs_config {
            tag: cfg_tag,
            // Only count the worker queues, exclude the high prio queue
            num_request_queues: Le32::from(default_queues - 1),
        };

        let queue_sizes = QueueSizes::AskDevice {
            queue_size: QUEUE_SIZE,
            default_queues: default_queues as usize,
        };
        let max_queues = 2;

        let allow_features = 0;

        let allow_protocol_features =
            VhostUserProtocolFeatures::MQ | VhostUserProtocolFeatures::CONFIG;

        VhostUserVirtioDevice::new(
            connection,
            DeviceType::Fs,
            queue_sizes,
            max_queues,
            allow_features,
            allow_protocol_features,
            base_features,
            Some(cfg.as_slice()),
            false,
        )
    }
}
