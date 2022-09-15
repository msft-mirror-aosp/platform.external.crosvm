// Copyright 2022 The ChromiumOS Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

//! Contains shared code between the broker & its children, specifically any IPC messages or common
//! bootstrapping code.

use std::fs::File;
use std::fs::OpenOptions;

use anyhow::Context;
use base::enable_high_res_timers;
use base::syslog;
use base::EnabledHighResTimer;
use base::FromRawDescriptor;
use base::IntoRawDescriptor;
use base::SafeDescriptor;
use base::Tube;
use serde::Deserialize;
use serde::Serialize;

mod generic;
use std::path::PathBuf;

use generic as product;
use product::init_child_crash_reporting;
use product::product_child_setup;
use product::ProductAttributes;

/// Arguments that are common to all devices & helper processes.
#[derive(Serialize, Deserialize)]
pub struct CommonChildStartupArgs {
    syslog_file: Option<SafeDescriptor>,
    metrics_tube: Option<Tube>,
    product_attrs: ProductAttributes,
}

pub struct ChildLifecycleCleanup {
    _timer_resolution: Box<dyn EnabledHighResTimer>,
}

/// Initializes crash reporting, metrics, logging, and product specific features
/// for a process.
///
/// Returns a value that should be dropped when the process exits.
pub fn common_child_setup(args: CommonChildStartupArgs) -> anyhow::Result<ChildLifecycleCleanup> {
    // Logging must initialize first in case there are other startup errors.
    let mut cfg = syslog::LogConfig::default();
    if let Some(log_file_descriptor) = args.syslog_file {
        // Safe because we are taking ownership of a SafeDescriptor.
        let log_file =
            unsafe { File::from_raw_descriptor(log_file_descriptor.into_raw_descriptor()) };
        cfg.pipe = Some(Box::new(log_file));
        cfg.stderr = false;
    } else {
        cfg.stderr = true;
    }
    syslog::init_with(cfg)?;

    // Crash reporting should start as early as possible, in case other startup tasks fail.
    init_child_crash_reporting(&args.product_attrs);

    // Initialize anything product specific.
    product_child_setup(&args.product_attrs)?;

    if let Some(metrics_tube) = args.metrics_tube {
        metrics::initialize(metrics_tube);
    }

    let timer_resolution = enable_high_res_timers().context("failed to enable high res timer")?;

    Ok(ChildLifecycleCleanup {
        _timer_resolution: timer_resolution,
    })
}

pub(crate) fn log_file_from_path(path: Option<PathBuf>) -> anyhow::Result<Option<SafeDescriptor>> {
    Ok(match path {
        Some(path) => Some(SafeDescriptor::from(
            OpenOptions::new()
                .append(true)
                .create(true)
                .open(path.as_path())
                .context(format!("failed to open log file {}", path.display()))?,
        )),
        None => None,
    })
}
