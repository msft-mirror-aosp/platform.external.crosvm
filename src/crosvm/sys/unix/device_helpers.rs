// Copyright 2017 The ChromiumOS Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use std::cell::Cell;
use std::collections::BTreeMap;
use std::convert::TryFrom;
use std::fs::OpenOptions;
use std::net::Ipv4Addr;
use std::ops::RangeInclusive;
use std::os::unix::net::UnixListener;
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::path::PathBuf;
use std::str;
use std::sync::Arc;

use anyhow::anyhow;
use anyhow::bail;
use anyhow::Context;
use anyhow::Result;
use arch::VirtioDeviceStub;
use base::*;
use devices::serial_device::SerialParameters;
use devices::serial_device::SerialType;
use devices::vfio::VfioCommonSetup;
use devices::vfio::VfioCommonTrait;
use devices::virtio;
use devices::virtio::block::block::DiskOption;
use devices::virtio::console::asynchronous::AsyncConsole;
#[cfg(any(feature = "video-decoder", feature = "video-encoder"))]
use devices::virtio::device_constants::video::VideoBackendType;
use devices::virtio::device_constants::video::VideoDeviceType;
use devices::virtio::ipc_memory_mapper::create_ipc_mapper;
use devices::virtio::ipc_memory_mapper::CreateIpcMapperRet;
use devices::virtio::memory_mapper::BasicMemoryMapper;
use devices::virtio::memory_mapper::MemoryMapperTrait;
#[cfg(feature = "audio")]
use devices::virtio::snd::parameters::Parameters as SndParameters;
use devices::virtio::vfio_wrapper::VfioWrapper;
use devices::virtio::vhost::user::proxy::VirtioVhostUser;
use devices::virtio::vhost::user::vmm::Block as VhostUserBlock;
use devices::virtio::vhost::user::vmm::Console as VhostUserConsole;
use devices::virtio::vhost::user::vmm::Fs as VhostUserFs;
use devices::virtio::vhost::user::vmm::Gpu as VhostUserGpu;
use devices::virtio::vhost::user::vmm::Mac80211Hwsim as VhostUserMac80211Hwsim;
use devices::virtio::vhost::user::vmm::Net as VhostUserNet;
use devices::virtio::vhost::user::vmm::Snd as VhostUserSnd;
use devices::virtio::vhost::user::vmm::Video as VhostUserVideo;
use devices::virtio::vhost::user::vmm::Vsock as VhostUserVsock;
use devices::virtio::vhost::user::vmm::Wl as VhostUserWl;
use devices::virtio::vhost::user::VhostUserDevice;
use devices::virtio::vhost::vsock::VhostVsockConfig;
#[cfg(feature = "balloon")]
use devices::virtio::BalloonMode;
use devices::virtio::VirtioDevice;
use devices::BusDeviceObj;
use devices::IommuDevType;
use devices::PciAddress;
use devices::PciDevice;
#[cfg(feature = "tpm")]
use devices::SoftwareTpm;
use devices::VfioDevice;
use devices::VfioPciDevice;
use devices::VfioPlatformDevice;
#[cfg(all(feature = "tpm", feature = "chromeos", target_arch = "x86_64"))]
use devices::VtpmProxy;
use hypervisor::ProtectionType;
use hypervisor::Vm;
use minijail::Minijail;
use net_util::sys::unix::Tap;
use net_util::MacAddress;
use resources::Alloc;
use resources::AllocOptions;
use resources::SystemAllocator;
use sync::Mutex;
use vm_memory::GuestAddress;

use super::jail_helpers::*;
use crate::crosvm::config::JailConfig;
use crate::crosvm::config::TouchDeviceOption;
use crate::crosvm::config::VhostUserFsOption;
use crate::crosvm::config::VhostUserOption;
use crate::crosvm::config::VvuOption;

pub enum TaggedControlTube {
    Fs(Tube),
    Vm(Tube),
    VmMemory(Tube),
    VmIrq(Tube),
    VmMsync(Tube),
}

impl AsRef<Tube> for TaggedControlTube {
    fn as_ref(&self) -> &Tube {
        use self::TaggedControlTube::*;
        match &self {
            Fs(tube) | Vm(tube) | VmMemory(tube) | VmIrq(tube) | VmMsync(tube) => tube,
        }
    }
}

impl AsRawDescriptor for TaggedControlTube {
    fn as_raw_descriptor(&self) -> RawDescriptor {
        self.as_ref().as_raw_descriptor()
    }
}

pub trait IntoUnixStream {
    fn into_unix_stream(self) -> Result<UnixStream>;
}

impl<'a> IntoUnixStream for &'a Path {
    fn into_unix_stream(self) -> Result<UnixStream> {
        if let Some(fd) = safe_descriptor_from_path(self).context("failed to open event device")? {
            Ok(fd.into())
        } else {
            UnixStream::connect(self).context("failed to open event device")
        }
    }
}

impl<'a> IntoUnixStream for &'a PathBuf {
    fn into_unix_stream(self) -> Result<UnixStream> {
        self.as_path().into_unix_stream()
    }
}

impl IntoUnixStream for UnixStream {
    fn into_unix_stream(self) -> Result<UnixStream> {
        Ok(self)
    }
}

pub type DeviceResult<T = VirtioDeviceStub> = Result<T>;

/// Type of virtio device.
///
/// The virtio protocol can be backed by several means, which affects a few things about the device
/// - for instance, the seccomp policy we need to use when jailing the device.
pub enum VirtioDeviceType {
    /// A regular (in-VMM) virtio device.
    Regular,
    /// Socket-backed vhost-user device.
    VhostUser,
    /// VFIO-backed vhost-user device, aka virtio-vhost-user.
    Vvu,
}

impl VirtioDeviceType {
    /// Returns the seccomp policy file that we will want to load depending on the jail type,
    /// constructed from `base`.
    fn seccomp_policy_file(&self, base: &str) -> String {
        match self {
            VirtioDeviceType::Regular => format!("{base}_device"),
            VirtioDeviceType::VhostUser => format!("{base}_device_vhost_user"),
            VirtioDeviceType::Vvu => format!("{base}_device_vvu"),
        }
    }
}

/// A trait for spawning virtio device instances and jails from their configuration structure.
///
/// Implementors become able to create virtio devices and jails following their own configuration.
/// This trait also provides a few convenience methods for e.g. creating a virtio device and jail
/// at once.
pub trait VirtioDeviceBuilder {
    /// Base name of the device, as it will appear in logs.
    const NAME: &'static str;

    /// Create a regular virtio device from the configuration and `protection_type` setting.
    fn create_virtio_device(
        &self,
        protection_type: ProtectionType,
    ) -> anyhow::Result<Box<dyn VirtioDevice>>;

