// Copyright 2020 The Chromium OS Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use std::convert::TryFrom;

use base::errno_result;
use base::error;
use base::ioctl_with_mut_ref;
use base::ioctl_with_ref;
use base::ioctl_with_val;
use base::warn;
use base::Error;
use base::MemoryMappingBuilder;
use base::Result;
use kvm_sys::*;
use libc::EINVAL;
use libc::ENOMEM;
use libc::ENOTSUP;
use libc::ENXIO;
use vm_memory::GuestAddress;

use super::Kvm;
use super::KvmCap;
use super::KvmVcpu;
use super::KvmVm;
use crate::ClockState;
use crate::DeviceKind;
use crate::Hypervisor;
use crate::IrqSourceChip;
use crate::ProtectionType;
use crate::PsciVersion;
use crate::VcpuAArch64;
use crate::VcpuExit;
use crate::VcpuFeature;
use crate::VcpuRegAArch64;
use crate::Vm;
use crate::VmAArch64;
use crate::VmCap;
use crate::PSCI_0_2;

/// Gives the ID for a register to be used with `set_one_reg`.
///
/// Pass the name of a field in `user_pt_regs` to get the corresponding register
/// ID, e.g. `arm64_core_reg!(pstate)`
///
/// To get ID for registers `x0`-`x31`, refer to the `regs` field along with the
/// register number, e.g. `arm64_core_reg!(regs, 5)` for `x5`. This is different
/// to work around `offset_of!(kvm_sys::user_pt_regs, regs[$x])` not working.
#[macro_export]
macro_rules! arm64_core_reg {
    ($reg: tt) => {{
        let off = (memoffset::offset_of!(::kvm_sys::user_pt_regs, $reg) / 4) as u64;
        ::kvm_sys::KVM_REG_ARM64
            | ::kvm_sys::KVM_REG_SIZE_U64
            | ::kvm_sys::KVM_REG_ARM_CORE as u64
            | off
    }};
    (regs, $x: literal) => {{
        let off = ((memoffset::offset_of!(::kvm_sys::user_pt_regs, regs)
            + ($x * ::std::mem::size_of::<u64>()))
            / 4) as u64;
        ::kvm_sys::KVM_REG_ARM64
            | ::kvm_sys::KVM_REG_SIZE_U64
            | ::kvm_sys::KVM_REG_ARM_CORE as u64
            | off
    }};
}

impl Kvm {
    // Compute the machine type, which should be the IPA range for the VM
    // Ideally, this would take a description of the memory map and return
    // the closest machine type for this VM. Here, we just return the maximum
    // the kernel support.
    pub fn get_vm_type(&self, protection_type: ProtectionType) -> Result<u32> {
        // Safe because we know self is a real kvm fd
        let ipa_size = match unsafe {
            ioctl_with_val(self, KVM_CHECK_EXTENSION(), KVM_CAP_ARM_VM_IPA_SIZE.into())
        } {
            // Not supported? Use 0 as the machine type, which implies 40bit IPA
            ret if ret < 0 => 0,
            ipa => ipa as u32,
        };
        let protection_flag = match protection_type {
            ProtectionType::Unprotected | ProtectionType::UnprotectedWithFirmware => 0,
            ProtectionType::Protected | ProtectionType::ProtectedWithoutFirmware => {
                KVM_VM_TYPE_ARM_PROTECTED
            }
        };
        // Use the lower 8 bits representing the IPA space as the machine type
        Ok((ipa_size & KVM_VM_TYPE_ARM_IPA_SIZE_MASK) | protection_flag)
    }

    /// Get the size of guest physical addresses (IPA) in bits.
    pub fn get_guest_phys_addr_bits(&self) -> u8 {
        // Safe because we know self is a real kvm fd
        match unsafe { ioctl_with_val(self, KVM_CHECK_EXTENSION(), KVM_CAP_ARM_VM_IPA_SIZE.into()) }
        {
            // Default physical address size is 40 bits if the extension is not supported.
            ret if ret <= 0 => 40,
            ipa => ipa as u8,
        }
    }
}

