// Copyright 2021 The Chromium OS Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

// virtio-sound spec: https://github.com/oasis-tcs/virtio-spec/blob/master/virtio-sound.tex

use std::fmt;
use std::io;
use std::rc::Rc;
use std::thread;

use anyhow::Context;
use audio_streams::BoxError;
use audio_streams::SampleFormat;
use audio_streams::StreamSource;
use audio_streams::StreamSourceGenerator;
use base::error;
use base::warn;
use base::Error as SysError;
use base::Event;
use base::RawDescriptor;
use cros_async::sync::Mutex as AsyncMutex;
use cros_async::AsyncError;
use cros_async::EventAsync;
use cros_async::Executor;
use data_model::DataInit;
use futures::channel::mpsc;
use futures::channel::oneshot;
use futures::channel::oneshot::Canceled;
use futures::pin_mut;
use futures::select;
use futures::Future;
use futures::FutureExt;
use futures::TryFutureExt;
use thiserror::Error as ThisError;
use vm_memory::GuestMemory;

use crate::virtio::async_utils;
use crate::virtio::copy_config;
use crate::virtio::device_constants::snd::virtio_snd_config;
use crate::virtio::snd::common::*;
use crate::virtio::snd::common_backend::async_funcs::*;
use crate::virtio::snd::constants::*;
use crate::virtio::snd::layout::*;
use crate::virtio::snd::null_backend::create_null_stream_source_generators;
use crate::virtio::snd::parameters::Parameters;
use crate::virtio::snd::parameters::StreamSourceBackend;
use crate::virtio::snd::sys::create_stream_source_generators as sys_create_stream_source_generators;
use crate::virtio::snd::sys::set_audio_thread_priority;
use crate::virtio::DescriptorChain;
use crate::virtio::DescriptorError;
use crate::virtio::DeviceType;
use crate::virtio::Interrupt;
use crate::virtio::Queue;
use crate::virtio::VirtioDevice;
use crate::virtio::Writer;

pub mod async_funcs;

// control + event + tx + rx queue
pub const MAX_QUEUE_NUM: usize = 4;
pub const MAX_VRING_LEN: u16 = 1024;

#[derive(ThisError, Debug)]
pub enum Error {
    /// next_async failed.
    #[error("Failed to read descriptor asynchronously: {0}")]
    Async(AsyncError),
    /// Creating stream failed.
    #[error("Failed to create stream: {0}")]
    CreateStream(BoxError),
    /// Creating kill event failed.
    #[error("Failed to create kill event: {0}")]
    CreateKillEvent(SysError),
    /// Creating WaitContext failed.
    #[error("Failed to create wait context: {0}")]
    CreateWaitContext(SysError),
    /// Cloning kill event failed.
    #[error("Failed to clone kill event: {0}")]
    CloneKillEvent(SysError),
    /// Descriptor chain was invalid.
    #[error("Failed to valildate descriptor chain: {0}")]
    DescriptorChain(DescriptorError),
    // Future error.
    #[error("Unexpected error. Done was not triggered before dropped: {0}")]
    DoneNotTriggered(Canceled),
    /// Error reading message from queue.
    #[error("Failed to read message: {0}")]
    ReadMessage(io::Error),
    /// Failed writing a response to a control message.
    #[error("Failed to write message response: {0}")]
    WriteResponse(io::Error),
    // Mpsc read error.
    #[error("Error in mpsc: {0}")]
    MpscSend(futures::channel::mpsc::SendError),
    // Oneshot send error.
    #[error("Error in oneshot send")]
    OneshotSend(()),
    /// Stream not found.
    #[error("stream id ({0}) < num_streams ({1})")]
    StreamNotFound(usize, usize),
    /// Fetch buffer error
    #[error("Failed to get buffer from CRAS: {0}")]
    FetchBuffer(BoxError),
    /// Invalid buffer size
    #[error("Invalid buffer size")]
    InvalidBufferSize,
    /// IoError
    #[error("I/O failed: {0}")]
    Io(io::Error),
    /// Operation not supported.
    #[error("Operation not supported")]
    OperationNotSupported,
    /// Writing to a buffer in the guest failed.
    #[error("failed to write to buffer: {0}")]
    WriteBuffer(io::Error),
    // Invalid PCM worker state.
    #[error("Invalid PCM worker state")]
    InvalidPCMWorkerState,
    // Invalid backend.
    #[error("Backend is not implemented")]
    InvalidBackend,
    // Failed to generate StreamSource
    #[error("Failed to generate stream source: {0}")]
    GenerateStreamSource(BoxError),
}

