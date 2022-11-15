// Copyright 2021 The ChromiumOS Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use std::path::PathBuf;

use anyhow::anyhow;
use anyhow::bail;
use anyhow::Context;
use argh::FromArgs;
use base::error;
use base::Event;
use base::RawDescriptor;
use base::Terminal;
use cros_async::Executor;
use data_model::DataInit;
use hypervisor::ProtectionType;
use vm_memory::GuestMemory;
use vmm_vhost::message::VhostUserProtocolFeatures;
use vmm_vhost::message::VhostUserVirtioFeatures;

use crate::virtio;
use crate::virtio::console::asynchronous::ConsoleDevice;
use crate::virtio::console::virtio_console_config;
use crate::virtio::copy_config;
use crate::virtio::vhost::user::device::handler::sys::Doorbell;
use crate::virtio::vhost::user::device::handler::VhostUserBackend;
use crate::virtio::vhost::user::device::listener::sys::VhostUserListener;
use crate::virtio::vhost::user::device::listener::VhostUserListenerTrait;
use crate::virtio::vhost::user::device::VhostUserDevice;
use crate::SerialHardware;
use crate::SerialParameters;
use crate::SerialType;

const MAX_QUEUE_NUM: usize = 2 /* transmit and receive queues */;
const MAX_VRING_LEN: u16 = 256;

/// Console device for use with vhost-user. Will set stdin back to canon mode if we are getting
/// input from it.
pub struct VhostUserConsoleDevice {
    console: ConsoleDevice,
    /// Whether we should set stdin to raw mode because we are getting user input from there.
    raw_stdin: bool,
}

impl Drop for VhostUserConsoleDevice {
    fn drop(&mut self) {
        if self.raw_stdin {
            // Restore terminal capabilities back to what they were before
            match std::io::stdin().set_canon_mode() {
                Ok(()) => (),
                Err(e) => error!("failed to restore canonical mode for terminal: {:#}", e),
            }
        }
    }
}

impl VhostUserDevice for VhostUserConsoleDevice {
    fn max_queue_num(&self) -> usize {
        MAX_QUEUE_NUM
    }

    fn into_backend(self: Box<Self>, ex: &Executor) -> anyhow::Result<Box<dyn VhostUserBackend>> {
        if self.raw_stdin {
            // Set stdin() to raw mode so we can send over individual keystrokes unbuffered
            std::io::stdin()
                .set_raw_mode()
                .context("failed to set terminal in raw mode")?;
        }

        Ok(Box::new(ConsoleBackend {
            device: *self,
            acked_features: 0,
            acked_protocol_features: VhostUserProtocolFeatures::empty(),
            ex: ex.clone(),
        }))
    }
}

struct ConsoleBackend {
    device: VhostUserConsoleDevice,
    acked_features: u64,
    acked_protocol_features: VhostUserProtocolFeatures,
    ex: Executor,
}

impl VhostUserBackend for ConsoleBackend {
    fn max_queue_num(&self) -> usize {
        self.device.max_queue_num()
    }

    fn max_vring_len(&self) -> u16 {
        MAX_VRING_LEN
    }

    fn features(&self) -> u64 {
        self.device.console.avail_features() | VhostUserVirtioFeatures::PROTOCOL_FEATURES.bits()
    }

    fn ack_features(&mut self, value: u64) -> anyhow::Result<()> {
        let unrequested_features = value & !self.features();
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
        let config = virtio_console_config {
            max_nr_ports: 1.into(),
            ..Default::default()
        };
        copy_config(data, 0, config.as_slice(), offset);
    }

    fn reset(&mut self) {
        for queue_num in 0..self.max_queue_num() {
            self.stop_queue(queue_num);
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
        // Enable any virtqueue features that were negotiated (like VIRTIO_RING_F_EVENT_IDX).
        queue.ack_features(self.acked_features);

        match idx {
            // ReceiveQueue
            0 => self
                .device
                .console
                .start_receive_queue(&self.ex, mem, queue, doorbell, kick_evt),
            // TransmitQueue
            1 => self
                .device
                .console
                .start_transmit_queue(&self.ex, mem, queue, doorbell, kick_evt),
            _ => bail!("attempted to start unknown queue: {}", idx),
        }
    }

    fn stop_queue(&mut self, idx: usize) {
        match idx {
            0 => {
                if let Err(e) = self.device.console.stop_receive_queue() {
                    error!("error while stopping rx queue: {}", e);
                }
            }
            1 => {
                if let Err(e) = self.device.console.stop_transmit_queue() {
                    error!("error while stopping tx queue: {}", e);
                }
            }
            _ => error!("attempted to stop unknown queue: {}", idx),
        };
    }
}

#[derive(FromArgs)]
#[argh(subcommand, name = "console")]
/// Console device
pub struct Options {
    #[argh(option, arg_name = "PATH")]
    /// path to a vhost-user socket
    socket: Option<String>,
    #[argh(option, arg_name = "STRING")]
    /// VFIO-PCI device name (e.g. '0000:00:07.0')
    vfio: Option<String>,
    #[argh(option, arg_name = "OUTFILE")]
    /// path to a file
    output_file: Option<PathBuf>,
    #[argh(option, arg_name = "INFILE")]
    /// path to a file
    input_file: Option<PathBuf>,
    /// whether we are logging to syslog or not
    #[argh(switch)]
    syslog: bool,
}

/// Return a new vhost-user console device. `params` are the device's configuration, and `keep_rds`
/// is a vector into which `RawDescriptors` that need to survive a fork are added, in case the
/// device is meant to run within a child process.
pub fn create_vu_console_device(
    params: &SerialParameters,
    keep_rds: &mut Vec<RawDescriptor>,
) -> anyhow::Result<VhostUserConsoleDevice> {
    let device = params.create_serial_device::<ConsoleDevice>(
        ProtectionType::Unprotected,
        // We need to pass an event as per Serial Device API but we don't really use it anyway.
        &Event::new()?,
        keep_rds,
    )?;

    Ok(VhostUserConsoleDevice {
        console: device,
        raw_stdin: params.stdin,
    })
}

/// Starts a vhost-user console device.
/// Returns an error if the given `args` is invalid or the device fails to run.
pub fn run_console_device(opts: Options) -> anyhow::Result<()> {
    let type_ = match opts.output_file {
        Some(_) => {
            if opts.syslog {
                bail!("--output-file and --syslog options cannot be used together.");
            }
            SerialType::File
        }
        None => {
            if opts.syslog {
                SerialType::Syslog
            } else {
                SerialType::Stdout
            }
        }
    };

    let params = SerialParameters {
        type_,
        hardware: SerialHardware::VirtioConsole,
        // Required only if type_ is SerialType::File or SerialType::UnixSocket
        path: opts.output_file,
        input: opts.input_file,
        num: 1,
        console: true,
        earlycon: false,
        // We don't use stdin if syslog mode is enabled
        stdin: !opts.syslog,
        out_timestamp: false,
        ..Default::default()
    };

    // We won't jail the device and can simply ignore `keep_rds`.
    let device = Box::new(create_vu_console_device(&params, &mut Vec::new())?);
    let ex = Executor::new().context("Failed to create executor")?;
    let backend = device.into_backend(&ex)?;

    let listener = VhostUserListener::new_from_socket_or_vfio(
        &opts.socket,
        &opts.vfio,
        backend.max_queue_num(),
        None,
    )?;

    // run_until() returns an Result<Result<..>> which the ? operator lets us flatten.
    ex.run_until(listener.run_backend(backend, &ex))?
}