    /// Create a device suitable for being run as a vhost-user instance.
    ///
    /// It is ok to leave this method unimplemented if the device is not intended to be used with
    /// vhost-user.
    fn create_vhost_user_device(
        &self,
        _keep_rds: &mut Vec<RawDescriptor>,
    ) -> anyhow::Result<Box<dyn VhostUserDevice>> {
        unimplemented!()
    }

    /// Create a jail that is suitable to run a device.
    ///
    /// The default implementation creates a simple jail with a seccomp policy derived from the
    /// base name of the device.
    fn create_jail(
        &self,
        jail_config: &Option<JailConfig>,
        jail_type: VirtioDeviceType,
    ) -> anyhow::Result<Option<Minijail>> {
        simple_jail(jail_config, &jail_type.seccomp_policy_file(Self::NAME))
    }

    /// Helper method to return a `VirtioDeviceStub` filled using `create_virtio_device` and
    /// `create_jail`.
    ///
    /// This helper should cover the needs of most devices when run as regular virtio devices.
    fn create_virtio_device_and_jail(
        &self,
        protection_type: ProtectionType,
        jail_config: &Option<JailConfig>,
    ) -> DeviceResult {
        Ok(VirtioDeviceStub {
            dev: self.create_virtio_device(protection_type)?,
            jail: self.create_jail(jail_config, VirtioDeviceType::Regular)?,
        })
    }
}

/// A one-shot configuration structure for implementing `VirtioDeviceBuilder`. We cannot do it on
/// `DiskOption` directly because disk devices can be passed an optional control tube.
pub struct DiskConfig<'a> {
    /// Options for disk creation.
    disk: &'a DiskOption,
    /// Optional control tube for the device. Placed behind a Cell so it can be taken from a
    /// non-mutable reference.
    device_tube: Cell<Option<Tube>>,
}

impl<'a> DiskConfig<'a> {
    pub fn new(disk: &'a DiskOption, device_tube: Option<Tube>) -> Self {
        Self {
            disk,
            device_tube: Cell::new(device_tube),
        }
    }
}

impl<'a> VirtioDeviceBuilder for DiskConfig<'a> {
    const NAME: &'static str = "block";

    fn create_virtio_device(
        &self,
        protection_type: ProtectionType,
    ) -> anyhow::Result<Box<dyn VirtioDevice>> {
        info!(
            "Trying to attach block device: {}",
            self.disk.path.display(),
        );
        let disk_image = self.disk.open()?;

        let disk_device_tube = self.device_tube.take();
        Ok(Box::new(
            virtio::BlockAsync::new(
                virtio::base_features(protection_type),
                disk_image,
                self.disk.read_only,
                self.disk.sparse,
                self.disk.block_size,
                self.disk.id,
                disk_device_tube,
            )
            .context("failed to create block device")?,
        ))
    }

    fn create_vhost_user_device(
        &self,
        keep_rds: &mut Vec<RawDescriptor>,
    ) -> anyhow::Result<Box<dyn VhostUserDevice>> {
        let disk = self.disk;

        let disk_device_tube = self.device_tube.take();
        if let Some(device_tube) = &disk_device_tube {
            keep_rds.push(device_tube.as_raw_descriptor());
        }

        let disk_image = disk.open()?;
        keep_rds.extend(disk_image.as_raw_descriptors());

        let block = Box::new(
            virtio::BlockAsync::new(
                virtio::base_features(ProtectionType::Unprotected),
                disk_image,
                disk.read_only,
                disk.sparse,
                disk.block_size,
                disk.id,
                disk_device_tube,
            )
            .context("failed to create block device")?,
        );

        Ok(block)
    }
}

pub fn create_vhost_user_block_device(
    protection_type: ProtectionType,
    opt: &VhostUserOption,
) -> DeviceResult {
    let dev = VhostUserBlock::new(virtio::base_features(protection_type), &opt.socket)
        .context("failed to set up vhost-user block device")?;

    Ok(VirtioDeviceStub {
        dev: Box::new(dev),
        // no sandbox here because virtqueue handling is exported to a different process.
        jail: None,
    })
}

pub fn create_vhost_user_console_device(
    protection_type: ProtectionType,
    opt: &VhostUserOption,
) -> DeviceResult {
    let dev = VhostUserConsole::new(virtio::base_features(protection_type), &opt.socket)
        .context("failed to set up vhost-user console device")?;

    Ok(VirtioDeviceStub {
        dev: Box::new(dev),
        // no sandbox here because virtqueue handling is exported to a different process.
        jail: None,
    })
}

pub fn create_vhost_user_fs_device(
    protection_type: ProtectionType,
    option: &VhostUserFsOption,
) -> DeviceResult {
    let dev = VhostUserFs::new(
        virtio::base_features(protection_type),
        &option.socket,
        &option.tag,
    )
    .context("failed to set up vhost-user fs device")?;

    Ok(VirtioDeviceStub {
        dev: Box::new(dev),
        // no sandbox here because virtqueue handling is exported to a different process.
        jail: None,
    })
}

pub fn create_vhost_user_mac80211_hwsim_device(
    protection_type: ProtectionType,
    opt: &VhostUserOption,
) -> DeviceResult {
    let dev = VhostUserMac80211Hwsim::new(virtio::base_features(protection_type), &opt.socket)
        .context("failed to set up vhost-user mac80211_hwsim device")?;

    Ok(VirtioDeviceStub {
        dev: Box::new(dev),
        // no sandbox here because virtqueue handling is exported to a different process.
        jail: None,
    })
}

pub fn create_vhost_user_snd_device(
    protection_type: ProtectionType,
    option: &VhostUserOption,
) -> DeviceResult {
    let dev = VhostUserSnd::new(virtio::base_features(protection_type), &option.socket)
        .context("failed to set up vhost-user snd device")?;

    Ok(VirtioDeviceStub {
        dev: Box::new(dev),
        // no sandbox here because virtqueue handling is exported to a different process.
        jail: None,
    })
}

pub fn create_vhost_user_gpu_device(
    protection_type: ProtectionType,
    opt: &VhostUserOption,
) -> DeviceResult {
    // The crosvm gpu device expects us to connect the tube before it will accept a vhost-user
    // connection.
    let dev = VhostUserGpu::new(virtio::base_features(protection_type), &opt.socket)
        .context("failed to set up vhost-user gpu device")?;

    Ok(VirtioDeviceStub {
        dev: Box::new(dev),
        // no sandbox here because virtqueue handling is exported to a different process.
        jail: None,
    })
}

