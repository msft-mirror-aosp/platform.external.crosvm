// Copyright 2022 The ChromiumOS Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

pub mod fixture;
use fixture::vm::Config;
use fixture::vm::TestVm;

// Tests for possible backwards compatibility issues.
//
// There is no backwards compatibility policy yet, these are just "change detector" tests. If you
// break a test, make sure the change is intended and then ask in go/crosvm-chat to see if anyone
// objects to updating the golden file.

// Many changes to PCI devices can cause issues, e.g. some users depend on crosvm always choosing
// the same PCI slots for particular devices.
#[test]
fn backcompat_test_simple_lspci() {
    let mut vm = TestVm::new(Config::new()).unwrap();
    let expected = include_str!("goldens/backcompat_test_simple_lspci.txt").trim();
    let result = vm
        .exec_in_guest("lspci -n")
        .unwrap()
        .trim()
        .replace("\r", "");
    assert_eq!(
        expected,
        result,
        "PCI Devices changed:\n<<< Expected <<<\n{}\n<<<<<<<<<<<<<<<<\n>>> Got      >>>\n{}\n>>>>>>>>>>>>>>>>\n",
        expected, result
    );
}
