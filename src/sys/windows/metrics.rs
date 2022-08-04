// Copyright 2022 The ChromiumOS Authors.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

cfg_if::cfg_if! {
    if #[cfg(feature = "kiwi")] {
        extern crate metrics as metrics_crate;
        use anyhow::{Context};
        use broker_ipc::{common_child_setup, CommonChildStartupArgs};
        use base::Tube;
        use std::thread;
        use metrics_crate::MetricsController;
        use crate::crosvm::sys::windows::exit::{Exit, ExitContext, ExitContextAnyhow};
        use crate::sys::windows::main::set_bootstrap_arguments;
        use tube_transporter::{TubeToken, TubeTransporterReader};
        use base::FromRawDescriptor;
    }
}

use anyhow::Result;
pub(crate) use metrics::get_destructor;
pub(crate) use metrics::log_descriptor;
#[cfg(feature = "kiwi")]
pub(crate) use metrics::merge_session_invariants;
#[cfg(feature = "kiwi")]
pub(crate) use metrics::set_auth_token;
#[cfg(feature = "kiwi")]
pub(crate) use metrics::set_package_name;
pub(crate) use metrics::MetricEventType;

#[cfg(feature = "kiwi")]
use crate::crosvm::argument::Argument;

pub(crate) fn run_metrics(#[allow(unused_variables)] args: Vec<String>) -> Result<()> {
    #[cfg(not(feature = "kiwi"))]
    return Ok(());

    #[cfg(feature = "kiwi")]
    {
        let arguments = &[Argument::value(
            "bootstrap",
            "TRANSPORT_TUBE_RD",
            "TubeTransporter descriptor used to bootstrap the metrics process.",
        )];

        let raw_transport_tube = set_bootstrap_arguments(args, arguments).exit_context(
            Exit::InvalidSubCommandArgs,
            "error in setting crosvm metrics controller args",
        )?;

        // Safe because we know that raw_transport_tube is valid (passed by inheritance), and that the
        // blocking & framing modes are accurate because we create them ourselves in the broker.
        let tube_transporter =
            unsafe { TubeTransporterReader::from_raw_descriptor(raw_transport_tube.unwrap()) };

        let mut tube_data_list = tube_transporter
            .read_tubes()
            .exit_context(Exit::TubeTransporterInit, "failed to initialize tube")?;

        let bootstrap_tube = tube_data_list.get_tube(TubeToken::Bootstrap).unwrap();

        let startup_args: CommonChildStartupArgs =
            bootstrap_tube.recv::<CommonChildStartupArgs>().unwrap();
        let _child_cleanup = common_child_setup(startup_args).exit_context(
            Exit::CommonChildSetupError,
            "failed to perform common child setup",
        )?;

        let metrics_tubes = bootstrap_tube.recv::<Vec<Tube>>().unwrap();

        tracing::init();
        crate::sys::sandbox_lower_token()?;

        let mut metrics_controller = MetricsController::new(metrics_tubes);
        metrics_controller
            .run()
            .exit_context(Exit::MetricsController, "metrics controller failed")
    }
}

pub(crate) fn setup_metrics_reporting() -> Result<()> {
    #[cfg(not(feature = "kiwi"))]
    return Ok(());

    #[cfg(feature = "kiwi")]
    {
        let (metrics_controller_tube, metrics_agent_tube) =
            Tube::pair().exit_context(Exit::CreateTube, "failed to create tube")?;
        thread::spawn(move || {
            let mut metrics_controller = MetricsController::new(vec![metrics_controller_tube]);
            metrics_controller
                .run()
                .context("metrics controller failed")
                .unwrap();
        });
        metrics::initialize(metrics_agent_tube);
        Ok(())
    }
}
