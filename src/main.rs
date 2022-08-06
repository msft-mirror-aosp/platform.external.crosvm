// Copyright 2017 The Chromium OS Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

//! Runs a virtual machine

use std::fs::OpenOptions;
use std::path::Path;

use anyhow::anyhow;
use anyhow::Result;
use argh::FromArgs;
use base::error;
use base::info;
use base::syslog;
use base::syslog::LogConfig;
use cmdline::RunCommand;
use cmdline::UsbAttachCommand;
mod crosvm;
use crosvm::cmdline;
#[cfg(feature = "plugin")]
use crosvm::config::executable_is_plugin;
use crosvm::config::Config;
use devices::virtio::vhost::user::device::run_block_device;
use devices::virtio::vhost::user::device::run_net_device;
#[cfg(feature = "composite-disk")]
use disk::create_composite_disk;
#[cfg(feature = "composite-disk")]
use disk::create_disk_file;
#[cfg(feature = "composite-disk")]
use disk::create_zero_filler;
#[cfg(feature = "composite-disk")]
use disk::ImagePartitionType;
#[cfg(feature = "composite-disk")]
use disk::PartitionInfo;
use disk::QcowFile;
mod sys;
use crosvm::cmdline::Command;
use crosvm::cmdline::CrossPlatformCommands;
use crosvm::cmdline::CrossPlatformDevicesCommands;
#[cfg(windows)]
use sys::windows::metrics;
use vm_control::client::do_modify_battery;
use vm_control::client::do_usb_attach;
use vm_control::client::do_usb_detach;
use vm_control::client::do_usb_list;
use vm_control::client::handle_request;
use vm_control::client::vms_request;
use vm_control::client::ModifyUsbResult;
#[cfg(feature = "balloon")]
use vm_control::BalloonControlCommand;
use vm_control::DiskControlCommand;
use vm_control::UsbControlResult;
use vm_control::VmRequest;
#[cfg(feature = "balloon")]
use vm_control::VmResponse;

use crate::sys::error_to_exit_code;
use crate::sys::init_log;

#[cfg(feature = "scudo")]
#[global_allocator]
static ALLOCATOR: scudo::GlobalScudoAllocator = scudo::GlobalScudoAllocator;

enum CommandStatus {
    Success,
    VmReset,
    VmStop,
    VmCrash,
    GuestPanic,
}

fn to_command_status(result: Result<sys::ExitState>) -> Result<CommandStatus> {
    match result {
        Ok(sys::ExitState::Stop) => {
            info!("crosvm has exited normally");
            Ok(CommandStatus::VmStop)
        }
        Ok(sys::ExitState::Reset) => {
            info!("crosvm has exited normally due to reset request");
            Ok(CommandStatus::VmReset)
        }
        Ok(sys::ExitState::Crash) => {
            info!("crosvm has exited due to a VM crash");
            Ok(CommandStatus::VmCrash)
        }
        Ok(sys::ExitState::GuestPanic) => {
            info!("crosvm has exited due to a kernel panic in guest");
            Ok(CommandStatus::GuestPanic)
        }
        Err(e) => {
            error!("crosvm has exited with error: {:#}", e);
            Err(e)
        }
    }
}

fn run_vm<F: 'static>(cmd: RunCommand, log_config: LogConfig<F>) -> Result<CommandStatus>
where
    F: Fn(&mut syslog::fmt::Formatter, &log::Record<'_>) -> std::io::Result<()> + Sync + Send,
{
    let cfg = match TryInto::<Config>::try_into(cmd) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("{}", e);
            return Err(anyhow!("{}", e));
        }
    };

    #[cfg(feature = "plugin")]
    if executable_is_plugin(&cfg.executable_path) {
        let res = match crosvm::plugin::run_config(cfg) {
            Ok(_) => {
                info!("crosvm and plugin have exited normally");
                Ok(CommandStatus::VmStop)
            }
            Err(e) => {
                eprintln!("{:#}", e);
                Err(e)
            }
        };
        return res;
    }

    #[cfg(feature = "crash-report")]
    crosvm::sys::setup_emulator_crash_reporting(&cfg)?;

    #[cfg(windows)]
    metrics::setup_metrics_reporting()?;

    init_log(log_config, &cfg)?;
    let exit_state = crate::sys::run_config(cfg);
    to_command_status(exit_state)
}