impl KvmVm {
    /// Checks if a particular `VmCap` is available, or returns None if arch-independent
    /// Vm.check_capability() should handle the check.
    pub fn check_capability_arch(&self, _c: VmCap) -> Option<bool> {
        None
    }

    /// Returns the params to pass to KVM_CREATE_DEVICE for a `kind` device on this arch, or None to
    /// let the arch-independent `KvmVm::create_device` handle it.
    pub fn get_device_params_arch(&self, kind: DeviceKind) -> Option<kvm_create_device> {
        match kind {
            DeviceKind::ArmVgicV2 => Some(kvm_create_device {
                type_: kvm_device_type_KVM_DEV_TYPE_ARM_VGIC_V2,
                fd: 0,
                flags: 0,
            }),
            DeviceKind::ArmVgicV3 => Some(kvm_create_device {
                type_: kvm_device_type_KVM_DEV_TYPE_ARM_VGIC_V3,
                fd: 0,
                flags: 0,
            }),
            _ => None,
        }
    }

    /// Arch-specific implementation of `Vm::get_pvclock`.  Always returns an error on AArch64.
    pub fn get_pvclock_arch(&self) -> Result<ClockState> {
        Err(Error::new(ENXIO))
    }

    /// Arch-specific implementation of `Vm::set_pvclock`.  Always returns an error on AArch64.
    pub fn set_pvclock_arch(&self, _state: &ClockState) -> Result<()> {
        Err(Error::new(ENXIO))
    }

    fn get_protected_vm_info(&self) -> Result<KvmProtectedVmInfo> {
        let mut info = KvmProtectedVmInfo {
            firmware_size: 0,
            reserved: [0; 7],
        };
        // Safe because we allocated the struct and we know the kernel won't write beyond the end of
        // the struct or keep a pointer to it.
        unsafe {
            self.enable_raw_capability(
                KvmCap::ArmProtectedVm,
                KVM_CAP_ARM_PROTECTED_VM_FLAGS_INFO,
                &[&mut info as *mut KvmProtectedVmInfo as u64, 0, 0, 0],
            )
        }?;
        Ok(info)
    }

    fn set_protected_vm_firmware_ipa(&self, fw_addr: GuestAddress) -> Result<()> {
        // Safe because none of the args are pointers.
        unsafe {
            self.enable_raw_capability(
                KvmCap::ArmProtectedVm,
                KVM_CAP_ARM_PROTECTED_VM_FLAGS_SET_FW_IPA,
                &[fw_addr.0, 0, 0, 0],
            )
        }
    }

    /// Enable userspace msr. This is not available on ARM, just succeed.
    pub fn enable_userspace_msr(&self) -> Result<()> {
        Ok(())
    }
}

#[repr(C)]
struct KvmProtectedVmInfo {
    firmware_size: u64,
    reserved: [u64; 7],
}

struct KvmVcpuRegister(u64);

impl VmAArch64 for KvmVm {
    fn get_hypervisor(&self) -> &dyn Hypervisor {
        &self.kvm
    }

    fn load_protected_vm_firmware(
        &mut self,
        fw_addr: GuestAddress,
        fw_max_size: u64,
    ) -> Result<()> {
        let info = self.get_protected_vm_info()?;
        if info.firmware_size == 0 {
            Err(Error::new(EINVAL))
        } else {
            if info.firmware_size > fw_max_size {
                return Err(Error::new(ENOMEM));
            }
            self.set_protected_vm_firmware_ipa(fw_addr)
        }
    }

    fn create_vcpu(&self, id: usize) -> Result<Box<dyn VcpuAArch64>> {
        // create_vcpu is declared separately in VmAArch64 and VmX86, so it can return VcpuAArch64
        // or VcpuX86.  But both use the same implementation in KvmVm::create_vcpu.
        Ok(Box::new(KvmVm::create_vcpu(self, id)?))
    }
}