pub enum DirectionalStream {
    Input(Box<dyn audio_streams::capture::AsyncCaptureBufferStream>),
    Output(Box<dyn audio_streams::AsyncPlaybackBufferStream>),
}

#[derive(Copy, Clone, std::cmp::PartialEq)]
pub enum WorkerStatus {
    Pause = 0,
    Running = 1,
    Quit = 2,
}

pub struct StreamInfo {
    stream_source: Option<Box<dyn StreamSource>>,
    stream_source_generator: Box<dyn StreamSourceGenerator>,
    channels: u8,
    format: SampleFormat,
    frame_rate: u32,
    buffer_bytes: usize,
    period_bytes: usize,
    direction: u8, // VIRTIO_SND_D_*
    state: u32,    // VIRTIO_SND_R_PCM_SET_PARAMS -> VIRTIO_SND_R_PCM_STOP, or 0 (uninitialized)

    // Worker related
    status_mutex: Rc<AsyncMutex<WorkerStatus>>,
    sender: Option<mpsc::UnboundedSender<DescriptorChain>>,
    worker_future: Option<Box<dyn Future<Output = Result<(), Error>> + Unpin>>,
}

impl fmt::Debug for StreamInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StreamInfo")
            .field("channels", &self.channels)
            .field("format", &self.format)
            .field("frame_rate", &self.frame_rate)
            .field("buffer_bytes", &self.buffer_bytes)
            .field("period_bytes", &self.period_bytes)
            .field("direction", &get_virtio_direction_name(self.direction))
            .field("state", &get_virtio_snd_r_pcm_cmd_name(self.state))
            .finish()
    }
}

// Stores constant data
pub struct SndData {
    jack_info: Vec<virtio_snd_jack_info>,
    pcm_info: Vec<virtio_snd_pcm_info>,
    chmap_info: Vec<virtio_snd_chmap_info>,
}

impl SndData {
    pub fn pcm_info_len(&self) -> usize {
        self.pcm_info.len()
    }
}

const SUPPORTED_FORMATS: u64 = 1 << VIRTIO_SND_PCM_FMT_U8
    | 1 << VIRTIO_SND_PCM_FMT_S16
    | 1 << VIRTIO_SND_PCM_FMT_S24
    | 1 << VIRTIO_SND_PCM_FMT_S32;
const SUPPORTED_FRAME_RATES: u64 = 1 << VIRTIO_SND_PCM_RATE_8000
    | 1 << VIRTIO_SND_PCM_RATE_11025
    | 1 << VIRTIO_SND_PCM_RATE_16000
    | 1 << VIRTIO_SND_PCM_RATE_22050
    | 1 << VIRTIO_SND_PCM_RATE_32000
    | 1 << VIRTIO_SND_PCM_RATE_44100
    | 1 << VIRTIO_SND_PCM_RATE_48000;

// Response from pcm_worker to pcm_queue
pub struct PcmResponse {
    desc_index: u16,
    status: virtio_snd_pcm_status, // response to the pcm message
    writer: Writer,
    done: Option<oneshot::Sender<()>>, // when pcm response is written to the queue
}

impl StreamInfo {
    pub fn new(stream_source_generator: Box<dyn StreamSourceGenerator>) -> Self {
        StreamInfo {
            stream_source: None,
            stream_source_generator,
            channels: 0,
            format: SampleFormat::U8,
            frame_rate: 0,
            buffer_bytes: 0,
            period_bytes: 0,
            direction: 0,
            state: 0,
            status_mutex: Rc::new(AsyncMutex::new(WorkerStatus::Pause)),
            sender: None,
            worker_future: None,
        }
    }