pub fn create_vvu_proxy_device(
    protection_type: ProtectionType,
    jail_config: &Option<JailConfig>,
    opt: &VvuOption,
    tube: Tube,
    max_sibling_mem_size: u64,
) -> DeviceResult {
    let listener =
        UnixListener::bind(&opt.socket).context("failed to bind listener for vvu proxy device")?;

    let dev = VirtioVhostUser::new(
        virtio::base_features(protection_type),
        listener,
        tube,
        opt.addr,
        opt.uuid,
        max_sibling_mem_size,
    )
    .context("failed to create VVU proxy device")?;

    Ok(VirtioDeviceStub {
        dev: Box::new(dev),
        jail: simple_jail(jail_config, "vvu_proxy_device")?,
    })
}

pub fn create_rng_device(
    protection_type: ProtectionType,
    jail_config: &Option<JailConfig>,
) -> DeviceResult {
    let dev =
        virtio::Rng::new(virtio::base_features(protection_type)).context("failed to set up rng")?;

    Ok(VirtioDeviceStub {
        dev: Box::new(dev),
        jail: simple_jail(jail_config, "rng_device")?,
    })
}

#[cfg(feature = "audio")]
pub fn create_virtio_snd_device(
    protection_type: ProtectionType,
    jail_config: &Option<JailConfig>,
    snd_params: SndParameters,
) -> DeviceResult {
    let backend = snd_params.backend;
    let dev = virtio::snd::common_backend::VirtioSnd::new(
        virtio::base_features(protection_type),
        snd_params,
    )
    .context("failed to create cras sound device")?;

    use virtio::snd::parameters::StreamSourceBackend as Backend;

    let policy = match backend {
        Backend::NULL => "snd_null_device",
        #[cfg(feature = "audio_cras")]
        Backend::Sys(virtio::snd::sys::StreamSourceBackend::CRAS) => "snd_cras_device",
        #[cfg(not(feature = "audio_cras"))]
        _ => unreachable!(),
    };

    let jail = match simple_jail(jail_config, policy)? {
        Some(mut jail) => {
            // Create a tmpfs in the device's root directory for cras_snd_device.
            // The size is 20*1024, or 20 KB.
            #[cfg(feature = "audio_cras")]
            if backend == Backend::Sys(virtio::snd::sys::StreamSourceBackend::CRAS) {
                jail.mount_with_data(
                    Path::new("none"),
                    Path::new("/"),
                    "tmpfs",
                    (libc::MS_NOSUID | libc::MS_NODEV | libc::MS_NOEXEC) as usize,
                    "size=20480",
                )?;

                let run_cras_path = Path::new("/run/cras");
                jail.mount_bind(run_cras_path, run_cras_path, true)?;
            }

            add_current_user_to_jail(&mut jail)?;

            Some(jail)
        }
        None => None,
    };

    Ok(VirtioDeviceStub {
        dev: Box::new(dev),
        jail,
    })
}

#[cfg(feature = "tpm")]
pub fn create_software_tpm_device(
    protection_type: ProtectionType,
    jail_config: &Option<JailConfig>,
) -> DeviceResult {
    use std::ffi::CString;
    use std::fs;
    use std::process;

    let tpm_storage: PathBuf;
    let mut tpm_jail = simple_jail(jail_config, "tpm_device")?;

    match &mut tpm_jail {
        Some(jail) => {
            // Create a tmpfs in the device's root directory for tpm
            // simulator storage. The size is 20*1024, or 20 KB.
            jail.mount_with_data(
                Path::new("none"),
                Path::new("/"),
                "tmpfs",
                (libc::MS_NOSUID | libc::MS_NODEV | libc::MS_NOEXEC) as usize,
                "size=20480",
            )?;

            let crosvm_ids = add_current_user_to_jail(jail)?;

            let pid = process::id();
            let tpm_pid_dir = format!("/run/vm/tpm.{}", pid);
            tpm_storage = Path::new(&tpm_pid_dir).to_owned();
            fs::create_dir_all(&tpm_storage).with_context(|| {
                format!("failed to create tpm storage dir {}", tpm_storage.display())
            })?;
            let tpm_pid_dir_c = CString::new(tpm_pid_dir).expect("no nul bytes");
            chown(&tpm_pid_dir_c, crosvm_ids.uid, crosvm_ids.gid)
                .context("failed to chown tpm storage")?;

            jail.mount_bind(&tpm_storage, &tpm_storage, true)?;
        }
        None => {
            // Path used inside cros_sdk which does not have /run/vm.
            tpm_storage = Path::new("/tmp/tpm-simulator").to_owned();
        }
    }

    let backend = SoftwareTpm::new(tpm_storage).context("failed to create SoftwareTpm")?;
    let dev = virtio::Tpm::new(Box::new(backend), virtio::base_features(protection_type));

    Ok(VirtioDeviceStub {
        dev: Box::new(dev),
        jail: tpm_jail,
    })
}

#[cfg(all(feature = "tpm", feature = "chromeos", target_arch = "x86_64"))]
pub fn create_vtpm_proxy_device(
    protection_type: ProtectionType,
    jail_config: &Option<JailConfig>,
) -> DeviceResult {
    let mut tpm_jail = simple_jail(jail_config, "vtpm_proxy_device")?;

    match &mut tpm_jail {
        Some(jail) => {
            // Create a tmpfs in the device's root directory so that we can bind mount files.
            // The size is 20*1024, or 20 KB.
            jail.mount_with_data(
                Path::new("none"),
                Path::new("/"),
                "tmpfs",
                (libc::MS_NOSUID | libc::MS_NODEV | libc::MS_NOEXEC) as usize,
                "size=20480",
            )?;

            add_current_user_to_jail(jail)?;

            let system_bus_socket_path = Path::new("/run/dbus/system_bus_socket");
            jail.mount_bind(system_bus_socket_path, system_bus_socket_path, true)?;
        }
        None => {}
    }

    let backend = VtpmProxy::new();
    let dev = virtio::Tpm::new(Box::new(backend), virtio::base_features(protection_type));

    Ok(VirtioDeviceStub {
        dev: Box::new(dev),
        jail: tpm_jail,
    })
}

pub fn create_single_touch_device(
    protection_type: ProtectionType,
    jail_config: &Option<JailConfig>,
    single_touch_spec: &TouchDeviceOption,
    idx: u32,
) -> DeviceResult {
    let socket = single_touch_spec
        .get_path()
        .into_unix_stream()
        .context("failed configuring virtio single touch")?;

    let (width, height) = single_touch_spec.get_size();
    let dev = virtio::new_single_touch(
        idx,
        socket,
        width,
        height,
        virtio::base_features(protection_type),
    )
    .context("failed to set up input device")?;
    Ok(VirtioDeviceStub {
        dev: Box::new(dev),
        jail: simple_jail(jail_config, "input_device")?,
    })
}