impl KvmVcpu {
    /// Arch-specific implementation of `Vcpu::pvclock_ctrl`.  Always returns an error on AArch64.
    pub fn pvclock_ctrl_arch(&self) -> Result<()> {
        Err(Error::new(ENXIO))
    }

    /// Handles a `KVM_EXIT_SYSTEM_EVENT` with event type `KVM_SYSTEM_EVENT_RESET` with the given
    /// event flags and returns the appropriate `VcpuExit` value for the run loop to handle.
    ///
    /// `event_flags` should be one or more of the `KVM_SYSTEM_EVENT_RESET_FLAG_*` values defined by
    /// KVM.
    pub fn system_event_reset(&self, event_flags: u64) -> Result<VcpuExit> {
        if event_flags & KVM_SYSTEM_EVENT_RESET_FLAG_PSCI_RESET2 != 0 {
            // Read reset_type and cookie from x1 and x2.
            let reset_type = self.get_one_reg(VcpuRegAArch64::X(1))?;
            let cookie = self.get_one_reg(VcpuRegAArch64::X(2))?;
            warn!(
                "PSCI SYSTEM_RESET2 with reset_type={:#x}, cookie={:#x}",
                reset_type, cookie
            );
        }
        Ok(VcpuExit::SystemEventReset)
    }

    fn set_one_kvm_reg_u64(&self, kvm_reg_id: KvmVcpuRegister, data: u64) -> Result<()> {
        self.set_one_kvm_reg(kvm_reg_id, data.to_ne_bytes().as_slice())
    }

    fn set_one_kvm_reg(&self, kvm_reg_id: KvmVcpuRegister, data: &[u8]) -> Result<()> {
        let onereg = kvm_one_reg {
            id: kvm_reg_id.0,
            addr: (data.as_ptr() as usize)
                .try_into()
                .expect("can't represent usize as u64"),
        };
        // Safe because we allocated the struct and we know the kernel will read exactly the size of
        // the struct.
        let ret = unsafe { ioctl_with_ref(self, KVM_SET_ONE_REG(), &onereg) };
        if ret == 0 {
            Ok(())
        } else {
            errno_result()
        }
    }

    fn get_one_kvm_reg_u64(&self, kvm_reg_id: KvmVcpuRegister) -> Result<u64> {
        let mut bytes = 0u64.to_ne_bytes();
        self.get_one_kvm_reg(kvm_reg_id, bytes.as_mut_slice())?;
        Ok(u64::from_ne_bytes(bytes))
    }

    fn get_one_kvm_reg(&self, kvm_reg_id: KvmVcpuRegister, data: &mut [u8]) -> Result<()> {
        let onereg = kvm_one_reg {
            id: kvm_reg_id.0,
            addr: (data.as_mut_ptr() as usize)
                .try_into()
                .expect("can't represent usize as u64"),
        };

        // Safe because we allocated the struct and we know the kernel will read exactly the size of
        // the struct.
        let ret = unsafe { ioctl_with_ref(self, KVM_GET_ONE_REG(), &onereg) };
        if ret == 0 {
            Ok(())
        } else {
            errno_result()
        }
    }
}

