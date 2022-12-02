// Copyright 2020 The ChromiumOS Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

//! Safe wrapper over the Linux `io_uring` system calls.

mod bindings;
mod syscalls;
mod uring;

pub use uring::*;
