// Copyright 2022 The ChromiumOS Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use std::env;
use std::fs;
use std::path::PathBuf;

use anyhow::Context;
use anyhow::Result;
use cbindgen::Config;
use cbindgen::Language;
use tempfile::TempDir;

static COPYRIGHT_CLAUSE: &str = "// Copyright 2022 The ChromiumOS Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.";

static AUTOGENERATED_DISCLAIMER: &str =
    "/* Warning, this file is autogenerated by cbindgen. Don't modify this manually. */";

static INCLUDE_GUARD: &str = "CROSVM_CONTROL_H_";

static CROSVM_CONTROL_HEADER_NAME: &str = "crosvm_control.h";

fn main() -> Result<()> {
    // Skip building dependencies when generating documents.
    if std::env::var("CARGO_DOC").is_ok() {
        return Ok(());
    }

    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

    let output_dir = PathBuf::from(env::var("OUT_DIR").context("failed to get OUT_DIR")?);

    let output_file = output_dir
        .join(CROSVM_CONTROL_HEADER_NAME)
        .display()
        .to_string();

    let config = Config {
        language: Language::C,
        cpp_compat: true,
        header: Some(String::from(COPYRIGHT_CLAUSE)),
        include_guard: Some(String::from(INCLUDE_GUARD)),
        autogen_warning: Some(String::from(AUTOGENERATED_DISCLAIMER)),
        include_version: true,
        ..Default::default()
    };

    cbindgen::Builder::new()
        .with_crate(crate_dir)
        .with_config(config)
        .generate()
        .context("Unable to generate bindings")?
        .write_to_file(&output_file);

    // Do not perform the compilation check on Windows since GCC might not be installed.
    if std::env::var("CARGO_CFG_WINDOWS").is_ok() {
        return Ok(());
    }

    // Do a quick compile test of the generated header to ensure it is valid
    let temp_dir = TempDir::new()?;
    let test_file = temp_dir
        .path()
        .join("crosvm_control_test.c")
        .display()
        .to_string();

    fs::write(
        &test_file,
        format!("{}{}{}", "#include \"", CROSVM_CONTROL_HEADER_NAME, "\""),
    )
    .context("Failed to write crosvm_control test C file")?;

    cc::Build::new()
        .include(output_dir)
        .file(test_file)
        .compile("crosvm_control_test");

    Ok(())
}