    async fn prepare(
        &mut self,
        ex: &Executor,
        mem: GuestMemory,
        tx_send: &mpsc::UnboundedSender<PcmResponse>,
        rx_send: &mpsc::UnboundedSender<PcmResponse>,
    ) -> Result<(), Error> {
        if self.state != VIRTIO_SND_R_PCM_SET_PARAMS
            && self.state != VIRTIO_SND_R_PCM_PREPARE
            && self.state != VIRTIO_SND_R_PCM_RELEASE
        {
            error!(
                "Invalid PCM state transition from {} to {}",
                get_virtio_snd_r_pcm_cmd_name(self.state),
                get_virtio_snd_r_pcm_cmd_name(VIRTIO_SND_R_PCM_PREPARE)
            );
            return Err(Error::OperationNotSupported);
        }
        if self.state == VIRTIO_SND_R_PCM_PREPARE {
            self.release_worker().await?;
        }
        let frame_size = self.channels as usize * self.format.sample_bytes();
        if self.period_bytes % frame_size != 0 {
            error!("period_bytes must be divisible by frame size");
            return Err(Error::OperationNotSupported);
        }
        if self.stream_source.is_none() {
            self.stream_source = Some(
                self.stream_source_generator
                    .generate()
                    .map_err(Error::GenerateStreamSource)?,
            );
        }
        // (*)
        // `buffer_size` in `audio_streams` API indicates the buffer size in bytes that the stream
        // consumes (or transmits) each time (next_playback/capture_buffer).
        // `period_bytes` in virtio-snd device (or ALSA) indicates the device transmits (or
        // consumes) for each PCM message.
        // Therefore, `buffer_size` in `audio_streams` == `period_bytes` in virtio-snd.
        let (stream, pcm_sender) = match self.direction {
            VIRTIO_SND_D_OUTPUT => (
                DirectionalStream::Output(
                    self.stream_source
                        .as_mut()
                        .unwrap()
                        .new_async_playback_stream(
                            self.channels as usize,
                            self.format,
                            self.frame_rate,
                            // See (*)
                            self.period_bytes / frame_size,
                            ex,
                        )
                        .map_err(Error::CreateStream)?
                        .1,
                ),
                tx_send.clone(),
            ),
            VIRTIO_SND_D_INPUT => {
                (
                    DirectionalStream::Input(
                        self.stream_source
                            .as_mut()
                            .unwrap()
                            .new_async_capture_stream(
                                self.channels as usize,
                                self.format,
                                self.frame_rate,
                                // See (*)
                                self.period_bytes / frame_size,
                                &[],
                                ex,
                            )
                            .map_err(Error::CreateStream)?
                            .1,
                    ),
                    rx_send.clone(),
                )
            }
            _ => unreachable!(),
        };

        let (sender, receiver) = mpsc::unbounded();
        self.sender = Some(sender);
        self.state = VIRTIO_SND_R_PCM_PREPARE;

        self.status_mutex = Rc::new(AsyncMutex::new(WorkerStatus::Pause));
        let f = start_pcm_worker(
            ex.clone(),
            stream,
            receiver,
            self.status_mutex.clone(),
            mem,
            pcm_sender,
            self.period_bytes,
        );
        self.worker_future = Some(Box::new(ex.spawn_local(f).into_future()));
        Ok(())
    }

    async fn start(&mut self) -> Result<(), Error> {
        if self.state != VIRTIO_SND_R_PCM_PREPARE && self.state != VIRTIO_SND_R_PCM_STOP {
            error!(
                "Invalid PCM state transition from {} to {}",
                get_virtio_snd_r_pcm_cmd_name(self.state),
                get_virtio_snd_r_pcm_cmd_name(VIRTIO_SND_R_PCM_START)
            );
            return Err(Error::OperationNotSupported);
        }
        self.state = VIRTIO_SND_R_PCM_START;
        *self.status_mutex.lock().await = WorkerStatus::Running;
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), Error> {
        if self.state != VIRTIO_SND_R_PCM_START {
            error!(
                "Invalid PCM state transition from {} to {}",
                get_virtio_snd_r_pcm_cmd_name(self.state),
                get_virtio_snd_r_pcm_cmd_name(VIRTIO_SND_R_PCM_STOP)
            );
            return Err(Error::OperationNotSupported);
        }
        self.state = VIRTIO_SND_R_PCM_STOP;
        *self.status_mutex.lock().await = WorkerStatus::Pause;
        Ok(())
    }

