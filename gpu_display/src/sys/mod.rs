// Copyright 2022 The ChromiumOS Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

cfg_if::cfg_if! {
    if #[cfg(unix)] {
        pub(crate) mod unix;
        pub use unix::UnixGpuDisplayExt as SysGpuDisplayExt;
        pub(crate) use unix::UnixDisplayT as SysDisplayT;
    } else if #[cfg(windows)] {
        pub(crate) mod windows;
        pub use windows::WinGpuDisplayExt as SysGpuDisplayExt;
        pub(crate) use windows::WinDisplayT as SysDisplayT;
    }
}
