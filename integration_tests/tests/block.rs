// Copyright 2022 The Chromium OS Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

//! Testing virtio-block.

pub mod fixture;

use std::process::Command;

use fixture::Config;
use fixture::TestVm;
use tempfile::NamedTempFile;

fn prepare_disk_img() -> NamedTempFile {
    let mut disk = NamedTempFile::new().unwrap();
    disk.as_file_mut().set_len(1024 * 1024).unwrap();

    // TODO(b/243127910): Use `mkfs.ext4 -d` to include test data.
    Command::new("sudo")
        .arg("mkfs.ext4")
        .arg(disk.path().to_str().unwrap())
        .output()
        .expect("failed to execute process");
    disk
}

// TODO(b/243127498): Add tests for write and sync operations.
#[test]
fn mount_block() {
    let disk = prepare_disk_img();
    let disk_path = disk.path().to_str().unwrap().to_string();
    println!("disk={disk_path}");

    let config = Config::new().extra_args(vec!["--rwdisk".to_string(), disk_path]);
    let mut vm = TestVm::new(config).unwrap();
    assert_eq!(
        vm.exec_in_guest("mount -t ext4 /dev/vdb /mnt && echo 42")
            .unwrap()
            .trim(),
        "42"
    );
}