    async fn release(&mut self) -> Result<(), Error> {
        if self.state != VIRTIO_SND_R_PCM_PREPARE && self.state != VIRTIO_SND_R_PCM_STOP {
            error!(
                "Invalid PCM state transition from {} to {}",
                get_virtio_snd_r_pcm_cmd_name(self.state),
                get_virtio_snd_r_pcm_cmd_name(VIRTIO_SND_R_PCM_RELEASE)
            );
            return Err(Error::OperationNotSupported);
        }
        self.state = VIRTIO_SND_R_PCM_RELEASE;
        self.release_worker().await?;
        self.stream_source = None;
        Ok(())
    }

    async fn release_worker(&mut self) -> Result<(), Error> {
        *self.status_mutex.lock().await = WorkerStatus::Quit;
        if let Some(s) = self.sender.take() {
            s.close_channel();
        }
        if let Some(f) = self.worker_future.take() {
            f.await?;
        }
        Ok(())
    }
}

pub struct VirtioSnd {
    cfg: virtio_snd_config,
    snd_data: Option<SndData>,
    avail_features: u64,
    acked_features: u64,
    queue_sizes: Box<[u16]>,
    worker_threads: Vec<thread::JoinHandle<()>>,
    kill_evt: Option<Event>,
    stream_source_generators: Option<Vec<Box<dyn StreamSourceGenerator>>>,
}

impl VirtioSnd {
    pub fn new(base_features: u64, params: Parameters) -> Result<VirtioSnd, Error> {
        let cfg = hardcoded_virtio_snd_config(&params);
        let snd_data = hardcoded_snd_data(&params);
        let avail_features = base_features;
        let stream_source_generators = create_stream_source_generators(&params, &snd_data);

        Ok(VirtioSnd {
            cfg,
            snd_data: Some(snd_data),
            avail_features,
            acked_features: 0,
            queue_sizes: vec![MAX_VRING_LEN; MAX_QUEUE_NUM].into_boxed_slice(),
            worker_threads: Vec::new(),
            kill_evt: None,
            stream_source_generators: Some(stream_source_generators),
        })
    }
}

pub(crate) fn create_stream_source_generators(
    params: &Parameters,
    snd_data: &SndData,
) -> Vec<Box<dyn StreamSourceGenerator>> {
    match params.backend {
        StreamSourceBackend::NULL => create_null_stream_source_generators(snd_data),
        StreamSourceBackend::Sys(backend) => {
            sys_create_stream_source_generators(backend, params, snd_data)
        }
    }
}

// To be used with hardcoded_snd_data
pub fn hardcoded_virtio_snd_config(params: &Parameters) -> virtio_snd_config {
    virtio_snd_config {
        jacks: 0.into(),
        streams: params.get_total_streams().into(),
        chmaps: (params.num_output_devices * 3 + params.num_input_devices).into(),
    }
}