pub fn create_multi_touch_device(
    protection_type: ProtectionType,
    jail_config: &Option<JailConfig>,
    multi_touch_spec: &TouchDeviceOption,
    idx: u32,
) -> DeviceResult {
    let socket = multi_touch_spec
        .get_path()
        .into_unix_stream()
        .context("failed configuring virtio multi touch")?;

    let (width, height) = multi_touch_spec.get_size();
    let dev = virtio::new_multi_touch(
        idx,
        socket,
        width,
        height,
        virtio::base_features(protection_type),
    )
    .context("failed to set up input device")?;

    Ok(VirtioDeviceStub {
        dev: Box::new(dev),
        jail: simple_jail(jail_config, "input_device")?,
    })
}

pub fn create_trackpad_device(
    protection_type: ProtectionType,
    jail_config: &Option<JailConfig>,
    trackpad_spec: &TouchDeviceOption,
    idx: u32,
) -> DeviceResult {
    let socket = trackpad_spec
        .get_path()
        .into_unix_stream()
        .context("failed configuring virtio trackpad")?;

    let (width, height) = trackpad_spec.get_size();
    let dev = virtio::new_trackpad(
        idx,
        socket,
        width,
        height,
        virtio::base_features(protection_type),
    )
    .context("failed to set up input device")?;

    Ok(VirtioDeviceStub {
        dev: Box::new(dev),
        jail: simple_jail(jail_config, "input_device")?,
    })
}

pub fn create_mouse_device<T: IntoUnixStream>(
    protection_type: ProtectionType,
    jail_config: &Option<JailConfig>,
    mouse_socket: T,
    idx: u32,
) -> DeviceResult {
    let socket = mouse_socket
        .into_unix_stream()
        .context("failed configuring virtio mouse")?;

    let dev = virtio::new_mouse(idx, socket, virtio::base_features(protection_type))
        .context("failed to set up input device")?;

    Ok(VirtioDeviceStub {
        dev: Box::new(dev),
        jail: simple_jail(jail_config, "input_device")?,
    })
}

pub fn create_keyboard_device<T: IntoUnixStream>(
    protection_type: ProtectionType,
    jail_config: &Option<JailConfig>,
    keyboard_socket: T,
    idx: u32,
) -> DeviceResult {
    let socket = keyboard_socket
        .into_unix_stream()
        .context("failed configuring virtio keyboard")?;

    let dev = virtio::new_keyboard(idx, socket, virtio::base_features(protection_type))
        .context("failed to set up input device")?;

    Ok(VirtioDeviceStub {
        dev: Box::new(dev),
        jail: simple_jail(jail_config, "input_device")?,
    })
}

pub fn create_switches_device<T: IntoUnixStream>(
    protection_type: ProtectionType,
    jail_config: &Option<JailConfig>,
    switches_socket: T,
    idx: u32,
) -> DeviceResult {
    let socket = switches_socket
        .into_unix_stream()
        .context("failed configuring virtio switches")?;

    let dev = virtio::new_switches(idx, socket, virtio::base_features(protection_type))
        .context("failed to set up input device")?;

    Ok(VirtioDeviceStub {
        dev: Box::new(dev),
        jail: simple_jail(jail_config, "input_device")?,
    })
}

pub fn create_vinput_device(
    protection_type: ProtectionType,
    jail_config: &Option<JailConfig>,
    dev_path: &Path,
) -> DeviceResult {
    let dev_file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(dev_path)
        .with_context(|| format!("failed to open vinput device {}", dev_path.display()))?;

    let dev = virtio::new_evdev(dev_file, virtio::base_features(protection_type))
        .context("failed to set up input device")?;

    Ok(VirtioDeviceStub {
        dev: Box::new(dev),
        jail: simple_jail(jail_config, "input_device")?,
    })
}

#[cfg(feature = "balloon")]
pub fn create_balloon_device(
    protection_type: ProtectionType,
    jail_config: &Option<JailConfig>,
    mode: BalloonMode,
    tube: Tube,
    inflate_tube: Option<Tube>,
    init_balloon_size: u64,
    enabled_features: u64,
) -> DeviceResult {
    let dev = virtio::Balloon::new(
        virtio::base_features(protection_type),
        tube,
        inflate_tube,
        init_balloon_size,
        mode,
        enabled_features,
    )
    .context("failed to create balloon")?;

    Ok(VirtioDeviceStub {
        dev: Box::new(dev),
        jail: simple_jail(jail_config, "balloon_device")?,
    })
}

/// Generic method for creating a network device. `create_device` is a closure that takes the virtio
/// features and number of queue pairs as parameters, and is responsible for creating the device
/// itself.
pub fn create_net_device<F, T>(
    protection_type: ProtectionType,
    jail_config: &Option<JailConfig>,
    mut vq_pairs: u16,
    vcpu_count: usize,
    policy: &str,
    create_device: F,
) -> DeviceResult
where
    F: Fn(u64, u16) -> Result<T>,
    T: VirtioDevice + 'static,
{
    if vcpu_count < vq_pairs as usize {
        warn!("the number of net vq pairs must not exceed the vcpu count, falling back to single queue mode");
        vq_pairs = 1;
    }
    let features = virtio::base_features(protection_type);

    let dev = create_device(features, vq_pairs)?;

    Ok(VirtioDeviceStub {
        dev: Box::new(dev) as Box<dyn VirtioDevice>,
        jail: simple_jail(jail_config, policy)?,
    })
}

/// Returns a network device created from a new TAP interface configured with `host_ip`, `netmask`,
/// and `mac_address`.
pub fn create_net_device_from_config(
    protection_type: ProtectionType,
    jail_config: &Option<JailConfig>,
    vq_pairs: u16,
    vcpu_count: usize,
    vhost_net: Option<PathBuf>,
    host_ip: Ipv4Addr,
    netmask: Ipv4Addr,
    mac_address: MacAddress,
) -> DeviceResult {
    if let Some(vhost_net_device_path) = vhost_net {
        create_net_device(
            protection_type,
            jail_config,
            vq_pairs,
            vcpu_count,
            "vhost_net_device",
            |features, _vq_pairs| {
                virtio::vhost::Net::<Tap, vhost::Net<Tap>>::new(
                    &vhost_net_device_path,
                    features,
                    host_ip,
                    netmask,
                    mac_address,
                )
                .context("failed to set up vhost networking")
            },
        )
    } else {
        create_net_device(
            protection_type,
            jail_config,
            vq_pairs,
            vcpu_count,
            "net_device",
            |features, vq_pairs| {
                virtio::Net::<Tap>::new(features, host_ip, netmask, mac_address, vq_pairs)
                    .context("failed to create virtio network device")
            },
        )
    }
}

