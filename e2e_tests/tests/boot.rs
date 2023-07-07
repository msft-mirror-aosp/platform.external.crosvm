// Copyright 2020 The ChromiumOS Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use fixture::vm::Config;
use fixture::vm::TestVm;

#[test]
fn boot_test_vm() -> anyhow::Result<()> {
    let mut vm = TestVm::new(Config::new()).unwrap();
    assert_eq!(vm.exec_in_guest("echo 42")?.trim(), "42");
    Ok(())
}

#[test]
fn boot_test_vm_uring() -> anyhow::Result<()> {
    let mut vm = TestVm::new(
        Config::new().extra_args(vec!["--async-executor".to_string(), "uring".to_string()]),
    )
    .unwrap();
    assert_eq!(vm.exec_in_guest("echo 42")?.trim(), "42");
    Ok(())
}

#[cfg(unix)]
#[test]
fn boot_test_vm_odirect() {
    let mut vm = TestVm::new(Config::new().o_direct()).unwrap();
    assert_eq!(vm.exec_in_guest("echo 42").unwrap().trim(), "42");
}

#[cfg(unix)]
#[test]
fn boot_test_vm_config_file() {
    let mut vm = TestVm::new_with_config_file(Config::new()).unwrap();
    assert_eq!(vm.exec_in_guest("echo 42").unwrap().trim(), "42");
}

#[cfg(unix)]
#[test]
fn boot_test_suspend_resume() {
    // There is no easy way for us to check if the VM is actually suspended. But at
    // least exercise the code-path.
    let mut vm = TestVm::new(Config::new()).unwrap();
    vm.suspend().unwrap();
    vm.resume().unwrap();
    assert_eq!(vm.exec_in_guest("echo 42").unwrap().trim(), "42");
}

#[cfg(unix)]
#[test]
fn boot_test_suspend_resume_full() {
    // There is no easy way for us to check if the VM is actually suspended. But at
    // least exercise the code-path.
    let mut config = Config::new();
    config = config.with_stdout_hardware("legacy-virtio-console");
    config = config.extra_args(vec![
        "--no-usb".to_string(),
        "--no-balloon".to_string(),
        "--no-rng".to_string(),
    ]);

    let mut vm = TestVm::new(config).unwrap();
    vm.suspend_full().unwrap();
    vm.resume_full().unwrap();
    assert_eq!(vm.exec_in_guest("echo 42").unwrap().trim(), "42");
}

#[cfg(unix)]
#[test]
fn boot_test_vm_disable_sandbox() {
    let mut vm = TestVm::new(Config::new().disable_sandbox()).unwrap();
    assert_eq!(vm.exec_in_guest("echo 42").unwrap().trim(), "42");
}

#[cfg(unix)]
#[test]
fn boot_test_vm_disable_sandbox_odirect() {
    let mut vm = TestVm::new(Config::new().disable_sandbox().o_direct()).unwrap();
    assert_eq!(vm.exec_in_guest("echo 42").unwrap().trim(), "42");
}

#[cfg(unix)]
#[test]
fn boot_test_vm_disable_sandbox_config_file() {
    let mut vm = TestVm::new_with_config_file(Config::new().disable_sandbox()).unwrap();
    assert_eq!(vm.exec_in_guest("echo 42").unwrap().trim(), "42");
}

#[cfg(unix)]
#[test]
fn boot_test_disable_sandbox_suspend_resume() {
    // There is no easy way for us to check if the VM is actually suspended. But at
    // least exercise the code-path.
    let mut vm = TestVm::new(Config::new().disable_sandbox()).unwrap();
    vm.suspend().unwrap();
    vm.resume().unwrap();
    assert_eq!(vm.exec_in_guest("echo 42").unwrap().trim(), "42");
}
