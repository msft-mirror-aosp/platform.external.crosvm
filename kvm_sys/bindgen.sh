#!/usr/bin/env bash
# Copyright 2022 The ChromiumOS Authors
# Use of this source code is governed by a BSD-style license that can be
# found in the LICENSE file.

# Regenerate kvm_sys bindgen bindings.

set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."

source tools/impl/bindgen-common.sh

KVM_EXTRAS="// Added by kvm_sys/bindgen.sh
use zerocopy::FromBytes;
use zerocopy::Immutable;
use zerocopy::IntoBytes;
use zerocopy::KnownLayout;

// TODO(b/388092267): Replace this with an upstream equivalent when available.
// The original index (236) used in the ChromeOS v6.6 kernel was reused upstream for another
// capability, so this may return incorrect information on some kernels.
pub const KVM_CAP_USER_CONFIGURE_NONCOHERENT_DMA_CROS: u32 = 236;

// TODO(qwandor): Update this once the pKVM patches are merged upstream with a stable capability ID.
pub const KVM_CAP_ARM_PROTECTED_VM: u32 = 0xffbadab1;
pub const KVM_CAP_ARM_PROTECTED_VM_FLAGS_SET_FW_IPA: u32 = 0;
pub const KVM_CAP_ARM_PROTECTED_VM_FLAGS_INFO: u32 = 1;
pub const KVM_VM_TYPE_ARM_PROTECTED: u32 = 0x80000000;
pub const KVM_X86_PKVM_PROTECTED_VM: u32 = 28;
pub const KVM_CAP_X86_PROTECTED_VM: u32 = 0xffbadab2;
pub const KVM_CAP_X86_PROTECTED_VM_FLAGS_SET_FW_GPA: u32 = 0;
pub const KVM_CAP_X86_PROTECTED_VM_FLAGS_INFO: u32 = 1;
pub const KVM_DEV_VFIO_PVIOMMU: u32 = 2;
pub const KVM_DEV_VFIO_PVIOMMU_ATTACH: u32 = 1;
#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
pub struct kvm_vfio_iommu_info {
    pub device_fd: i32,
    pub nr_sids: u32,
}
pub const KVM_DEV_VFIO_PVIOMMU_GET_INFO: u32 = 2;
#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
pub struct kvm_vfio_iommu_config {
    pub device_fd: i32,
    pub sid_idx: u32,
    pub vsid: u32,
}"

bindgen_generate \
    --raw-line "${KVM_EXTRAS}" \
    --blocklist-item='__kernel.*' \
    --blocklist-item='__BITS_PER_.*' \
    --blocklist-item='__FD_SETSIZE' \
    --blocklist-item='_?IOC.*' \
    --with-derive-custom "kvm_regs=FromBytes,Immutable,IntoBytes,KnownLayout" \
    --with-derive-custom "kvm_sregs=FromBytes,Immutable,IntoBytes,KnownLayout" \
    --with-derive-custom "kvm_fpu=FromBytes,Immutable,IntoBytes,KnownLayout" \
    --with-derive-custom "kvm_debugregs=FromBytes,Immutable,IntoBytes,KnownLayout" \
    --with-derive-custom "kvm_xcr=FromBytes,Immutable,IntoBytes,KnownLayout" \
    --with-derive-custom "kvm_xcrs=FromBytes,Immutable,IntoBytes,KnownLayout" \
    --with-derive-custom "kvm_lapic_state=FromBytes,Immutable,IntoBytes,KnownLayout" \
    --with-derive-custom "kvm_mp_state=FromBytes,Immutable,IntoBytes,KnownLayout" \
    --with-derive-custom "kvm_vcpu_events=FromBytes,Immutable,IntoBytes,KnownLayout" \
    --with-derive-custom "kvm_vcpu_events__bindgen_ty_1=FromBytes,Immutable,IntoBytes,KnownLayout" \
    --with-derive-custom "kvm_vcpu_events__bindgen_ty_2=FromBytes,Immutable,IntoBytes,KnownLayout" \
    --with-derive-custom "kvm_vcpu_events__bindgen_ty_3=FromBytes,Immutable,IntoBytes,KnownLayout" \
    --with-derive-custom "kvm_vcpu_events__bindgen_ty_4=FromBytes,Immutable,IntoBytes,KnownLayout" \
    --with-derive-custom "kvm_vcpu_events__bindgen_ty_5=FromBytes,Immutable,IntoBytes,KnownLayout" \
    --with-derive-custom "kvm_dtable=FromBytes,Immutable,IntoBytes,KnownLayout" \
    --with-derive-custom "kvm_segment=FromBytes,Immutable,IntoBytes,KnownLayout" \
    --with-derive-custom "kvm_pic_state=FromBytes,Immutable,IntoBytes,KnownLayout" \
    --with-derive-custom "kvm_pit_state2=FromBytes,Immutable,IntoBytes,KnownLayout" \
    --with-derive-custom "kvm_clock_data=FromBytes,Immutable,IntoBytes,KnownLayout" \
    --with-derive-custom "kvm_pit_channel_state=FromBytes,Immutable,IntoBytes,KnownLayout" \
    "${BINDGEN_LINUX_X86_HEADERS}/include/linux/kvm.h" \
    -- \
    -isystem "${BINDGEN_LINUX_X86_HEADERS}/include" \
    | replace_linux_int_types \
    > kvm_sys/src/x86/bindings.rs

bindgen_generate \
    --raw-line "${KVM_EXTRAS}" \
    --blocklist-item='__kernel.*' \
    --blocklist-item='__BITS_PER_.*' \
    --blocklist-item='__FD_SETSIZE' \
    --blocklist-item='_?IOC.*' \
    --with-derive-custom "kvm_regs=FromBytes,Immutable,IntoBytes,KnownLayout" \
    --with-derive-custom "kvm_sregs=FromBytes,Immutable,IntoBytes,KnownLayout" \
    --with-derive-custom "kvm_fpu=FromBytes,Immutable,IntoBytes,KnownLayout" \
    --with-derive-custom "kvm_vcpu_events=FromBytes,Immutable,IntoBytes,KnownLayout" \
    --with-derive-custom "kvm_vcpu_events__bindgen_ty_1=FromBytes,Immutable,IntoBytes,KnownLayout" \
    --with-derive-custom "kvm_mp_state=FromBytes,Immutable,IntoBytes,KnownLayout" \
    --with-derive-custom "user_fpsimd_state=FromBytes,Immutable,IntoBytes,KnownLayout" \
    --with-derive-custom "user_pt_regs=FromBytes,Immutable,IntoBytes,KnownLayout" \
    "${BINDGEN_LINUX_ARM64_HEADERS}/include/linux/kvm.h" \
    -- \
    -isystem "${BINDGEN_LINUX_ARM64_HEADERS}/include" \
    | replace_linux_int_types \
    > kvm_sys/src/aarch64/bindings.rs

bindgen_generate \
    --raw-line "${KVM_EXTRAS}" \
    --blocklist-item='__kernel.*' \
    --blocklist-item='__BITS_PER_.*' \
    --blocklist-item='__FD_SETSIZE' \
    --blocklist-item='_?IOC.*' \
    "${BINDGEN_LINUX_RISCV_HEADERS}/include/linux/kvm.h" \
    -- \
    -isystem "${BINDGEN_LINUX_RISCV_HEADERS}/include" \
    | replace_linux_int_types \
    > kvm_sys/src/riscv64/bindings.rs