fn stop_vms(cmd: cmdline::StopCommand) -> std::result::Result<(), ()> {
    vms_request(&VmRequest::Exit, cmd.socket_path)
}

fn suspend_vms(cmd: cmdline::SuspendCommand) -> std::result::Result<(), ()> {
    vms_request(&VmRequest::Suspend, cmd.socket_path)
}

fn resume_vms(cmd: cmdline::ResumeCommand) -> std::result::Result<(), ()> {
    vms_request(&VmRequest::Resume, cmd.socket_path)
}

fn powerbtn_vms(cmd: cmdline::PowerbtnCommand) -> std::result::Result<(), ()> {
    vms_request(&VmRequest::Powerbtn, cmd.socket_path)
}

fn sleepbtn_vms(cmd: cmdline::SleepCommand) -> std::result::Result<(), ()> {
    vms_request(&VmRequest::Sleepbtn, cmd.socket_path)
}

fn inject_gpe(cmd: cmdline::GpeCommand) -> std::result::Result<(), ()> {
    vms_request(&VmRequest::Gpe(cmd.gpe), cmd.socket_path)
}

#[cfg(feature = "balloon")]
fn balloon_vms(cmd: cmdline::BalloonCommand) -> std::result::Result<(), ()> {
    let command = BalloonControlCommand::Adjust {
        num_bytes: cmd.num_bytes,
    };
    vms_request(&VmRequest::BalloonCommand(command), cmd.socket_path)
}

#[cfg(feature = "balloon")]
fn balloon_stats(cmd: cmdline::BalloonStatsCommand) -> std::result::Result<(), ()> {
    let command = BalloonControlCommand::Stats {};
    let request = &VmRequest::BalloonCommand(command);
    let response = handle_request(request, cmd.socket_path)?;
    match serde_json::to_string_pretty(&response) {
        Ok(response_json) => println!("{}", response_json),
        Err(e) => {
            error!("Failed to serialize into JSON: {}", e);
            return Err(());
        }
    }
    match response {
        VmResponse::BalloonStats { .. } => Ok(()),
        _ => Err(()),
    }
}

fn modify_battery(cmd: cmdline::BatteryCommand) -> std::result::Result<(), ()> {
    do_modify_battery(
        cmd.socket_path,
        &cmd.battery_type,
        &cmd.property,
        &cmd.target,
    )
}

fn modify_vfio(cmd: cmdline::VfioCrosvmCommand) -> std::result::Result<(), ()> {
    let (request, socket_path, vfio_path) = match cmd.command {
        cmdline::VfioSubCommand::Add(c) => {
            let request = VmRequest::VfioCommand {
                vfio_path: c.vfio_path.clone(),
                add: true,
                hp_interrupt: true,
            };
            (request, c.socket_path, c.vfio_path)
        }
        cmdline::VfioSubCommand::Remove(c) => {
            let request = VmRequest::VfioCommand {
                vfio_path: c.vfio_path.clone(),
                add: true,
                hp_interrupt: true,
            };
            (request, c.socket_path, c.vfio_path)
        }
    };
    if !vfio_path.exists() || !vfio_path.is_dir() {
        error!("Invalid host sysfs path: {:?}", vfio_path);
        return Err(());
    }
    handle_request(&request, socket_path)?;
    Ok(())
}

