// Copyright 2021 The ChromiumOS Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

pub mod sys;

use std::rc::Rc;
use std::sync::Arc;

use anyhow::anyhow;
use anyhow::bail;
use anyhow::Context;
use base::error;
use base::warn;
use base::Event;
use cros_async::sync::Mutex as AsyncMutex;
use cros_async::EventAsync;
use cros_async::Executor;
use futures::channel::mpsc;
use futures::future::AbortHandle;
use futures::future::Abortable;
use hypervisor::ProtectionType;
use once_cell::sync::OnceCell;
pub use sys::run_snd_device;
pub use sys::Options;
use vm_memory::GuestMemory;
use vmm_vhost::message::VhostUserProtocolFeatures;
use vmm_vhost::message::VhostUserVirtioFeatures;
use zerocopy::AsBytes;

use crate::virtio;
use crate::virtio::copy_config;
use crate::virtio::device_constants::snd::virtio_snd_config;
use crate::virtio::snd::common_backend::async_funcs::handle_ctrl_queue;
use crate::virtio::snd::common_backend::async_funcs::handle_pcm_queue;
use crate::virtio::snd::common_backend::async_funcs::send_pcm_response_worker;
use crate::virtio::snd::common_backend::create_stream_source_generators;
use crate::virtio::snd::common_backend::hardcoded_snd_data;
use crate::virtio::snd::common_backend::hardcoded_virtio_snd_config;
use crate::virtio::snd::common_backend::stream_info::StreamInfo;
use crate::virtio::snd::common_backend::PcmResponse;
use crate::virtio::snd::common_backend::SndData;
use crate::virtio::snd::common_backend::MAX_QUEUE_NUM;
use crate::virtio::snd::parameters::Parameters;
use crate::virtio::vhost::user::device::handler::sys::Doorbell;
use crate::virtio::vhost::user::device::handler::VhostUserBackend;

static SND_EXECUTOR: OnceCell<Executor> = OnceCell::new();

// Async workers:
// 0 - ctrl
// 1 - event
// 2 - tx
// 3 - rx
const PCM_RESPONSE_WORKER_IDX_OFFSET: usize = 2;
struct SndBackend {
    cfg: virtio_snd_config,
    avail_features: u64,
    acked_features: u64,
    acked_protocol_features: VhostUserProtocolFeatures,
    workers: [Option<AbortHandle>; MAX_QUEUE_NUM],
    response_workers: [Option<AbortHandle>; 2], // tx and rx
    snd_data: Rc<SndData>,
    streams: Rc<AsyncMutex<Vec<AsyncMutex<StreamInfo>>>>,
    tx_send: mpsc::UnboundedSender<PcmResponse>,
    rx_send: mpsc::UnboundedSender<PcmResponse>,
    tx_recv: Option<mpsc::UnboundedReceiver<PcmResponse>>,
    rx_recv: Option<mpsc::UnboundedReceiver<PcmResponse>>,
}

impl SndBackend {
    pub fn new(params: Parameters) -> anyhow::Result<Self> {
        let cfg = hardcoded_virtio_snd_config(&params);
        let avail_features = virtio::base_features(ProtectionType::Unprotected)
            | VhostUserVirtioFeatures::PROTOCOL_FEATURES.bits();

        let snd_data = hardcoded_snd_data(&params);
        let generators = create_stream_source_generators(&params, &snd_data);

        if snd_data.pcm_info_len() != generators.len() {
            error!(
                "snd: expected {} stream source generators, got {}",
                snd_data.pcm_info_len(),
                generators.len(),
            )
        }

        let streams = generators
            .into_iter()
            .map(Arc::new)
            .map(StreamInfo::new)
            .map(AsyncMutex::new)
            .collect();
        let streams = Rc::new(AsyncMutex::new(streams));

        let (tx_send, tx_recv) = mpsc::unbounded();
        let (rx_send, rx_recv) = mpsc::unbounded();

        Ok(SndBackend {
            cfg,
            avail_features,
            acked_features: 0,
            acked_protocol_features: VhostUserProtocolFeatures::empty(),
            workers: Default::default(),
            response_workers: Default::default(),
            snd_data: Rc::new(snd_data),
            streams,
            tx_send,
            rx_send,
            tx_recv: Some(tx_recv),
            rx_recv: Some(rx_recv),
        })
    }
}

impl VhostUserBackend for SndBackend {
    fn max_queue_num(&self) -> usize {
        MAX_QUEUE_NUM
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
        copy_config(data, 0, self.cfg.as_bytes(), offset)
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

        // Safe because the executor is initialized in main() below.
        let ex = SND_EXECUTOR.get().expect("Executor not initialized");

        let mut kick_evt =
            EventAsync::new(kick_evt, ex).context("failed to create EventAsync for kick_evt")?;
        let (handle, registration) = AbortHandle::new_pair();
        match idx {
            0 => {
                // ctrl queue
                let streams = self.streams.clone();
                let snd_data = self.snd_data.clone();
                let tx_send = self.tx_send.clone();
                let rx_send = self.rx_send.clone();
                ex.spawn_local(Abortable::new(
                    async move {
                        handle_ctrl_queue(
                            ex,
                            &mem,
                            &streams,
                            &snd_data,
                            &mut queue,
                            &mut kick_evt,
                            doorbell,
                            tx_send,
                            rx_send,
                            None,
                        )
                        .await
                    },
                    registration,
                ))
                .detach();
            }
            1 => {} // TODO(woodychow): Add event queue support
            2 | 3 => {
                let (send, recv) = if idx == 2 {
                    (self.tx_send.clone(), self.tx_recv.take())
                } else {
                    (self.rx_send.clone(), self.rx_recv.take())
                };
                let mut recv = recv.ok_or_else(|| anyhow!("queue restart is not supported"))?;
                let queue = Rc::new(AsyncMutex::new(queue));
                let queue2 = Rc::clone(&queue);
                let mem = Rc::new(mem);
                let mem2 = Rc::clone(&mem);
                let streams = Rc::clone(&self.streams);
                ex.spawn_local(Abortable::new(
                    async move {
                        handle_pcm_queue(&mem, &streams, send, &queue, &kick_evt, None).await
                    },
                    registration,
                ))
                .detach();

                let (handle2, registration2) = AbortHandle::new_pair();

                ex.spawn_local(Abortable::new(
                    async move {
                        send_pcm_response_worker(&mem2, &queue2, doorbell, &mut recv, None).await
                    },
                    registration2,
                ))
                .detach();

                self.response_workers[idx - PCM_RESPONSE_WORKER_IDX_OFFSET] = Some(handle2);
            }
            _ => bail!("attempted to start unknown queue: {}", idx),
        }

        self.workers[idx] = Some(handle);
        Ok(())
    }

    fn stop_queue(&mut self, idx: usize) {
        if let Some(handle) = self.workers.get_mut(idx).and_then(Option::take) {
            handle.abort();
        }
        if idx == 2 || idx == 3 {
            if let Some(handle) = self
                .response_workers
                .get_mut(idx - PCM_RESPONSE_WORKER_IDX_OFFSET)
                .and_then(Option::take)
            {
                handle.abort();
            }
        }
    }
}
