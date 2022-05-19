// Copyright 2022 The Chromium OS Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

cfg_if::cfg_if! {
    if #[cfg(unix)] {
        pub(crate) mod unix;
        use unix as platform;
    } else {
        compile_error!("Unsupported platform");
    }
}

pub(crate) use platform::main::{get_arguments, set_arguments, start_device};

#[cfg(feature = "audio")]
pub(crate) use platform::main::{check_ac97_backend, parse_ac97_options};