/// Returns a network device from a file descriptor to a configured TAP interface.
pub fn create_tap_net_device_from_fd(
    protection_type: ProtectionType,
    jail_config: &Option<JailConfig>,
    vq_pairs: u16,
    vcpu_count: usize,
    tap_fd: RawDescriptor,
) -> DeviceResult {
    create_net_device(
        protection_type,
        jail_config,
        vq_pairs,
        vcpu_count,
        "net_device",
        |features, vq_pairs| {
            // Safe because we ensure that we get a unique handle to the fd.
            let tap = unsafe {
                Tap::from_raw_descriptor(
                    validate_raw_descriptor(tap_fd).context("failed to validate tap descriptor")?,
                )
                .context("failed to create tap device")?
            };

            virtio::Net::from(features, tap, vq_pairs).context("failed to create tap net device")
        },
    )
}

/// Returns a network device created by opening the persistent, configured TAP interface `tap_name`.
pub fn create_tap_net_device_from_name(
    protection_type: ProtectionType,
    jail_config: &Option<JailConfig>,
    vq_pairs: u16,
    vcpu_count: usize,
    tap_name: &[u8],
) -> DeviceResult {
    create_net_device(
        protection_type,
        jail_config,
        vq_pairs,
        vcpu_count,
        "net_device",
        |features, vq_pairs| {
            virtio::Net::<Tap>::new_from_name(features, tap_name, vq_pairs)
                .context("failed to create configured virtio network device")
        },
    )
}

pub fn create_vhost_user_net_device(
    protection_type: ProtectionType,
    opt: &VhostUserOption,
) -> DeviceResult {
    let dev = VhostUserNet::new(virtio::base_features(protection_type), &opt.socket)
        .context("failed to set up vhost-user net device")?;

    Ok(VirtioDeviceStub {
        dev: Box::new(dev),
        // no sandbox here because virtqueue handling is exported to a different process.
        jail: None,
    })
}

pub fn create_vhost_user_vsock_device(
    protection_type: ProtectionType,
    opt: &VhostUserOption,
) -> DeviceResult {
    let dev = VhostUserVsock::new(virtio::base_features(protection_type), &opt.socket)
        .context("failed to set up vhost-user vsock device")?;

    Ok(VirtioDeviceStub {
        dev: Box::new(dev),
        // no sandbox here because virtqueue handling is exported to a different process.
        jail: None,
    })
}

pub fn create_vhost_user_wl_device(
    protection_type: ProtectionType,
    opt: &VhostUserOption,
) -> DeviceResult {
    // The crosvm wl device expects us to connect the tube before it will accept a vhost-user
    // connection.
    let dev = VhostUserWl::new(virtio::base_features(protection_type), &opt.socket)
        .context("failed to set up vhost-user wl device")?;

    Ok(VirtioDeviceStub {
        dev: Box::new(dev),
        // no sandbox here because virtqueue handling is exported to a different process.
        jail: None,
    })
}

pub fn create_wayland_device(
    protection_type: ProtectionType,
    jail_config: &Option<JailConfig>,
    wayland_socket_paths: &BTreeMap<String, PathBuf>,
    resource_bridge: Option<Tube>,
) -> DeviceResult {
    let wayland_socket_dirs = wayland_socket_paths
        .iter()
        .map(|(_name, path)| path.parent())
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| anyhow!("wayland socket path has no parent or file name"))?;

    let features = virtio::base_features(protection_type);
    let dev = virtio::Wl::new(features, wayland_socket_paths.clone(), resource_bridge)
        .context("failed to create wayland device")?;

    let jail = match gpu_jail(jail_config, "wl_device")? {
        Some(mut jail) => {
            // Bind mount the wayland socket's directory into jail's root. This is necessary since
            // each new wayland context must open() the socket. If the wayland socket is ever
            // destroyed and remade in the same host directory, new connections will be possible
            // without restarting the wayland device.
            for dir in &wayland_socket_dirs {
                jail.mount_bind(dir, dir, true)?;
            }
            add_current_user_to_jail(&mut jail)?;

            Some(jail)
        }
        None => None,
    };

    Ok(VirtioDeviceStub {
        dev: Box::new(dev),
        jail,
    })
}

#[cfg(any(feature = "video-decoder", feature = "video-encoder"))]
pub fn create_video_device(
    backend: VideoBackendType,
    protection_type: ProtectionType,
    jail_config: &Option<JailConfig>,
    typ: VideoDeviceType,
    resource_bridge: Tube,
) -> DeviceResult {
    let jail = match simple_jail(jail_config, "video_device")? {
        Some(mut jail) => {
            match typ {
                #[cfg(feature = "video-decoder")]
                VideoDeviceType::Decoder => add_current_user_to_jail(&mut jail)?,
                #[cfg(feature = "video-encoder")]
                VideoDeviceType::Encoder => add_current_user_to_jail(&mut jail)?,
                #[cfg(any(not(feature = "video-decoder"), not(feature = "video-encoder")))]
                // `typ` is always a VideoDeviceType enabled
                device_type => unreachable!("Not compiled with {:?} enabled", device_type),
            };

            // Create a tmpfs in the device's root directory so that we can bind mount files.
            jail.mount_with_data(
                Path::new("none"),
                Path::new("/"),
                "tmpfs",
                (libc::MS_NOSUID | libc::MS_NODEV | libc::MS_NOEXEC) as usize,
                "size=67108864",
            )?;

            #[cfg(feature = "libvda")]
            // Render node for libvda.
            if backend == VideoBackendType::Libvda || backend == VideoBackendType::LibvdaVd {
                // follow the implementation at:
                // https://chromium.googlesource.com/chromiumos/platform/minigbm/+/c06cc9cccb3cf3c7f9d2aec706c27c34cd6162a0/cros_gralloc/cros_gralloc_driver.cc#90
                const DRM_NUM_NODES: u32 = 63;
                const DRM_RENDER_NODE_START: u32 = 128;
                for offset in 0..DRM_NUM_NODES {
                    let path_str = format!("/dev/dri/renderD{}", DRM_RENDER_NODE_START + offset);
                    let dev_dri_path = Path::new(&path_str);
                    if !dev_dri_path.exists() {
                        break;
                    }
                    jail.mount_bind(dev_dri_path, dev_dri_path, false)?;
                }
            }

            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            {
                // Device nodes used by libdrm through minigbm in libvda on AMD devices.
                let sys_dev_char_path = Path::new("/sys/dev/char");
                jail.mount_bind(sys_dev_char_path, sys_dev_char_path, false)?;
                let sys_devices_path = Path::new("/sys/devices");
                jail.mount_bind(sys_devices_path, sys_devices_path, false)?;

                // Required for loading dri libraries loaded by minigbm on AMD devices.
                jail_mount_bind_if_exists(&mut jail, &["/usr/lib64"])?;
            }

            // Device nodes required by libchrome which establishes Mojo connection in libvda.
            let dev_urandom_path = Path::new("/dev/urandom");
            jail.mount_bind(dev_urandom_path, dev_urandom_path, false)?;
            let system_bus_socket_path = Path::new("/run/dbus/system_bus_socket");
            jail.mount_bind(system_bus_socket_path, system_bus_socket_path, true)?;

            Some(jail)
        }
        None => None,
    };

    Ok(VirtioDeviceStub {
        dev: Box::new(devices::virtio::VideoDevice::new(
            virtio::base_features(protection_type),
            typ,
            backend,
            Some(resource_bridge),
        )),
        jail,
    })
}