#[cfg(feature = "composite-disk")]
fn create_composite(cmd: cmdline::CreateCompositeCommand) -> std::result::Result<(), ()> {
    use std::fs::File;
    use std::path::PathBuf;

    let composite_image_path = &cmd.path;
    let zero_filler_path = format!("{}.filler", composite_image_path);
    let header_path = format!("{}.header", composite_image_path);
    let footer_path = format!("{}.footer", composite_image_path);

    let mut composite_image_file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(true)
        .open(&composite_image_path)
        .map_err(|e| {
            error!(
                "Failed opening composite disk image file at '{}': {}",
                composite_image_path, e
            );
        })?;
    create_zero_filler(&zero_filler_path).map_err(|e| {
        error!(
            "Failed to create zero filler file at '{}': {}",
            &zero_filler_path, e
        );
    })?;
    let mut header_file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(true)
        .open(&header_path)
        .map_err(|e| {
            error!(
                "Failed opening header image file at '{}': {}",
                header_path, e
            );
        })?;
    let mut footer_file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(true)
        .open(&footer_path)
        .map_err(|e| {
            error!(
                "Failed opening footer image file at '{}': {}",
                footer_path, e
            );
        })?;

    let partitions = cmd
        .partitions
        .into_iter()
        .map(|partition_arg| {
            if let [label, path] = partition_arg.split(":").collect::<Vec<_>>()[..] {
                let partition_file = File::open(path)
                    .map_err(|e| error!("Failed to open partition image: {}", e))?;

                // Sparseness for composite disks is not user provided on Linux
                // (e.g. via an option), and it has no runtime effect.
                let size = create_disk_file(
                    partition_file,
                    /* is_sparse_file= */ true,
                    disk::MAX_NESTING_DEPTH,
                    Path::new(path),
                )
                .map_err(|e| error!("Failed to create DiskFile instance: {}", e))?
                .get_len()
                .map_err(|e| error!("Failed to get length of partition image: {}", e))?;
                Ok(PartitionInfo {
                    label: label.to_owned(),
                    path: Path::new(path).to_owned(),
                    partition_type: ImagePartitionType::LinuxFilesystem,
                    writable: false,
                    size,
                })
            } else {
                error!(
                    "Must specify label and path for partition '{}', like LABEL:PATH",
                    partition_arg
                );
                Err(())
            }
        })
        .collect::<Result<Vec<_>, _>>()?;

    create_composite_disk(
        &partitions,
        &PathBuf::from(zero_filler_path),
        &PathBuf::from(header_path),
        &mut header_file,
        &PathBuf::from(footer_path),
        &mut footer_file,
        &mut composite_image_file,
    )
    .map_err(|e| {
        error!(
            "Failed to create composite disk image at '{}': {}",
            composite_image_path, e
        );
    })?;

    Ok(())
}

fn create_qcow2(cmd: cmdline::CreateQcow2Command) -> std::result::Result<(), ()> {
    if !(cmd.size.is_some() ^ cmd.backing_file.is_some()) {
        println!(
            "Create a new QCOW2 image at `PATH` of either the specified `SIZE` in bytes or
    with a '--backing_file'."
        );
        return Err(());
    }

    let file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(true)
        .open(&cmd.file_path)
        .map_err(|e| {
            error!("Failed opening qcow file at '{}': {}", cmd.file_path, e);
        })?;

    match (cmd.size, cmd.backing_file) {
        (Some(size), None) => QcowFile::new(file, size).map_err(|e| {
            error!("Failed to create qcow file at '{}': {}", cmd.file_path, e);
        })?,
        (None, Some(backing_file)) => {
            QcowFile::new_from_backing(file, &backing_file, disk::MAX_NESTING_DEPTH).map_err(
                |e| {
                    error!("Failed to create qcow file at '{}': {}", cmd.file_path, e);
                },
            )?
        }
        _ => unreachable!(),
    };
    Ok(())
}

fn start_device(opts: cmdline::DeviceCommand) -> std::result::Result<(), ()> {
    let result = match opts.command {
        cmdline::DeviceSubcommand::CrossPlatform(command) => match command {
            CrossPlatformDevicesCommands::Block(cfg) => run_block_device(cfg),
            CrossPlatformDevicesCommands::Net(cfg) => run_net_device(cfg),
        },
        cmdline::DeviceSubcommand::Sys(command) => sys::start_device(command),
    };

    result.map_err(|e| {
        error!("Failed to run device: {:#}", e);
    })
}

fn disk_cmd(cmd: cmdline::DiskCommand) -> std::result::Result<(), ()> {
    match cmd.command {
        cmdline::DiskSubcommand::Resize(cmd) => {
            let request = VmRequest::DiskCommand {
                disk_index: cmd.disk_index,
                command: DiskControlCommand::Resize {
                    new_size: cmd.disk_size,
                },
            };
            vms_request(&request, cmd.socket_path)
        }
    }
}

