// Copyright 2022 The ChromiumOS Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use std::env;
use std::path::PathBuf;

fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    build_protos(&PathBuf::from(manifest_dir));
}

fn build_protos(manifest_dir: &PathBuf) {
    let mut event_details_path = manifest_dir.to_owned();
    event_details_path.extend(["protos", "event_details.proto"]);

    let mut out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR env does not exist."));
    // ANDROID: b/259142784 - we remove metrics_out subdir b/c cargo2android
    // out_dir.push("metrics_protos");
    proto_build_tools::build_protos(&out_dir, &[event_details_path]);
}
