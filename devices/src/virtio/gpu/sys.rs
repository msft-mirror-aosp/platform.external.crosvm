// Copyright 2022 The Chromium OS Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

cfg_if::cfg_if! {
    if #[cfg(unix)] {
        mod unix;
        pub use unix::UnixDisplayMode as DisplayMode;
        pub(crate) use unix::UnixDisplayModeArg as DisplayModeArg;
    } else if #[cfg(windows)] {
        mod windows;
        pub use windows::WinDisplayMode<windows::DisplayDataProvider> as DisplayMode;
        pub(crate) use windows::WinDisplayModeArg as DisplayModeArg;
    }
}