impl From<VcpuRegAArch64> for KvmVcpuRegister {
    fn from(reg: VcpuRegAArch64) -> Self {
        match reg {
            VcpuRegAArch64::X(0) => Self(arm64_core_reg!(regs, 0)),
            VcpuRegAArch64::X(1) => Self(arm64_core_reg!(regs, 1)),
            VcpuRegAArch64::X(2) => Self(arm64_core_reg!(regs, 2)),
            VcpuRegAArch64::X(3) => Self(arm64_core_reg!(regs, 3)),
            VcpuRegAArch64::X(4) => Self(arm64_core_reg!(regs, 4)),
            VcpuRegAArch64::X(5) => Self(arm64_core_reg!(regs, 5)),
            VcpuRegAArch64::X(6) => Self(arm64_core_reg!(regs, 6)),
            VcpuRegAArch64::X(7) => Self(arm64_core_reg!(regs, 7)),
            VcpuRegAArch64::X(8) => Self(arm64_core_reg!(regs, 8)),
            VcpuRegAArch64::X(9) => Self(arm64_core_reg!(regs, 9)),
            VcpuRegAArch64::X(10) => Self(arm64_core_reg!(regs, 10)),
            VcpuRegAArch64::X(11) => Self(arm64_core_reg!(regs, 11)),
            VcpuRegAArch64::X(12) => Self(arm64_core_reg!(regs, 12)),
            VcpuRegAArch64::X(13) => Self(arm64_core_reg!(regs, 13)),
            VcpuRegAArch64::X(14) => Self(arm64_core_reg!(regs, 14)),
            VcpuRegAArch64::X(15) => Self(arm64_core_reg!(regs, 15)),
            VcpuRegAArch64::X(16) => Self(arm64_core_reg!(regs, 16)),
            VcpuRegAArch64::X(17) => Self(arm64_core_reg!(regs, 17)),
            VcpuRegAArch64::X(18) => Self(arm64_core_reg!(regs, 18)),
            VcpuRegAArch64::X(19) => Self(arm64_core_reg!(regs, 19)),
            VcpuRegAArch64::X(20) => Self(arm64_core_reg!(regs, 20)),
            VcpuRegAArch64::X(21) => Self(arm64_core_reg!(regs, 21)),
            VcpuRegAArch64::X(22) => Self(arm64_core_reg!(regs, 22)),
            VcpuRegAArch64::X(23) => Self(arm64_core_reg!(regs, 23)),
            VcpuRegAArch64::X(24) => Self(arm64_core_reg!(regs, 24)),
            VcpuRegAArch64::X(25) => Self(arm64_core_reg!(regs, 25)),
            VcpuRegAArch64::X(26) => Self(arm64_core_reg!(regs, 26)),
            VcpuRegAArch64::X(27) => Self(arm64_core_reg!(regs, 27)),
            VcpuRegAArch64::X(28) => Self(arm64_core_reg!(regs, 28)),
            VcpuRegAArch64::X(29) => Self(arm64_core_reg!(regs, 29)),
            VcpuRegAArch64::X(30) => Self(arm64_core_reg!(regs, 30)),
            VcpuRegAArch64::X(n) => unreachable!("invalid VcpuRegAArch64 index: {n}"),
            VcpuRegAArch64::Sp => Self(arm64_core_reg!(sp)),
            VcpuRegAArch64::Pc => Self(arm64_core_reg!(pc)),
            VcpuRegAArch64::Pstate => Self(arm64_core_reg!(pstate)),
        }
    }
}

impl VcpuAArch64 for KvmVcpu {
    fn init(&self, features: &[VcpuFeature]) -> Result<()> {
        let mut kvi = kvm_vcpu_init {
            target: KVM_ARM_TARGET_GENERIC_V8,
            features: [0; 7],
        };
        // Safe because we allocated the struct and we know the kernel will write exactly the size
        // of the struct.
        let ret = unsafe { ioctl_with_mut_ref(&self.vm, KVM_ARM_PREFERRED_TARGET(), &mut kvi) };
        if ret != 0 {
            return errno_result();
        }

        for f in features {
            let shift = match f {
                VcpuFeature::PsciV0_2 => KVM_ARM_VCPU_PSCI_0_2,
                VcpuFeature::PmuV3 => KVM_ARM_VCPU_PMU_V3,
                VcpuFeature::PowerOff => KVM_ARM_VCPU_POWER_OFF,
            };
            kvi.features[0] |= 1 << shift;
        }

        // Safe because we know self.vm is a real kvm fd
        let check_extension = |ext: u32| -> bool {
            unsafe { ioctl_with_val(&self.vm, KVM_CHECK_EXTENSION(), ext.into()) == 1 }
        };
        if check_extension(KVM_CAP_ARM_PTRAUTH_ADDRESS)
            && check_extension(KVM_CAP_ARM_PTRAUTH_GENERIC)
        {
            kvi.features[0] |= 1 << KVM_ARM_VCPU_PTRAUTH_ADDRESS;
            kvi.features[0] |= 1 << KVM_ARM_VCPU_PTRAUTH_GENERIC;
        }

        // Safe because we allocated the struct and we know the kernel will read exactly the size of
        // the struct.
        let ret = unsafe { ioctl_with_ref(self, KVM_ARM_VCPU_INIT(), &kvi) };
        if ret == 0 {
            Ok(())
        } else {
            errno_result()
        }
    }

