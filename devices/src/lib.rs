// Copyright 2017 The ChromiumOS Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#![cfg_attr(windows, allow(unused))]

//! Emulates virtual and hardware devices.

pub mod acpi;
pub mod bat;
mod bus;
#[cfg(feature = "stats")]
mod bus_stats;
mod cmos;
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
mod debugcon;
#[cfg(feature = "direct")]
pub mod direct_io;
#[cfg(feature = "direct")]
pub mod direct_irq;
mod i8042;
mod irq_event;
pub mod irqchip;
mod pci;
mod pflash;
pub mod pl030;
mod serial;
pub mod serial_device;
#[cfg(feature = "tpm")]
mod software_tpm;
mod suspendable;
mod sys;
pub mod virtio;
#[cfg(all(feature = "vtpm", target_arch = "x86_64"))]
mod vtpm_proxy;

cfg_if::cfg_if! {
    if #[cfg(any(target_arch = "x86", target_arch = "x86_64"))] {
        mod pit;
        pub use self::pit::{Pit, PitError};
        pub mod tsc;
    }
}

use std::collections::HashMap;
use std::collections::VecDeque;
use std::fs::OpenOptions;
use std::sync::Arc;

use anyhow::anyhow;
use base::error;
use base::info;
use base::Tube;
use base::TubeError;
use cros_async::AsyncTube;
use cros_async::Executor;
use vm_control::DeviceControlCommand;
use vm_control::RestoreControlResult;
use vm_control::SnapshotControlResult;

pub use self::acpi::ACPIPMFixedEvent;
pub use self::acpi::ACPIPMResource;
pub use self::bat::BatteryError;
pub use self::bat::GoldfishBattery;
pub use self::bus::Bus;
pub use self::bus::BusAccessInfo;
pub use self::bus::BusDevice;
pub use self::bus::BusDeviceObj;
pub use self::bus::BusDeviceSync;
pub use self::bus::BusRange;
pub use self::bus::BusResumeDevice;
pub use self::bus::BusType;
pub use self::bus::Error as BusError;
pub use self::bus::HostHotPlugKey;
pub use self::bus::HotPlugBus;
use self::bus::SerializedDevice;
#[cfg(feature = "stats")]
pub use self::bus_stats::BusStatistics;
pub use self::cmos::Cmos;
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub use self::debugcon::Debugcon;
#[cfg(feature = "direct")]
pub use self::direct_io::DirectIo;
#[cfg(feature = "direct")]
pub use self::direct_io::DirectMmio;
#[cfg(feature = "direct")]
pub use self::direct_irq::DirectIrq;
#[cfg(feature = "direct")]
pub use self::direct_irq::DirectIrqError;
pub use self::i8042::I8042Device;
pub use self::irq_event::IrqEdgeEvent;
pub use self::irq_event::IrqLevelEvent;
pub use self::irqchip::*;
#[cfg(feature = "audio")]
pub use self::pci::Ac97Backend;
#[cfg(feature = "audio")]
pub use self::pci::Ac97Dev;
#[cfg(feature = "audio")]
pub use self::pci::Ac97Parameters;
pub use self::pci::BarRange;
pub use self::pci::CrosvmDeviceId;
pub use self::pci::PciAddress;
pub use self::pci::PciAddressError;
pub use self::pci::PciBus;
pub use self::pci::PciClassCode;
pub use self::pci::PciConfigIo;
pub use self::pci::PciConfigMmio;
pub use self::pci::PciDevice;
pub use self::pci::PciDeviceError;
pub use self::pci::PciInterruptPin;
pub use self::pci::PciRoot;
pub use self::pci::PciRootCommand;
pub use self::pci::PciVirtualConfigMmio;
pub use self::pci::PreferredIrq;
pub use self::pci::StubPciDevice;
pub use self::pci::StubPciParameters;
pub use self::pflash::Pflash;
pub use self::pflash::PflashParameters;
pub use self::pl030::Pl030;
pub use self::serial::Serial;
pub use self::serial_device::Error as SerialError;
pub use self::serial_device::SerialDevice;
pub use self::serial_device::SerialHardware;
pub use self::serial_device::SerialParameters;
pub use self::serial_device::SerialType;
#[cfg(feature = "tpm")]
pub use self::software_tpm::SoftwareTpm;
pub use self::suspendable::DeviceState;
pub use self::suspendable::Suspendable;
pub use self::virtio::VirtioMmioDevice;
pub use self::virtio::VirtioPciDevice;
#[cfg(all(feature = "vtpm", target_arch = "x86_64"))]
pub use self::vtpm_proxy::VtpmProxy;