fn make_rt(cmd: cmdline::MakeRTCommand) -> std::result::Result<(), ()> {
    vms_request(&VmRequest::MakeRT, cmd.socket_path)
}

fn usb_attach(cmd: UsbAttachCommand) -> ModifyUsbResult<UsbControlResult> {
    let dev_path = Path::new(&cmd.dev_path);

    do_usb_attach(cmd.socket_path, dev_path)
}

fn usb_detach(cmd: cmdline::UsbDetachCommand) -> ModifyUsbResult<UsbControlResult> {
    do_usb_detach(cmd.socket_path, cmd.port)
}

fn usb_list(cmd: cmdline::UsbListCommand) -> ModifyUsbResult<UsbControlResult> {
    do_usb_list(cmd.socket_path)
}

fn modify_usb(cmd: cmdline::UsbCommand) -> std::result::Result<(), ()> {
    let result = match cmd.command {
        cmdline::UsbSubCommand::Attach(cmd) => usb_attach(cmd),
        cmdline::UsbSubCommand::Detach(cmd) => usb_detach(cmd),
        cmdline::UsbSubCommand::List(cmd) => usb_list(cmd),
    };
    match result {
        Ok(response) => {
            println!("{}", response);
            Ok(())
        }
        Err(e) => {
            println!("error {}", e);
            Err(())
        }
    }
}

#[allow(clippy::unnecessary_wraps)]
fn pkg_version() -> std::result::Result<(), ()> {
    const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");
    const PKG_VERSION: Option<&'static str> = option_env!("PKG_VERSION");

    print!("crosvm {}", VERSION.unwrap_or("UNKNOWN"));
    match PKG_VERSION {
        Some(v) => println!("-{}", v),
        None => println!(),
    }
    Ok(())
}

// Returns true if the argument is a flag (e.g. `-s` or `--long`).
//
// As a special case, `-` is not treated as a flag, since it is typically used to represent
// `stdin`/`stdout`.
fn is_flag(arg: &str) -> bool {
    arg.len() > 1 && arg.starts_with('-')
}

// Perform transformations on `args_iter` to produce arguments suitable for parsing by `argh`.
fn prepare_argh_args<I: IntoIterator<Item = String>>(args_iter: I) -> Vec<String> {
    let mut args: Vec<String> = Vec::default();
    // http://b/235882579
    for arg in args_iter {
        match arg.as_str() {
            "--host_ip" => {
                eprintln!("`--host_ip` option is deprecated!");
                eprintln!("Please use `--host-ip` instead");
                args.push("--host-ip".to_string());
            }
            "--balloon_bias_mib" => {
                eprintln!("`--balloon_bias_mib` option is deprecated!");
                eprintln!("Please use `--balloon-bias-mib` instead");
                args.push("--balloon-bias-mib".to_string());
            }
            "-h" => args.push("--help".to_string()),
            // TODO(238361778): This block should work on windows as well.
            #[cfg(unix)]
            arg if is_flag(arg) => {
                // Split `--arg=val` into `--arg val`, since argh doesn't support the former.
                if let Some((key, value)) = arg.split_once("=") {
                    args.push(key.to_string());
                    args.push(value.to_string());
                } else {
                    args.push(arg.to_string());
                }
            }
            arg => args.push(arg.to_string()),
        }
    }

    args
}