    fn init_pmu(&self, irq: u64) -> Result<()> {
        let irq_addr = &irq as *const u64;

        // The in-kernel PMU virtualization is initialized by setting the irq
        // with KVM_ARM_VCPU_PMU_V3_IRQ and then by KVM_ARM_VCPU_PMU_V3_INIT.

        let irq_attr = kvm_device_attr {
            group: KVM_ARM_VCPU_PMU_V3_CTRL,
            attr: KVM_ARM_VCPU_PMU_V3_IRQ as u64,
            addr: irq_addr as u64,
            flags: 0,
        };
        // Safe because we allocated the struct and we know the kernel will read exactly the size of
        // the struct.
        let ret = unsafe { ioctl_with_ref(self, kvm_sys::KVM_HAS_DEVICE_ATTR(), &irq_attr) };
        if ret < 0 {
            return errno_result();
        }

        // Safe because we allocated the struct and we know the kernel will read exactly the size of
        // the struct.
        let ret = unsafe { ioctl_with_ref(self, kvm_sys::KVM_SET_DEVICE_ATTR(), &irq_attr) };
        if ret < 0 {
            return errno_result();
        }

        let init_attr = kvm_device_attr {
            group: KVM_ARM_VCPU_PMU_V3_CTRL,
            attr: KVM_ARM_VCPU_PMU_V3_INIT as u64,
            addr: 0,
            flags: 0,
        };
        // Safe because we allocated the struct and we know the kernel will read exactly the size of
        // the struct.
        let ret = unsafe { ioctl_with_ref(self, kvm_sys::KVM_SET_DEVICE_ATTR(), &init_attr) };
        if ret < 0 {
            return errno_result();
        }

        Ok(())
    }

    fn has_pvtime_support(&self) -> bool {
        // The in-kernel PV time structure is initialized by setting the base
        // address with KVM_ARM_VCPU_PVTIME_IPA
        let pvtime_attr = kvm_device_attr {
            group: KVM_ARM_VCPU_PVTIME_CTRL,
            attr: KVM_ARM_VCPU_PVTIME_IPA as u64,
            addr: 0,
            flags: 0,
        };
        // Safe because we allocated the struct and we know the kernel will read exactly the size of
        // the struct.
        let ret = unsafe { ioctl_with_ref(self, kvm_sys::KVM_HAS_DEVICE_ATTR(), &pvtime_attr) };
        if ret < 0 {
            return false;
        }

        return true;
    }

    fn init_pvtime(&self, pvtime_ipa: u64) -> Result<()> {
        let pvtime_ipa_addr = &pvtime_ipa as *const u64;

        // The in-kernel PV time structure is initialized by setting the base
        // address with KVM_ARM_VCPU_PVTIME_IPA
        let pvtime_attr = kvm_device_attr {
            group: KVM_ARM_VCPU_PVTIME_CTRL,
            attr: KVM_ARM_VCPU_PVTIME_IPA as u64,
            addr: pvtime_ipa_addr as u64,
            flags: 0,
        };

        // Safe because we allocated the struct and we know the kernel will read exactly the size of
        // the struct.
        let ret = unsafe { ioctl_with_ref(self, kvm_sys::KVM_SET_DEVICE_ATTR(), &pvtime_attr) };
        if ret < 0 {
            return errno_result();
        }

        Ok(())
    }

