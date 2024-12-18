// Copyright 2024 The ChromiumOS Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

//! This crate provides a logic for creating an ext2 filesystem on memory.

#![cfg(any(target_os = "android", target_os = "linux"))]
#![deny(missing_docs)]

mod arena;
mod bitmap;
mod blockgroup;
mod builder;
mod fs;
mod inode;
mod superblock;
mod xattr;

pub use blockgroup::BLOCK_SIZE;
pub use builder::Builder;
pub use xattr::dump_xattrs;
pub use xattr::set_xattr;