#[cfg(any(feature = "video-decoder", feature = "video-encoder"))]
pub fn register_video_device(
    backend: VideoBackendType,
    devs: &mut Vec<VirtioDeviceStub>,
    video_tube: Tube,
    protection_type: ProtectionType,
    jail_config: &Option<JailConfig>,
    typ: VideoDeviceType,
) -> Result<()> {
    devs.push(create_video_device(
        backend,
        protection_type,
        jail_config,
        typ,
        video_tube,
    )?);
    Ok(())
}

pub fn create_vhost_user_video_device(
    protection_type: ProtectionType,
    opt: &VhostUserOption,
    device_type: VideoDeviceType,
) -> DeviceResult {
    let dev = VhostUserVideo::new(
        virtio::base_features(protection_type),
        &opt.socket,
        device_type,
    )
    .context("failed to set up vhost-user video device")?;

    Ok(VirtioDeviceStub {
        dev: Box::new(dev),
        // no sandbox here because virtqueue handling is exported to a different process.
        jail: None,
    })
}

pub fn create_vhost_vsock_device(
    protection_type: ProtectionType,
    jail_config: &Option<JailConfig>,
    vhost_config: &VhostVsockConfig,
) -> DeviceResult {
    let features = virtio::base_features(protection_type);

    let dev = virtio::vhost::Vsock::new(features, vhost_config)
        .context("failed to set up virtual socket device")?;

    Ok(VirtioDeviceStub {
        dev: Box::new(dev),
        jail: simple_jail(jail_config, "vhost_vsock_device")?,
    })
}

pub fn create_fs_device(
    protection_type: ProtectionType,
    jail_config: &Option<JailConfig>,
    uid_map: &str,
    gid_map: &str,
    src: &Path,
    tag: &str,
    fs_cfg: virtio::fs::passthrough::Config,
    device_tube: Tube,
) -> DeviceResult {
    let max_open_files =
        base::get_max_open_files().context("failed to get max number of open files")?;
    let j = if let Some(jail_config) = jail_config {
        let policy_path = jail_config
            .seccomp_policy_dir
            .as_ref()
            .map(|dir| dir.join("fs_device"));
        let config = SandboxConfig {
            limit_caps: false,
            uid_map: Some(uid_map),
            gid_map: Some(gid_map),
            log_failures: jail_config.seccomp_log_failures,
            seccomp_policy_path: policy_path.as_deref(),
            seccomp_policy_name: "fs_device",
            // We want bind mounts from the parent namespaces to propagate into the fs device's
            // namespace.
            remount_mode: Some(libc::MS_SLAVE),
        };
        create_base_minijail(src, Some(max_open_files), Some(&config))?
    } else {
        create_base_minijail(src, Some(max_open_files), None)?
    };

    let features = virtio::base_features(protection_type);
    // TODO(chirantan): Use more than one worker once the kernel driver has been fixed to not panic
    // when num_queues > 1.
    let dev = virtio::fs::Fs::new(features, tag, 1, fs_cfg, device_tube)
        .context("failed to create fs device")?;

    Ok(VirtioDeviceStub {
        dev: Box::new(dev),
        jail: Some(j),
    })
}

pub fn create_9p_device(
    protection_type: ProtectionType,
    jail_config: &Option<JailConfig>,
    uid_map: &str,
    gid_map: &str,
    src: &Path,
    tag: &str,
    mut p9_cfg: p9::Config,
) -> DeviceResult {
    let max_open_files =
        base::get_max_open_files().context("failed to get max number of open files")?;
    let (jail, root) = if let Some(jail_config) = jail_config {
        let policy_path = jail_config
            .seccomp_policy_dir
            .as_ref()
            .map(|dir| dir.join("9p_device"));
        let config = SandboxConfig {
            limit_caps: false,
            uid_map: Some(uid_map),
            gid_map: Some(gid_map),
            log_failures: jail_config.seccomp_log_failures,
            seccomp_policy_path: policy_path.as_deref(),
            seccomp_policy_name: "9p_device",
            // We want bind mounts from the parent namespaces to propagate into the 9p server's
            // namespace.
            remount_mode: Some(libc::MS_SLAVE),
        };

        let jail = create_base_minijail(src, Some(max_open_files), Some(&config))?;

        //  The shared directory becomes the root of the device's file system.
        let root = Path::new("/");
        (Some(jail), root)
    } else {
        // There's no mount namespace so we tell the server to treat the source directory as the
        // root.
        (None, src)
    };

    let features = virtio::base_features(protection_type);
    p9_cfg.root = root.into();
    let dev = virtio::P9::new(features, tag, p9_cfg).context("failed to create 9p device")?;

    Ok(VirtioDeviceStub {
        dev: Box::new(dev),
        jail,
    })
}