cfg_if::cfg_if! {
    if #[cfg(unix)] {
        mod platform;
        mod proxy;
        pub mod vmwdt;
        pub mod vfio;
        #[cfg(feature = "usb")]
        #[macro_use]
        mod register_space;
        #[cfg(feature = "usb")]
        pub mod usb;
        #[cfg(feature = "usb")]
        mod utils;

        pub use self::pci::{
            CoIommuDev, CoIommuParameters, CoIommuUnpinPolicy, PciBridge, PcieDownstreamPort,
            PcieHostPort, PcieRootPort, PcieUpstreamPort, PvPanicCode, PvPanicPciDevice,
            VfioPciDevice,
        };
        pub use self::platform::VfioPlatformDevice;
        pub use self::proxy::Error as ProxyError;
        pub use self::proxy::ProxyDevice;
        #[cfg(feature = "usb")]
        pub use self::usb::host_backend::host_backend_device_provider::HostBackendDeviceProvider;
        #[cfg(feature = "usb")]
        pub use self::usb::xhci::xhci_controller::XhciController;
        pub use self::vfio::{VfioContainer, VfioDevice};
        pub use self::virtio::vfio_wrapper;

    } else if #[cfg(windows)] {
        // We define Minijail as an empty struct on Windows because the concept
        // of jailing is baked into a bunch of places where it isn't easy
        // to compile it out. In the long term, this should go away.
        #[cfg(windows)]
        pub struct Minijail {}
    } else {
        compile_error!("Unsupported platform");
    }
}

/// Request CoIOMMU to unpin a specific range.
use serde::Deserialize;
/// Request CoIOMMU to unpin a specific range.
use serde::Serialize;
#[derive(Serialize, Deserialize, Debug)]
pub struct UnpinRequest {
    /// The ranges presents (start gfn, count).
    ranges: Vec<(u64, u64)>,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum UnpinResponse {
    Success,
    Failed,
}

#[derive(Debug)]
pub enum ParseIommuDevTypeResult {
    NoSuchType,
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum IommuDevType {
    NoIommu,
    VirtioIommu,
    CoIommu,
}

use std::str::FromStr;
impl FromStr for IommuDevType {
    type Err = ParseIommuDevTypeResult;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "off" => Ok(IommuDevType::NoIommu),
            "viommu" => Ok(IommuDevType::VirtioIommu),
            "coiommu" => Ok(IommuDevType::CoIommu),
            _ => Err(ParseIommuDevTypeResult::NoSuchType),
        }
    }
}

// Thread that handles commands sent to devices - such as snapshot, sleep, suspend
// Created when the VM is first created, and re-created on resumption of the VM.
pub fn create_devices_worker_thread(
    io_bus: Arc<Bus>,
    mmio_bus: Arc<Bus>,
    device_ctrl_resp: Tube,
) -> std::io::Result<std::thread::JoinHandle<()>> {
    std::thread::Builder::new()
        .name("device_control".to_string())
        .spawn(|| {
            let ex = Executor::new().expect("Failed to create an executor");

            let async_control = AsyncTube::new(&ex, device_ctrl_resp).unwrap();
            match ex.run_until(ex.spawn_local(async move {
                handle_command_tube(async_control, io_bus, mmio_bus).await
            })) {
                Ok(_) => {}
                Err(e) => {
                    error!("Device control thread exited with error: {}", e);
                }
            };
        })
}

fn sleep_devices(bus: &Bus) -> anyhow::Result<()> {
    match bus.sleep_devices() {
        Ok(_) => {
            info!("Devices slept successfully");
            Ok(())
        }
        Err(e) => {
            return Err(anyhow!(
                "Failed to sleep all devices: {}. Waking up sleeping devices.",
                e
            ));
        }
    }
}

fn wake_devices(bus: &Bus) {
    match bus.wake_devices() {
        Ok(_) => {
            info!("Devices awoken successfully");
        }
        Err(e) => {
            // Some devices may have slept. Eternally.
            // Recovery - impossible.
            // Shut down VM.
            panic!(
                "Failed to wake devices: {}. VM panicked to avoid unexpected behavior",
                e
            )
        }
    }
}

fn snapshot_devices(bus: &Bus, devices_vec: &mut Vec<SerializedDevice>) -> anyhow::Result<()> {
    match bus.snapshot_devices(devices_vec) {
        Ok(_) => {
            info!("Devices snapshot successfully");
            Ok(())
        }
        Err(e) => {
            // If snapshot fails, wake devices and return error
            error!("failed to snapshot devices: {}", e);
            Err(e)
        }
    }
}

fn restore_devices(
    bus: &Bus,
    devices_map: &mut HashMap<u32, VecDeque<String>>,
) -> anyhow::Result<()> {
    match bus.restore_devices(devices_map) {
        Ok(_) => {
            info!("Devices restore successfully");
            Ok(())
        }
        Err(e) => {
            // If restore fails, wake devices and return error
            error!("failed to restore devices: {}", e);
            Err(e)
        }
    }
}

