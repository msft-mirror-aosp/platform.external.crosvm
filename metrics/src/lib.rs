// Copyright 2022 The ChromiumOS Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

//! This crate serves to provide metrics bindings to be used throughout the codebase.
//! For binaries that wish to use metrics, the intention is that an independent metrics
//! process will run (main loop in the controller mod), and receive requests via a tube from
//! another process.
//!
//! At head, metrics requests are ignored. However, a branching codebase can choose to implement
//! their own handler which processes and uploads metrics requests as it sees fit, by setting the
//! appropriate RequestHandler.

mod controller;
mod event_types;
mod metrics_cleanup;
mod metrics_requests;
mod noop;
mod sys;
// Exports a <name>_proto module for each proto file
include!(concat!(env!("OUT_DIR"), "/generated.rs"));

pub use controller::MetricsController;
pub use event_types::MetricEventType;
pub use metrics_cleanup::MetricsClientDestructor;
pub use noop::*;
#[allow(unused_imports)]
pub use sys::*;

pub type RequestHandler = NoopMetricsRequestHandler;