pub fn create_pmem_device(
    protection_type: ProtectionType,
    jail_config: &Option<JailConfig>,
    vm: &mut impl Vm,
    resources: &mut SystemAllocator,
    disk: &DiskOption,
    index: usize,
    pmem_device_tube: Tube,
) -> DeviceResult {
    let fd = open_file(
        &disk.path,
        OpenOptions::new().read(true).write(!disk.read_only),
    )
    .with_context(|| format!("failed to load disk image {}", disk.path.display()))?;

    let (disk_size, arena_size) = {
        let metadata = std::fs::metadata(&disk.path).with_context(|| {
            format!("failed to get disk image {} metadata", disk.path.display())
        })?;
        let disk_len = metadata.len();
        // Linux requires pmem region sizes to be 2 MiB aligned. Linux will fill any partial page
        // at the end of an mmap'd file and won't write back beyond the actual file length, but if
        // we just align the size of the file to 2 MiB then access beyond the last page of the
        // mapped file will generate SIGBUS. So use a memory mapping arena that will provide
        // padding up to 2 MiB.
        let alignment = 2 * 1024 * 1024;
        let align_adjust = if disk_len % alignment != 0 {
            alignment - (disk_len % alignment)
        } else {
            0
        };
        (
            disk_len,
            disk_len
                .checked_add(align_adjust)
                .ok_or_else(|| anyhow!("pmem device image too big"))?,
        )
    };

    let protection = {
        if disk.read_only {
            Protection::read()
        } else {
            Protection::read_write()
        }
    };

    let arena = {
        // Conversion from u64 to usize may fail on 32bit system.
        let arena_size = usize::try_from(arena_size).context("pmem device image too big")?;
        let disk_size = usize::try_from(disk_size).context("pmem device image too big")?;

        let mut arena =
            MemoryMappingArena::new(arena_size).context("failed to reserve pmem memory")?;
        arena
            .add_fd_offset_protection(0, disk_size, &fd, 0, protection)
            .context("failed to reserve pmem memory")?;

        // If the disk is not a multiple of the page size, the OS will fill the remaining part
        // of the page with zeroes. However, the anonymous mapping added below must start on a
        // page boundary, so round up the size before calculating the offset of the anon region.
        let disk_size = round_up_to_page_size(disk_size);

        if arena_size > disk_size {
            // Add an anonymous region with the same protection as the disk mapping if the arena
            // size was aligned.
            arena
                .add_anon_protection(disk_size, arena_size - disk_size, protection)
                .context("failed to reserve pmem padding")?;
        }
        arena
    };

    let mapping_address = resources
        .allocate_mmio(
            arena_size,
            Alloc::PmemDevice(index),
            format!("pmem_disk_image_{}", index),
            AllocOptions::new()
                .top_down(true)
                .prefetchable(true)
                // Linux kernel requires pmem namespaces to be 128 MiB aligned.
                .align(128 * 1024 * 1024), /* 128 MiB */
        )
        .context("failed to allocate memory for pmem device")?;

    let slot = vm
        .add_memory_region(
            GuestAddress(mapping_address),
            Box::new(arena),
            /* read_only = */ disk.read_only,
            /* log_dirty_pages = */ false,
        )
        .context("failed to add pmem device memory")?;

    let dev = virtio::Pmem::new(
        virtio::base_features(protection_type),
        fd,
        GuestAddress(mapping_address),
        slot,
        arena_size,
        Some(pmem_device_tube),
    )
    .context("failed to create pmem device")?;

    Ok(VirtioDeviceStub {
        dev: Box::new(dev) as Box<dyn VirtioDevice>,
        jail: simple_jail(jail_config, "pmem_device")?,
    })
}

pub fn create_iommu_device(
    protection_type: ProtectionType,
    jail_config: &Option<JailConfig>,
    iova_max_addr: u64,
    endpoints: BTreeMap<u32, Arc<Mutex<Box<dyn MemoryMapperTrait>>>>,
    hp_endpoints_ranges: Vec<RangeInclusive<u32>>,
    translate_response_senders: Option<BTreeMap<u32, Tube>>,
    translate_request_rx: Option<Tube>,
    iommu_device_tube: Tube,
) -> DeviceResult {
    let dev = virtio::Iommu::new(
        virtio::base_features(protection_type),
        endpoints,
        iova_max_addr,
        hp_endpoints_ranges,
        translate_response_senders,
        translate_request_rx,
        Some(iommu_device_tube),
    )
    .context("failed to create IOMMU device")?;

    Ok(VirtioDeviceStub {
        dev: Box::new(dev),
        jail: simple_jail(jail_config, "iommu_device")?,
    })
}

fn add_bind_mounts(param: &SerialParameters, jail: &mut Minijail) -> Result<(), minijail::Error> {
    if let Some(path) = &param.path {
        if let SerialType::SystemSerialType = param.type_ {
            if let Some(parent) = path.as_path().parent() {
                if parent.exists() {
                    info!("Bind mounting dir {}", parent.display());
                    jail.mount_bind(parent, parent, true)?;
                }
            }
        }
    }
    Ok(())
}

/// For creating console virtio devices.
impl VirtioDeviceBuilder for SerialParameters {
    const NAME: &'static str = "serial";

    fn create_virtio_device(
        &self,
        protection_type: ProtectionType,
    ) -> anyhow::Result<Box<dyn VirtioDevice>> {
        let mut keep_rds = Vec::new();
        let evt = Event::new().context("failed to create event")?;

        Ok(Box::new(
            self.create_serial_device::<AsyncConsole>(protection_type, &evt, &mut keep_rds)
                .context("failed to create console device")?,
        ))
    }

    fn create_vhost_user_device(
        &self,
        keep_rds: &mut Vec<RawDescriptor>,
    ) -> anyhow::Result<Box<dyn VhostUserDevice>> {
        Ok(Box::new(virtio::vhost::user::create_vu_console_device(
            self, keep_rds,
        )?))
    }

    fn create_jail(
        &self,
        jail_config: &Option<JailConfig>,
        jail_type: VirtioDeviceType,
    ) -> anyhow::Result<Option<Minijail>> {
        let jail = match simple_jail(jail_config, &jail_type.seccomp_policy_file("serial"))? {
            Some(mut jail) => {
                // Create a tmpfs in the device's root directory so that we can bind mount the
                // log socket directory into it.
                // The size=67108864 is size=64*1024*1024 or size=64MB.
                jail.mount_with_data(
                    Path::new("none"),
                    Path::new("/"),
                    "tmpfs",
                    (libc::MS_NODEV | libc::MS_NOEXEC | libc::MS_NOSUID) as usize,
                    "size=67108864",
                )?;
                add_current_user_to_jail(&mut jail)?;
                add_bind_mounts(self, &mut jail)
                    .context("failed to add bind mounts for console device")?;
                Some(jail)
            }
            None => None,
        };

        Ok(jail)
    }
}

#[cfg(feature = "audio")]
pub fn create_sound_device(
    path: &Path,
    protection_type: ProtectionType,
    jail_config: &Option<JailConfig>,
) -> DeviceResult {
    let dev = virtio::new_sound(path, virtio::base_features(protection_type))
        .context("failed to create sound device")?;

    Ok(VirtioDeviceStub {
        dev: Box::new(dev),
        jail: simple_jail(jail_config, "vios_audio_device")?,
    })
}