// To be used with hardcoded_virtio_snd_config
pub fn hardcoded_snd_data(params: &Parameters) -> SndData {
    let jack_info: Vec<virtio_snd_jack_info> = Vec::new();
    let mut pcm_info: Vec<virtio_snd_pcm_info> = Vec::new();
    let mut chmap_info: Vec<virtio_snd_chmap_info> = Vec::new();

    for dev in 0..params.num_output_devices {
        for _ in 0..params.num_output_streams {
            pcm_info.push(virtio_snd_pcm_info {
                hdr: virtio_snd_info {
                    hda_fn_nid: dev.into(),
                },
                features: 0.into(), /* 1 << VIRTIO_SND_PCM_F_XXX */
                formats: SUPPORTED_FORMATS.into(),
                rates: SUPPORTED_FRAME_RATES.into(),
                direction: VIRTIO_SND_D_OUTPUT,
                channels_min: 1,
                channels_max: 6,
                padding: [0; 5],
            });
        }
    }
    for dev in 0..params.num_input_devices {
        for _ in 0..params.num_input_streams {
            pcm_info.push(virtio_snd_pcm_info {
                hdr: virtio_snd_info {
                    hda_fn_nid: dev.into(),
                },
                features: 0.into(), /* 1 << VIRTIO_SND_PCM_F_XXX */
                formats: SUPPORTED_FORMATS.into(),
                rates: SUPPORTED_FRAME_RATES.into(),
                direction: VIRTIO_SND_D_INPUT,
                channels_min: 1,
                channels_max: 2,
                padding: [0; 5],
            });
        }
    }
    // Use stereo channel map.
    let mut positions = [VIRTIO_SND_CHMAP_NONE; VIRTIO_SND_CHMAP_MAX_SIZE];
    positions[0] = VIRTIO_SND_CHMAP_FL;
    positions[1] = VIRTIO_SND_CHMAP_FR;
    for dev in 0..params.num_output_devices {
        chmap_info.push(virtio_snd_chmap_info {
            hdr: virtio_snd_info {
                hda_fn_nid: dev.into(),
            },
            direction: VIRTIO_SND_D_OUTPUT,
            channels: 2,
            positions,
        });
    }
    for dev in 0..params.num_input_devices {
        chmap_info.push(virtio_snd_chmap_info {
            hdr: virtio_snd_info {
                hda_fn_nid: dev.into(),
            },
            direction: VIRTIO_SND_D_INPUT,
            channels: 2,
            positions,
        });
    }
    positions[2] = VIRTIO_SND_CHMAP_RL;
    positions[3] = VIRTIO_SND_CHMAP_RR;
    for dev in 0..params.num_output_devices {
        chmap_info.push(virtio_snd_chmap_info {
            hdr: virtio_snd_info {
                hda_fn_nid: dev.into(),
            },
            direction: VIRTIO_SND_D_OUTPUT,
            channels: 4,
            positions,
        });
    }
    positions[2] = VIRTIO_SND_CHMAP_FC;
    positions[3] = VIRTIO_SND_CHMAP_LFE;
    positions[4] = VIRTIO_SND_CHMAP_RL;
    positions[5] = VIRTIO_SND_CHMAP_RR;
    for dev in 0..params.num_output_devices {
        chmap_info.push(virtio_snd_chmap_info {
            hdr: virtio_snd_info {
                hda_fn_nid: dev.into(),
            },
            direction: VIRTIO_SND_D_OUTPUT,
            channels: 6,
            positions,
        });
    }

    SndData {
        jack_info,
        pcm_info,
        chmap_info,
    }
}

impl VirtioDevice for VirtioSnd {
    fn keep_rds(&self) -> Vec<RawDescriptor> {
        Vec::new()
    }

    fn device_type(&self) -> DeviceType {
        DeviceType::Sound
    }

    fn queue_max_sizes(&self) -> &[u16] {
        &self.queue_sizes
    }

    fn features(&self) -> u64 {
        self.avail_features
    }

    fn ack_features(&mut self, mut v: u64) {
        // Check if the guest is ACK'ing a feature that we didn't claim to have.
        let unrequested_features = v & !self.avail_features;
        if unrequested_features != 0 {
            warn!("virtio_fs got unknown feature ack: {:x}", v);

            // Don't count these features as acked.
            v &= !unrequested_features;
        }
        self.acked_features |= v;
    }

    fn read_config(&self, offset: u64, data: &mut [u8]) {
        copy_config(data, 0, self.cfg.as_slice(), offset)
    }

    fn activate(
        &mut self,
        guest_mem: GuestMemory,
        interrupt: Interrupt,
        queues: Vec<Queue>,
        queue_evts: Vec<Event>,
    ) {
        if queues.len() != self.queue_sizes.len() || queue_evts.len() != self.queue_sizes.len() {
            error!(
                "snd: expected {} queues, got {}",
                self.queue_sizes.len(),
                queues.len()
            );
        }

        let (self_kill_evt, kill_evt) =
            match Event::new().and_then(|evt| Ok((evt.try_clone()?, evt))) {
                Ok(v) => v,
                Err(e) => {
                    error!("failed to create kill Event pair: {}", e);
                    return;
                }
            };
        self.kill_evt = Some(self_kill_evt);

        let snd_data = self.snd_data.take().expect("snd_data is none");
        let stream_source_generators = self
            .stream_source_generators
            .take()
            .expect("stream_source_generators is none");

        let worker_result = thread::Builder::new()
            .name("virtio_snd w".to_string())
            .spawn(move || {
                set_audio_thread_priority();
                if let Err(err_string) = run_worker(
                    interrupt,
                    queues,
                    guest_mem,
                    snd_data,
                    queue_evts,
                    kill_evt,
                    stream_source_generators,
                ) {
                    error!("{}", err_string);
                }
            });

        match worker_result {
            Err(e) => {
                error!("failed to spawn virtio_snd worker: {}", e);
                return;
            }
            Ok(join_handle) => self.worker_threads.push(join_handle),
        }
    }

