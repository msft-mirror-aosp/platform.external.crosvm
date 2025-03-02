// Copyright 2024 The ChromiumOS Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

//! Data structures and logic for virtio-fs IOCTLs specific to ARCVM.

use zerocopy::FromBytes;
use zerocopy::Immutable;
use zerocopy::IntoBytes;
use zerocopy::KnownLayout;

pub const FS_IOCTL_PATH_MAX_LEN: usize = 128;
pub const FS_IOCTL_XATTR_NAME_MAX_LEN: usize = 128;
pub const FS_IOCTL_XATTR_VALUE_MAX_LEN: usize = 128;

#[repr(C)]
#[derive(Clone, Copy, FromBytes, Immutable, IntoBytes, KnownLayout)]
pub(crate) struct FsPermissionDataBuffer {
    pub guest_uid: u32,
    pub guest_gid: u32,
    pub host_uid: u32,
    pub host_gid: u32,
    pub umask: u32,
    pub pad: u32,
    pub perm_path: [u8; FS_IOCTL_PATH_MAX_LEN],
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct XattrData {
    pub xattr_name: String,
    pub xattr_value: String,
    pub xattr_path: String,
}

impl XattrData {
    pub(crate) fn need_set_guest_xattr(&self, path: &str, name: &str) -> bool {
        path.starts_with(&self.xattr_path) && (name == self.xattr_name)
    }
}
#[repr(C)]
#[derive(Clone, Copy, FromBytes, Immutable, IntoBytes, KnownLayout)]
pub(crate) struct FsPathXattrDataBuffer {
    pub path: [u8; FS_IOCTL_PATH_MAX_LEN],
    pub xattr_name: [u8; FS_IOCTL_XATTR_NAME_MAX_LEN],
    pub xattr_value: [u8; FS_IOCTL_XATTR_VALUE_MAX_LEN],
}