pub fn create_vfio_device(
    jail_config: &Option<JailConfig>,
    vm: &impl Vm,
    resources: &mut SystemAllocator,
    control_tubes: &mut Vec<TaggedControlTube>,
    vfio_path: &Path,
    hotplug: bool,
    hotplug_bus: Option<u8>,
    guest_address: Option<PciAddress>,
    coiommu_endpoints: Option<&mut Vec<u16>>,
    iommu_dev: IommuDevType,
    #[cfg(feature = "direct")] is_intel_lpss: bool,
) -> DeviceResult<(Box<VfioPciDevice>, Option<Minijail>, Option<VfioWrapper>)> {
    let vfio_container = VfioCommonSetup::vfio_get_container(iommu_dev, Some(vfio_path))
        .context("failed to get vfio container")?;

    // create MSI, MSI-X, and Mem request sockets for each vfio device
    let (vfio_host_tube_msi, vfio_device_tube_msi) =
        Tube::pair().context("failed to create tube")?;
    control_tubes.push(TaggedControlTube::VmIrq(vfio_host_tube_msi));

    let (vfio_host_tube_msix, vfio_device_tube_msix) =
        Tube::pair().context("failed to create tube")?;
    control_tubes.push(TaggedControlTube::VmIrq(vfio_host_tube_msix));

    let (vfio_host_tube_mem, vfio_device_tube_mem) =
        Tube::pair().context("failed to create tube")?;
    control_tubes.push(TaggedControlTube::VmMemory(vfio_host_tube_mem));

    let vfio_device_tube_vm = if hotplug {
        let (vfio_host_tube_vm, device_tube_vm) = Tube::pair().context("failed to create tube")?;
        control_tubes.push(TaggedControlTube::Vm(vfio_host_tube_vm));
        Some(device_tube_vm)
    } else {
        None
    };

    let vfio_device = VfioDevice::new_passthrough(
        &vfio_path,
        vm,
        vfio_container.clone(),
        iommu_dev != IommuDevType::NoIommu,
    )
    .context("failed to create vfio device")?;
    let mut vfio_pci_device = Box::new(VfioPciDevice::new(
        #[cfg(feature = "direct")]
        vfio_path,
        vfio_device,
        hotplug,
        hotplug_bus,
        guest_address,
        vfio_device_tube_msi,
        vfio_device_tube_msix,
        vfio_device_tube_mem,
        vfio_device_tube_vm,
        #[cfg(feature = "direct")]
        is_intel_lpss,
    )?);
    // early reservation for pass-through PCI devices.
    let endpoint_addr = vfio_pci_device
        .allocate_address(resources)
        .context("failed to allocate resources early for vfio pci dev")?;

    let viommu_mapper = match iommu_dev {
        IommuDevType::NoIommu => None,
        IommuDevType::VirtioIommu => {
            Some(VfioWrapper::new(vfio_container, vm.get_memory().clone()))
        }
        IommuDevType::CoIommu => {
            if let Some(endpoints) = coiommu_endpoints {
                endpoints.push(endpoint_addr.to_u32() as u16);
            } else {
                bail!("Missed coiommu_endpoints vector to store the endpoint addr");
            }
            None
        }
    };

    if hotplug {
        Ok((vfio_pci_device, None, viommu_mapper))
    } else {
        Ok((
            vfio_pci_device,
            simple_jail(jail_config, "vfio_device")?,
            viommu_mapper,
        ))
    }
}

pub fn create_vfio_platform_device(
    jail_config: &Option<JailConfig>,
    vm: &impl Vm,
    _resources: &mut SystemAllocator,
    control_tubes: &mut Vec<TaggedControlTube>,
    vfio_path: &Path,
    _endpoints: &mut BTreeMap<u32, Arc<Mutex<Box<dyn MemoryMapperTrait>>>>,
    iommu_dev: IommuDevType,
) -> DeviceResult<(VfioPlatformDevice, Option<Minijail>)> {
    let vfio_container = VfioCommonSetup::vfio_get_container(iommu_dev, Some(vfio_path))
        .context("Failed to create vfio device")?;

    let (vfio_host_tube_mem, vfio_device_tube_mem) =
        Tube::pair().context("failed to create tube")?;
    control_tubes.push(TaggedControlTube::VmMemory(vfio_host_tube_mem));

    let vfio_device = VfioDevice::new_passthrough(
        &vfio_path,
        vm,
        vfio_container,
        iommu_dev != IommuDevType::NoIommu,
    )
    .context("Failed to create vfio device")?;
    let vfio_plat_dev = VfioPlatformDevice::new(vfio_device, vfio_device_tube_mem);

    Ok((
        vfio_plat_dev,
        simple_jail(jail_config, "vfio_platform_device")?,
    ))
}

/// Setup for devices with virtio-iommu
pub fn setup_virtio_access_platform(
    resources: &mut SystemAllocator,
    iommu_attached_endpoints: &mut BTreeMap<u32, Arc<Mutex<Box<dyn MemoryMapperTrait>>>>,
    devices: &mut [(Box<dyn BusDeviceObj>, Option<Minijail>)],
) -> DeviceResult<(Option<BTreeMap<u32, Tube>>, Option<Tube>)> {
    let mut translate_response_senders: Option<
        BTreeMap<
            u32, // endpoint id
            Tube,
        >,
    > = None;
    let mut tube_pair: Option<(Tube, Tube)> = None;

    for dev in devices.iter_mut() {
        if let Some(pci_dev) = dev.0.as_pci_device_mut() {
            if pci_dev.supports_iommu() {
                let endpoint_id = pci_dev
                    .allocate_address(resources)
                    .context("failed to allocate resources for pci dev")?
                    .to_u32();
                let mapper: Arc<Mutex<Box<dyn MemoryMapperTrait>>> =
                    Arc::new(Mutex::new(Box::new(BasicMemoryMapper::new(u64::MAX))));
                let (request_tx, _request_rx) =
                    tube_pair.get_or_insert_with(|| Tube::pair().unwrap());
                let CreateIpcMapperRet {
                    mapper: ipc_mapper,
                    response_tx,
                } = create_ipc_mapper(
                    endpoint_id,
                    #[allow(deprecated)]
                    request_tx.try_clone()?,
                );
                translate_response_senders
                    .get_or_insert_with(BTreeMap::new)
                    .insert(endpoint_id, response_tx);
                iommu_attached_endpoints.insert(endpoint_id, mapper);
                pci_dev.set_iommu(ipc_mapper)?;
            }
        }
    }

    Ok((
        translate_response_senders,
        tube_pair.map(|(_request_tx, request_rx)| request_rx),
    ))
}