    fn reset(&mut self) -> bool {
        if let Some(kill_evt) = self.kill_evt.take() {
            // Ignore the result because there is nothing we can do about it.
            let _ = kill_evt.write(1);
        }

        true
    }
}

impl Drop for VirtioSnd {
    fn drop(&mut self) {
        self.reset();
    }
}

fn run_worker(
    interrupt: Interrupt,
    mut queues: Vec<Queue>,
    mem: GuestMemory,
    snd_data: SndData,
    queue_evts: Vec<Event>,
    kill_evt: Event,
    stream_source_generators: Vec<Box<dyn StreamSourceGenerator>>,
) -> Result<(), String> {
    let ex = Executor::new().expect("Failed to create an executor");

    if snd_data.pcm_info_len() != stream_source_generators.len() {
        error!(
            "snd: expected {} streams, got {}",
            snd_data.pcm_info_len(),
            stream_source_generators.len(),
        );
    }
    let streams = stream_source_generators
        .into_iter()
        .map(|generator| AsyncMutex::new(StreamInfo::new(generator)))
        .collect();
    let streams = Rc::new(AsyncMutex::new(streams));

    let ctrl_queue = queues.remove(0);
    let _event_queue = queues.remove(0);
    let tx_queue = Rc::new(AsyncMutex::new(queues.remove(0)));
    let rx_queue = Rc::new(AsyncMutex::new(queues.remove(0)));

    let mut evts_async: Vec<EventAsync> = queue_evts
        .into_iter()
        .map(|e| EventAsync::new(e, &ex).expect("Failed to create async event for queue"))
        .collect();

    let ctrl_queue_evt = evts_async.remove(0);
    let _event_queue_evt = evts_async.remove(0);
    let tx_queue_evt = evts_async.remove(0);
    let rx_queue_evt = evts_async.remove(0);

    let (tx_send, mut tx_recv) = mpsc::unbounded();
    let (rx_send, mut rx_recv) = mpsc::unbounded();
    let tx_send2 = tx_send.clone();
    let rx_send2 = rx_send.clone();

    let f_ctrl = handle_ctrl_queue(
        &ex,
        &mem,
        &streams,
        &snd_data,
        ctrl_queue,
        ctrl_queue_evt,
        interrupt.clone(),
        tx_send,
        rx_send,
    );

    // TODO(woodychow): Enable this when libcras sends jack connect/disconnect evts
    // let f_event = handle_event_queue(
    //     &mem,
    //     snd_state,
    //     event_queue,
    //     event_queue_evt,
    //     interrupt,
    // );

    let f_tx = handle_pcm_queue(&mem, &streams, tx_send2, &tx_queue, tx_queue_evt);

    let f_tx_response = send_pcm_response_worker(&mem, &tx_queue, interrupt.clone(), &mut tx_recv);

    let f_rx = handle_pcm_queue(&mem, &streams, rx_send2, &rx_queue, rx_queue_evt);

    let f_rx_response = send_pcm_response_worker(&mem, &rx_queue, interrupt.clone(), &mut rx_recv);

    let f_resample = async_utils::handle_irq_resample(&ex, interrupt);

    // Exit if the kill event is triggered.
    let f_kill = async_utils::await_and_exit(&ex, kill_evt);

    pin_mut!(
        f_ctrl,
        f_tx,
        f_tx_response,
        f_rx,
        f_rx_response,
        f_resample,
        f_kill
    );

    let done = async {
        select! {
            res = f_ctrl.fuse() => res.context("error in handling ctrl queue"),
            res = f_tx.fuse() => res.context("error in handling tx queue"),
            res = f_tx_response.fuse() => res.context("error in handling tx response"),
            res = f_rx.fuse() => res.context("error in handling rx queue"),
            res = f_rx_response.fuse() => res.context("error in handling rx response"),
            res = f_resample.fuse() => res.context("error in handle_irq_resample"),
            res = f_kill.fuse() => res.context("error in await_and_exit"),
        }
    };
    match ex.run_until(done) {
        Ok(Ok(())) => {}
        Ok(Err(e)) => error!("Error in worker: {}", e),
        Err(e) => error!("Error happened in executor: {}", e),
    }

    Ok(())
}
