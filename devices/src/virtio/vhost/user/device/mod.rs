// Copyright 2021 The ChromiumOS Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

mod block;
#[cfg(feature = "gpu")]
pub mod gpu;
mod handler;
mod listener;
#[cfg(feature = "audio")]
pub mod snd;

pub use block::run_block_device;
pub use block::Options as BlockOptions;
use cros_async::Executor;
use cros_async::ExecutorKind;
#[cfg(feature = "gpu")]
pub use gpu::run_gpu_device;
#[cfg(feature = "gpu")]
pub use gpu::Options as GpuOptions;
pub use handler::VhostBackendReqConnectionState;
pub use handler::VhostUserBackend;
pub use listener::sys::VhostUserListener;
pub use listener::VhostUserListenerTrait;
#[cfg(feature = "audio")]
pub use snd::run_snd_device;
#[cfg(feature = "audio")]
pub use snd::Options as SndOptions;

cfg_if::cfg_if! {
    if #[cfg(unix)] {
        mod console;
        mod fs;
        mod net;
        mod vsock;
        mod vvu;
        mod wl;

        pub use vsock::{run_vsock_device, Options as VsockOptions};
        pub use wl::{run_wl_device, parse_wayland_sock, Options as WlOptions};
        pub use console::{create_vu_console_device, run_console_device, Options as ConsoleOptions};
        pub use fs::{run_fs_device, Options as FsOptions};
        pub use net::{run_net_device, Options as NetOptions};
    } else if #[cfg(windows)] {
        #[cfg(feature = "slirp")]
        mod net;
        #[cfg(feature = "slirp")]
        pub use net::{run_net_device, Options as NetOptions};
        #[cfg(feature = "slirp")]
        pub use net::sys::windows::NetBackendConfig;

    }
}

/// A trait for vhost-user devices.
///
/// Upon being given an [[Executor]], a device can be converted into a [[VhostUserBackend]], which
/// can then process the requests from the front-end.
///
/// We don't build `VhostUserBackend`s directly to ensure that a `VhostUserBackend` starts to
/// process queues in the jailed process, not in the main process. `VhostUserDevice` calls
/// [[VhostUserDevice::into_backend()]] only after jailing, which ensures that any operations by
/// `VhostUserBackend` is done in the jailed process.
pub trait VhostUserDevice {
    /// The maximum number of queues that this device can manage.
    fn max_queue_num(&self) -> usize;

    /// Turn this device into a `VhostUserBackend`, ready to process requests.
    ///
    /// If the device needs to perform something after being jailed, this is also the right place
    /// to do it.
    fn into_backend(self: Box<Self>, ex: &Executor) -> anyhow::Result<Box<dyn VhostUserBackend>>;

    /// The preferred ExecutorKind of an Executor to accept by [`VhostUserDevice::into_backend()`].
    fn executor_kind(&self) -> Option<ExecutorKind> {
        None
    }
}