fn crosvm_main() -> Result<CommandStatus> {
    let _library_watcher = sys::get_library_watcher();

    // The following panic hook will stop our crashpad hook on windows.
    // Only initialize when the crash-pad feature is off.
    #[cfg(not(feature = "crash-report"))]
    sys::set_panic_hook();

    // Ensure all processes detach from metrics on exit.
    #[cfg(windows)]
    let _metrics_destructor = metrics::get_destructor();

    let args = prepare_argh_args(std::env::args());
    let args = args.iter().map(|s| s.as_str()).collect::<Vec<_>>();
    let args = match crosvm::cmdline::CrosvmCmdlineArgs::from_args(&args[..1], &args[1..]) {
        Ok(args) => args,
        Err(e) => {
            println!("{}", e.output);
            return Ok(CommandStatus::Success);
        }
    };
    let extended_status = args.extended_status;

    let mut log_config = LogConfig {
        filter: &args.log_level,
        proc_name: args.syslog_tag.unwrap_or("crosvm".to_string()),
        syslog: !args.no_syslog,
        ..Default::default()
    };

    let ret = match args.command {
        Command::CrossPlatform(command) => {
            // Past this point, usage of exit is in danger of leaking zombie processes.
            if let CrossPlatformCommands::Run(cmd) = command {
                if let Some(syslog_tag) = &cmd.syslog_tag {
                    log_config.proc_name = syslog_tag.clone();
                }
                // We handle run_vm separately because it does not simply signal success/error
                // but also indicates whether the guest requested reset or stop.
                run_vm(cmd, log_config)
            } else if let CrossPlatformCommands::Device(cmd) = command {
                // On windows, the device command handles its own logging setup, so we can't handle it below
                // otherwise logging will double init.
                if cfg!(unix) {
                    syslog::init_with(log_config)
                        .map_err(|e| anyhow!("failed to initialize syslog: {}", e))?;
                }
                start_device(cmd)
                    .map_err(|_| anyhow!("start_device subcommand failed"))
                    .map(|_| CommandStatus::Success)
            } else {
                syslog::init_with(log_config)
                    .map_err(|e| anyhow!("failed to initialize syslog: {}", e))?;

                match command {
                    #[cfg(feature = "balloon")]
                    CrossPlatformCommands::Balloon(cmd) => {
                        balloon_vms(cmd).map_err(|_| anyhow!("balloon subcommand failed"))
                    }
                    #[cfg(feature = "balloon")]
                    CrossPlatformCommands::BalloonStats(cmd) => {
                        balloon_stats(cmd).map_err(|_| anyhow!("balloon_stats subcommand failed"))
                    }
                    CrossPlatformCommands::Battery(cmd) => {
                        modify_battery(cmd).map_err(|_| anyhow!("battery subcommand failed"))
                    }
                    #[cfg(feature = "composite-disk")]
                    CrossPlatformCommands::CreateComposite(cmd) => create_composite(cmd)
                        .map_err(|_| anyhow!("create_composite subcommand failed")),
                    CrossPlatformCommands::CreateQcow2(cmd) => {
                        create_qcow2(cmd).map_err(|_| anyhow!("create_qcow2 subcommand failed"))
                    }
                    CrossPlatformCommands::Device(_) => unreachable!(),
                    CrossPlatformCommands::Disk(cmd) => {
                        disk_cmd(cmd).map_err(|_| anyhow!("disk subcommand failed"))
                    }
                    CrossPlatformCommands::MakeRT(cmd) => {
                        make_rt(cmd).map_err(|_| anyhow!("make_rt subcommand failed"))
                    }
                    CrossPlatformCommands::Resume(cmd) => {
                        resume_vms(cmd).map_err(|_| anyhow!("resume subcommand failed"))
                    }
                    CrossPlatformCommands::Run(_) => unreachable!(),
                    CrossPlatformCommands::Stop(cmd) => {
                        stop_vms(cmd).map_err(|_| anyhow!("stop subcommand failed"))
                    }
                    CrossPlatformCommands::Suspend(cmd) => {
                        suspend_vms(cmd).map_err(|_| anyhow!("suspend subcommand failed"))
                    }
                    CrossPlatformCommands::Powerbtn(cmd) => {
                        powerbtn_vms(cmd).map_err(|_| anyhow!("powerbtn subcommand failed"))
                    }
                    CrossPlatformCommands::Sleepbtn(cmd) => {
                        sleepbtn_vms(cmd).map_err(|_| anyhow!("sleepbtn subcommand failed"))
                    }
                    CrossPlatformCommands::Gpe(cmd) => {
                        inject_gpe(cmd).map_err(|_| anyhow!("gpe subcommand failed"))
                    }
                    CrossPlatformCommands::Usb(cmd) => {
                        modify_usb(cmd).map_err(|_| anyhow!("usb subcommand failed"))
                    }
                    CrossPlatformCommands::Version(_) => {
                        pkg_version().map_err(|_| anyhow!("version subcommand failed"))
                    }
                    CrossPlatformCommands::Vfio(cmd) => {
                        modify_vfio(cmd).map_err(|_| anyhow!("vfio subcommand failed"))
                    }
                }
                .map(|_| CommandStatus::Success)
            }
        }
        cmdline::Command::Sys(command) => {
            // On windows, the sys commands handle their own logging setup, so we can't handle it
            // below otherwise logging will double init.
            if cfg!(unix) {
                syslog::init_with(log_config)
                    .map_err(|e| anyhow!("failed to initialize syslog: {}", e))?;
            }
            sys::run_command(command).map(|_| CommandStatus::Success)
        }
    };

    sys::cleanup();

    // WARNING: Any code added after this point is not guaranteed to run
    // since we may forcibly kill this process (and its children) above.
    ret.map(|s| {
        if extended_status {
            s
        } else {
            CommandStatus::Success
        }
    })
}