async fn handle_command_tube(
    command_tube: AsyncTube,
    io_bus: Arc<Bus>,
    mmio_bus: Arc<Bus>,
) -> anyhow::Result<()> {
    'listener: loop {
        match command_tube.next().await {
            Ok(command) => {
                match command {
                    DeviceControlCommand::SnapshotDevices {
                        snapshot_path: path,
                    } => {
                        let mut devices_vec: Vec<SerializedDevice> = Vec::new();
                        let file_res = OpenOptions::new()
                            .create(true)
                            .write(true)
                            .truncate(true)
                            .open(&path);

                        let mut file = match file_res {
                            Ok(file) => file,
                            Err(e) => {
                                error!(
                                    "failed to open {} for writing snapshot: {}",
                                    path.as_path().display(),
                                    e
                                );
                                if let Err(e) = command_tube
                                    .send(SnapshotControlResult::Failed(e.to_string()))
                                    .await
                                {
                                    return Err(anyhow!("Failed to send response: {}", e));
                                }
                                continue;
                            }
                        };

                        let buses = [&io_bus, &mmio_bus];
                        for bus in &buses {
                            if let Err(e) = sleep_devices(bus) {
                                // Failing to sleep could mean a single device failing to sleep.
                                // Wake up devices to resume functionality of the VM.
                                for bus in &buses {
                                    wake_devices(bus);
                                }
                                if let Err(e) = command_tube
                                    .send(SnapshotControlResult::Failed(e.to_string()))
                                    .await
                                {
                                    return Err(anyhow!("Failed to send response: {}", e));
                                }
                                // After sending the error, continue to the initial loop and wait
                                // for a new event
                                continue 'listener;
                            }
                        }
                        for bus in &buses {
                            if let Err(e) = snapshot_devices(bus, &mut devices_vec) {
                                // If snapshot fails, wake devices and return error
                                error!("failed to snapshot devices: {}", e);
                                for bus in &buses {
                                    wake_devices(bus);
                                }
                                if let Err(e) = command_tube
                                    .send(SnapshotControlResult::Failed(e.to_string()))
                                    .await
                                {
                                    return Err(anyhow!("Failed to send response: {}", e));
                                }
                                // After sending the error, continue to the initial loop and wait
                                // for a new event
                                continue 'listener;
                            }
                        }
                        for bus in buses {
                            wake_devices(bus);
                        }

                        if let Err(e) = serde_json::to_writer(&mut file, &devices_vec) {
                            error!("failed to write serialized device to snapshot");
                            if let Err(e) = command_tube
                                .send(SnapshotControlResult::Failed(e.to_string()))
                                .await
                            {
                                return Err(anyhow!("Failed to send response: {}", e));
                            }
                        }
                        if let Err(e) = command_tube.send(SnapshotControlResult::Ok).await {
                            return Err(anyhow!("Failed to send response: {}", e));
                        }
                    }
                    DeviceControlCommand::RestoreDevices { restore_path: path } => {
                        let file_res = OpenOptions::new().read(true).write(false).open(&path);

                        let file = match file_res {
                            Ok(file) => file,
                            Err(e) => {
                                error!(
                                    "failed to open {} for writing snapshot: {}",
                                    path.as_path().display(),
                                    e
                                );
                                if let Err(e) = command_tube
                                    .send(SnapshotControlResult::Failed(e.to_string()))
                                    .await
                                {
                                    return Err(anyhow!("Failed to send response: {}", e));
                                }
                                continue;
                            }
                        };
                        let mut devices_map: HashMap<u32, VecDeque<String>> = HashMap::new();
                        let res = serde_json::from_reader(file);
                        let deserialized_list: Vec<SerializedDevice> = match res {
                            Err(e) => {
                                error!("failed to deserialize devices list: {}", e);
                                continue;
                            }
                            Ok(list) => list,
                        };
                        for deserialized_device in deserialized_list {
                            let device_id = deserialized_device.device_id;
                            let device = deserialized_device.serialized_device;
                            devices_map.entry(device_id).or_default().push_back(device);
                        }
                        let buses = [&io_bus, &mmio_bus];
                        for bus in &buses {
                            if let Err(e) = sleep_devices(bus) {
                                if let Err(e) = command_tube
                                    .send(SnapshotControlResult::Failed(e.to_string()))
                                    .await
                                {
                                    return Err(anyhow!("Failed to send response: {}", e));
                                }
                                // After sending the error, continue to the initial loop and wait
                                // for a new event
                                continue 'listener;
                            }
                        }
                        for bus in &buses {
                            if let Err(e) = restore_devices(bus, &mut devices_map) {
                                for bus in &buses {
                                    wake_devices(bus);
                                }
                                if let Err(e) = command_tube
                                    .send(RestoreControlResult::Failed(e.to_string()))
                                    .await
                                {
                                    return Err(anyhow!("Failed to send response: {}", e));
                                }
                                // After sending the error, continue to the initial loop and wait
                                // for a new event
                                continue 'listener;
                            }
                        }
                        for bus in buses {
                            wake_devices(bus);
                        }
                        for (key, _) in devices_map.iter().filter(|(_, v)| !v.is_empty()) {
                            info!("Device with device_id: {} did was not restored due to an error or the device might be missing.", key);
                        }
                        if let Err(e) = command_tube.send(RestoreControlResult::Ok).await {
                            return Err(anyhow!("Failed to send response: {}", e));
                        }
                    }
                };
            }
            Err(e) => {
                if matches!(e, TubeError::Disconnected) {
                    // Tube disconnected - shut down thread.
                    return Ok(());
                }
                return Err(anyhow!("Failed to receive: {}", e));
            }
        }
    }
}
