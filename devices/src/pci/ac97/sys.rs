// Copyright 2022 The ChromiumOS Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

cfg_if::cfg_if! {
    if #[cfg(unix)] {
        mod unix;
        use unix as platform;
    } else if #[cfg(windows)] {
        mod windows;
        use windows as platform;
    }
}

pub(in crate::pci::ac97) use platform::ac97_backend_from_str;
pub(in crate::pci::ac97) use platform::create_null_server;
#[cfg(test)]
pub(in crate::pci::ac97) use platform::tests;
pub use platform::Ac97Backend;
pub(crate) use platform::AudioStreamSource;