fn main() {
    let res = crosvm_main();
    let exit_code = match &res {
        Ok(CommandStatus::Success | CommandStatus::VmStop) => {
            info!("exiting with success");
            0
        }
        Ok(CommandStatus::VmReset) => {
            info!("exiting with reset");
            32
        }
        Ok(CommandStatus::VmCrash) => {
            info!("exiting with crash");
            33
        }
        Ok(CommandStatus::GuestPanic) => {
            info!("exiting with guest panic");
            34
        }
        Err(e) => {
            let exit_code = error_to_exit_code(&res);
            error!("exiting with error {}:{:?}", exit_code, e);
            exit_code
        }
    };
    std::process::exit(exit_code);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn args_is_flag() {
        assert!(is_flag("--test"));
        assert!(is_flag("-s"));

        assert!(!is_flag("-"));
        assert!(!is_flag("no-leading-dash"));
    }

    // TODO(b/238361778) this doesn't work on Windows because is_flag isn't called yet.
    #[cfg(unix)]
    #[test]
    fn args_split_long() {
        assert_eq!(
            prepare_argh_args(
                ["crosvm", "run", "--something=options", "vm_kernel"].map(|x| x.to_string())
            ),
            ["crosvm", "run", "--something", "options", "vm_kernel"]
        );
    }

    // TODO(b/238361778) this doesn't work on Windows because is_flag isn't called yet.
    #[cfg(unix)]
    #[test]
    fn args_split_short() {
        assert_eq!(
            prepare_argh_args(
                ["crosvm", "run", "-p=init=/bin/bash", "vm_kernel"].map(|x| x.to_string())
            ),
            ["crosvm", "run", "-p", "init=/bin/bash", "vm_kernel"]
        );
    }

    #[test]
    fn args_host_ip() {
        assert_eq!(
            prepare_argh_args(
                ["crosvm", "run", "--host_ip", "1.2.3.4", "vm_kernel"].map(|x| x.to_string())
            ),
            ["crosvm", "run", "--host-ip", "1.2.3.4", "vm_kernel"]
        );
    }

    #[test]
    fn args_balloon_bias_mib() {
        assert_eq!(
            prepare_argh_args(
                ["crosvm", "run", "--balloon_bias_mib", "1234", "vm_kernel"].map(|x| x.to_string())
            ),
            ["crosvm", "run", "--balloon-bias-mib", "1234", "vm_kernel"]
        );
    }

    #[test]
    fn args_h() {
        assert_eq!(
            prepare_argh_args(["crosvm", "run", "-h"].map(|x| x.to_string())),
            ["crosvm", "run", "--help"]
        );
    }

    #[test]
    fn args_battery_option() {
        assert_eq!(
            prepare_argh_args(
                [
                    "crosvm",
                    "run",
                    "--battery",
                    "type=goldfish",
                    "-p",
                    "init=/bin/bash",
                    "vm_kernel"
                ]
                .map(|x| x.to_string())
            ),
            [
                "crosvm",
                "run",
                "--battery",
                "type=goldfish",
                "-p",
                "init=/bin/bash",
                "vm_kernel"
            ]
        );
    }
}
