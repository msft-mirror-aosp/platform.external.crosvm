// Copyright 2022 The ChromiumOS Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#[cfg(unix)]
pub(crate) mod unix;

#[cfg(windows)]
pub(crate) mod windows;

cfg_if::cfg_if! {
    if #[cfg(unix)] {
        use unix as platform;

        #[cfg(feature = "gpu")]
        pub(crate) use unix::gpu::GpuRenderServerParameters;
    } else if #[cfg(windows)] {
        use windows as platform;
    } else {
        compile_error!("Unsupported platform");
    }
}

pub(crate) use platform::cmdline;
pub(crate) use platform::config;
pub(crate) use platform::config::HypervisorKind;
#[cfg(feature = "crash-report")]
pub(crate) use platform::setup_emulator_crash_reporting;