    fn set_one_reg(&self, reg_id: VcpuRegAArch64, data: u64) -> Result<()> {
        self.set_one_kvm_reg_u64(KvmVcpuRegister::from(reg_id), data)
    }

    fn get_one_reg(&self, reg_id: VcpuRegAArch64) -> Result<u64> {
        self.get_one_kvm_reg_u64(KvmVcpuRegister::from(reg_id))
    }

    fn get_psci_version(&self) -> Result<PsciVersion> {
        // The definition of KVM_REG_ARM_PSCI_VERSION is in arch/arm64/include/uapi/asm/kvm.h.
        const KVM_REG_ARM_PSCI_VERSION: u64 =
            KVM_REG_ARM64 | (KVM_REG_SIZE_U64 as u64) | (KVM_REG_ARM_FW as u64);
        const PSCI_VERSION: KvmVcpuRegister = KvmVcpuRegister(KVM_REG_ARM_PSCI_VERSION);

        let version = if let Ok(v) = self.get_one_kvm_reg_u64(PSCI_VERSION) {
            let v = u32::try_from(v).map_err(|_| Error::new(EINVAL))?;
            PsciVersion::try_from(v)?
        } else {
            // When `KVM_REG_ARM_PSCI_VERSION` is not supported, we can return PSCI 0.2, as vCPU
            // has been initialized with `KVM_ARM_VCPU_PSCI_0_2` successfully.
            PSCI_0_2
        };

        if version < PSCI_0_2 {
            // PSCI v0.1 isn't currently supported for guests
            Err(Error::new(ENOTSUP))
        } else {
            Ok(version)
        }
    }
}

// This function translates an IrqSrouceChip to the kvm u32 equivalent. It has a different
// implementation between x86_64 and aarch64 because the irqchip KVM constants are not defined on
// all architectures.
pub(super) fn chip_to_kvm_chip(chip: IrqSourceChip) -> u32 {
    match chip {
        // ARM does not have a constant for this, but the default routing
        // setup seems to set this to 0
        IrqSourceChip::Gic => 0,
        _ => {
            error!("Invalid IrqChipSource for ARM {:?}", chip);
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use vm_memory::GuestAddress;
    use vm_memory::GuestMemory;

    use super::super::Kvm;
    use super::*;
    use crate::IrqRoute;
    use crate::IrqSource;
    use crate::IrqSourceChip;

    #[test]
    fn set_gsi_routing() {
        let kvm = Kvm::new().unwrap();
        let gm = GuestMemory::new(&[(GuestAddress(0), 0x10000)]).unwrap();
        let vm = KvmVm::new(&kvm, gm, ProtectionType::Unprotected).unwrap();
        vm.create_irq_chip().unwrap();
        vm.set_gsi_routing(&[]).unwrap();
        vm.set_gsi_routing(&[IrqRoute {
            gsi: 1,
            source: IrqSource::Irqchip {
                chip: IrqSourceChip::Gic,
                pin: 3,
            },
        }])
        .unwrap();
        vm.set_gsi_routing(&[IrqRoute {
            gsi: 1,
            source: IrqSource::Msi {
                address: 0xf000000,
                data: 0xa0,
            },
        }])
        .unwrap();
        vm.set_gsi_routing(&[
            IrqRoute {
                gsi: 1,
                source: IrqSource::Irqchip {
                    chip: IrqSourceChip::Gic,
                    pin: 3,
                },
            },
            IrqRoute {
                gsi: 2,
                source: IrqSource::Msi {
                    address: 0xf000000,
                    data: 0xa0,
                },
            },
        ])
        .unwrap();
    }
}
