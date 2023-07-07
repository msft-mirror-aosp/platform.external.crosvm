// Copyright 2022 The ChromiumOS Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

cfg_if::cfg_if! {
    if #[cfg(unix)] {
        pub(in crate::serial) mod unix;
        use unix as platform;
    } else if #[cfg(windows)] {
        pub(in crate::serial) mod windows;
        use windows as platform;
    }
}

pub(crate) use platform::InStreamType;
