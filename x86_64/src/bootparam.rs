// Copyright 2017 The ChromiumOS Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

/*
 * automatically generated by bindgen
 * From chromeos-linux v4.19
 * $ bindgen \
 *       --no-layout-tests --with-derive-default --no-doc-comments \
 *       --allowlist-type boot_params --allowlist-type setup_data \
 *       arch/x86/include/uapi/asm/bootparam.h
 */

// Editted to derive zerocopy traits, should migrate to bindgen when
// its command line support adding custom derives. Currently bindgen
// only support deriving custom traits with build.rs, and we don't want
// to run build.rs bindgen on kernel.

use zerocopy::AsBytes;
use zerocopy::FromBytes;
use zerocopy::FromZeroes;

#[repr(C)]
#[derive(Default)]
pub struct __IncompleteArrayField<T>(::std::marker::PhantomData<T>, [T; 0]);
impl<T> __IncompleteArrayField<T> {
    #[inline]
    pub fn new() -> Self {
        __IncompleteArrayField(::std::marker::PhantomData, [])
    }
    #[inline]
    pub unsafe fn as_ptr(&self) -> *const T {
        ::std::mem::transmute(self)
    }
    #[inline]
    pub unsafe fn as_mut_ptr(&mut self) -> *mut T {
        ::std::mem::transmute(self)
    }
    #[inline]
    pub unsafe fn as_slice(&self, len: usize) -> &[T] {
        ::std::slice::from_raw_parts(self.as_ptr(), len)
    }
    #[inline]
    pub unsafe fn as_mut_slice(&mut self, len: usize) -> &mut [T] {
        ::std::slice::from_raw_parts_mut(self.as_mut_ptr(), len)
    }
}
impl<T> ::std::fmt::Debug for __IncompleteArrayField<T> {
    fn fmt(&self, fmt: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        fmt.write_str("__IncompleteArrayField")
    }
}
impl<T> ::std::clone::Clone for __IncompleteArrayField<T> {
    #[inline]
    fn clone(&self) -> Self {
        Self::new()
    }
}
pub type __u8 = ::std::os::raw::c_uchar;
pub type __u16 = ::std::os::raw::c_ushort;
pub type __u32 = ::std::os::raw::c_uint;
pub type __u64 = ::std::os::raw::c_ulonglong;
#[repr(C, packed)]
#[derive(Debug, Default, Copy, Clone, FromZeroes, FromBytes, AsBytes)]
pub struct screen_info {
    pub orig_x: __u8,
    pub orig_y: __u8,
    pub ext_mem_k: __u16,
    pub orig_video_page: __u16,
    pub orig_video_mode: __u8,
    pub orig_video_cols: __u8,
    pub flags: __u8,
    pub unused2: __u8,
    pub orig_video_ega_bx: __u16,
    pub unused3: __u16,
    pub orig_video_lines: __u8,
    pub orig_video_isVGA: __u8,
    pub orig_video_points: __u16,
    pub lfb_width: __u16,
    pub lfb_height: __u16,
    pub lfb_depth: __u16,
    pub lfb_base: __u32,
    pub lfb_size: __u32,
    pub cl_magic: __u16,
    pub cl_offset: __u16,
    pub lfb_linelength: __u16,
    pub red_size: __u8,
    pub red_pos: __u8,
    pub green_size: __u8,
    pub green_pos: __u8,
    pub blue_size: __u8,
    pub blue_pos: __u8,
    pub rsvd_size: __u8,
    pub rsvd_pos: __u8,
    pub vesapm_seg: __u16,
    pub vesapm_off: __u16,
    pub pages: __u16,
    pub vesa_attributes: __u16,
    pub capabilities: __u32,
    pub ext_lfb_base: __u32,
    pub _reserved: [__u8; 2usize],
}
#[repr(C)]
#[derive(Debug, Default, Copy, Clone, FromZeroes, FromBytes, AsBytes)]
pub struct apm_bios_info {
    pub version: __u16,
    pub cseg: __u16,
    pub offset: __u32,
    pub cseg_16: __u16,
    pub dseg: __u16,
    pub flags: __u16,
    pub cseg_len: __u16,
    pub cseg_16_len: __u16,
    pub dseg_len: __u16,
}
#[repr(C, packed)]
#[derive(Copy, Clone, FromZeroes, FromBytes, AsBytes)]
pub struct edd_device_params {
    pub length: __u16,
    pub info_flags: __u16,
    pub num_default_cylinders: __u32,
    pub num_default_heads: __u32,
    pub sectors_per_track: __u32,
    pub number_of_sectors: __u64,
    pub bytes_per_sector: __u16,
    pub dpte_ptr: __u32,
    pub key: __u16,
    pub device_path_info_length: __u8,
    pub reserved2: __u8,
    pub reserved3: __u16,
    pub host_bus_type: [__u8; 4usize],
    pub interface_type: [__u8; 8usize],
    pub interface_path: edd_device_params__bindgen_ty_1,
    pub device_path: edd_device_params__bindgen_ty_2,
    pub reserved4: __u8,
    pub checksum: __u8,
}
#[repr(C)]
#[derive(Copy, Clone, FromZeroes, FromBytes, AsBytes)]
pub union edd_device_params__bindgen_ty_1 {
    pub isa: edd_device_params__bindgen_ty_1__bindgen_ty_1,
    pub pci: edd_device_params__bindgen_ty_1__bindgen_ty_2,
    pub ibnd: edd_device_params__bindgen_ty_1__bindgen_ty_3,
    pub xprs: edd_device_params__bindgen_ty_1__bindgen_ty_4,
    pub htpt: edd_device_params__bindgen_ty_1__bindgen_ty_5,
    pub unknown: edd_device_params__bindgen_ty_1__bindgen_ty_6,
    _bindgen_union_align: [u8; 8usize],
}
#[repr(C, packed)]
#[derive(Debug, Default, Copy, Clone, FromZeroes, FromBytes, AsBytes)]
pub struct edd_device_params__bindgen_ty_1__bindgen_ty_1 {
    pub base_address: __u16,
    pub reserved1: __u16,
    pub reserved2: __u32,
}
#[repr(C, packed)]
#[derive(Debug, Default, Copy, Clone, FromZeroes, FromBytes, AsBytes)]
pub struct edd_device_params__bindgen_ty_1__bindgen_ty_2 {
    pub bus: __u8,
    pub slot: __u8,
    pub function: __u8,
    pub channel: __u8,
    pub reserved: __u32,
}
#[repr(C, packed)]
#[derive(Debug, Default, Copy, Clone, FromZeroes, FromBytes, AsBytes)]
pub struct edd_device_params__bindgen_ty_1__bindgen_ty_3 {
    pub reserved: __u64,
}
#[repr(C, packed)]
#[derive(Debug, Default, Copy, Clone, FromZeroes, FromBytes, AsBytes)]
pub struct edd_device_params__bindgen_ty_1__bindgen_ty_4 {
    pub reserved: __u64,
}
#[repr(C, packed)]
#[derive(Debug, Default, Copy, Clone, FromZeroes, FromBytes, AsBytes)]
pub struct edd_device_params__bindgen_ty_1__bindgen_ty_5 {
    pub reserved: __u64,
}
#[repr(C, packed)]
#[derive(Debug, Default, Copy, Clone, FromZeroes, FromBytes, AsBytes)]
pub struct edd_device_params__bindgen_ty_1__bindgen_ty_6 {
    pub reserved: __u64,
}
impl Default for edd_device_params__bindgen_ty_1 {
    fn default() -> Self {
        unsafe { ::std::mem::zeroed() }
    }
}
#[repr(C)]
#[derive(Copy, Clone, FromZeroes, FromBytes, AsBytes)]
pub union edd_device_params__bindgen_ty_2 {
    pub ata: edd_device_params__bindgen_ty_2__bindgen_ty_1,
    pub atapi: edd_device_params__bindgen_ty_2__bindgen_ty_2,
    pub scsi: edd_device_params__bindgen_ty_2__bindgen_ty_3,
    pub usb: edd_device_params__bindgen_ty_2__bindgen_ty_4,
    pub i1394: edd_device_params__bindgen_ty_2__bindgen_ty_5,
    pub fibre: edd_device_params__bindgen_ty_2__bindgen_ty_6,
    pub i2o: edd_device_params__bindgen_ty_2__bindgen_ty_7,
    pub raid: edd_device_params__bindgen_ty_2__bindgen_ty_8,
    pub sata: edd_device_params__bindgen_ty_2__bindgen_ty_9,
    pub unknown: edd_device_params__bindgen_ty_2__bindgen_ty_10,
    _bindgen_union_align: [u8; 16usize],
}
#[repr(C, packed)]
#[derive(Debug, Default, Copy, Clone, FromZeroes, FromBytes, AsBytes)]
pub struct edd_device_params__bindgen_ty_2__bindgen_ty_1 {
    pub device: __u8,
    pub reserved1: __u8,
    pub reserved2: __u16,
    pub reserved3: __u32,
    pub reserved4: __u64,
}
#[repr(C, packed)]
#[derive(Debug, Default, Copy, Clone, FromZeroes, FromBytes, AsBytes)]
pub struct edd_device_params__bindgen_ty_2__bindgen_ty_2 {
    pub device: __u8,
    pub lun: __u8,
    pub reserved1: __u8,
    pub reserved2: __u8,
    pub reserved3: __u32,
    pub reserved4: __u64,
}
#[repr(C, packed)]
#[derive(Debug, Default, Copy, Clone, FromZeroes, FromBytes, AsBytes)]
pub struct edd_device_params__bindgen_ty_2__bindgen_ty_3 {
    pub id: __u16,
    pub lun: __u64,
    pub reserved1: __u16,
    pub reserved2: __u32,
}
#[repr(C, packed)]
#[derive(Debug, Default, Copy, Clone, FromZeroes, FromBytes, AsBytes)]
pub struct edd_device_params__bindgen_ty_2__bindgen_ty_4 {
    pub serial_number: __u64,
    pub reserved: __u64,
}
#[repr(C, packed)]
#[derive(Debug, Default, Copy, Clone, FromZeroes, FromBytes, AsBytes)]
pub struct edd_device_params__bindgen_ty_2__bindgen_ty_5 {
    pub eui: __u64,
    pub reserved: __u64,
}
#[repr(C, packed)]
#[derive(Debug, Default, Copy, Clone, FromZeroes, FromBytes, AsBytes)]
pub struct edd_device_params__bindgen_ty_2__bindgen_ty_6 {
    pub wwid: __u64,
    pub lun: __u64,
}
#[repr(C, packed)]
#[derive(Debug, Default, Copy, Clone, FromZeroes, FromBytes, AsBytes)]
pub struct edd_device_params__bindgen_ty_2__bindgen_ty_7 {
    pub identity_tag: __u64,
    pub reserved: __u64,
}
#[repr(C, packed)]
#[derive(Debug, Default, Copy, Clone, FromZeroes, FromBytes, AsBytes)]
pub struct edd_device_params__bindgen_ty_2__bindgen_ty_8 {
    pub array_number: __u32,
    pub reserved1: __u32,
    pub reserved2: __u64,
}
#[repr(C, packed)]
#[derive(Debug, Default, Copy, Clone, FromZeroes, FromBytes, AsBytes)]
pub struct edd_device_params__bindgen_ty_2__bindgen_ty_9 {
    pub device: __u8,
    pub reserved1: __u8,
    pub reserved2: __u16,
    pub reserved3: __u32,
    pub reserved4: __u64,
}
#[repr(C, packed)]
#[derive(Debug, Default, Copy, Clone, FromZeroes, FromBytes, AsBytes)]
pub struct edd_device_params__bindgen_ty_2__bindgen_ty_10 {
    pub reserved1: __u64,
    pub reserved2: __u64,
}
impl Default for edd_device_params__bindgen_ty_2 {
    fn default() -> Self {
        unsafe { ::std::mem::zeroed() }
    }
}
impl Default for edd_device_params {
    fn default() -> Self {
        unsafe { ::std::mem::zeroed() }
    }
}
#[repr(C, packed)]
#[derive(Copy, Clone, FromZeroes, FromBytes, AsBytes)]
pub struct edd_info {
    pub device: __u8,
    pub version: __u8,
    pub interface_support: __u16,
    pub legacy_max_cylinder: __u16,
    pub legacy_max_head: __u8,
    pub legacy_sectors_per_track: __u8,
    pub params: edd_device_params,
}
impl Default for edd_info {
    fn default() -> Self {
        unsafe { ::std::mem::zeroed() }
    }
}
#[repr(C)]
#[derive(Debug, Default, Copy, Clone, FromZeroes, FromBytes, AsBytes)]
pub struct ist_info {
    pub signature: __u32,
    pub command: __u32,
    pub event: __u32,
    pub perf_level: __u32,
}
#[repr(C)]
#[derive(Copy, Clone, AsBytes, FromZeroes, FromBytes)]
pub struct edid_info {
    pub dummy: [::std::os::raw::c_uchar; 128usize],
}
impl Default for edid_info {
    fn default() -> Self {
        unsafe { ::std::mem::zeroed() }
    }
}
#[repr(C)]
#[derive(Debug, Default)]
pub struct setup_data {
    pub next: __u64,
    pub type_: __u32,
    pub len: __u32,
    pub data: __IncompleteArrayField<__u8>,
}
#[repr(C, packed)]
#[derive(Debug, Default, Copy, Clone, FromZeroes, FromBytes, AsBytes)]
pub struct setup_header {
    pub setup_sects: __u8,
    pub root_flags: __u16,
    pub syssize: __u32,
    pub ram_size: __u16,
    pub vid_mode: __u16,
    pub root_dev: __u16,
    pub boot_flag: __u16,
    pub jump: __u16,
    pub header: __u32,
    pub version: __u16,
    pub realmode_swtch: __u32,
    pub start_sys_seg: __u16,
    pub kernel_version: __u16,
    pub type_of_loader: __u8,
    pub loadflags: __u8,
    pub setup_move_size: __u16,
    pub code32_start: __u32,
    pub ramdisk_image: __u32,
    pub ramdisk_size: __u32,
    pub bootsect_kludge: __u32,
    pub heap_end_ptr: __u16,
    pub ext_loader_ver: __u8,
    pub ext_loader_type: __u8,
    pub cmd_line_ptr: __u32,
    pub initrd_addr_max: __u32,
    pub kernel_alignment: __u32,
    pub relocatable_kernel: __u8,
    pub min_alignment: __u8,
    pub xloadflags: __u16,
    pub cmdline_size: __u32,
    pub hardware_subarch: __u32,
    pub hardware_subarch_data: __u64,
    pub payload_offset: __u32,
    pub payload_length: __u32,
    pub setup_data: __u64,
    pub pref_address: __u64,
    pub init_size: __u32,
    pub handover_offset: __u32,
}
#[repr(C)]
#[derive(Debug, Default, Copy, Clone, FromZeroes, FromBytes, AsBytes)]
pub struct sys_desc_table {
    pub length: __u16,
    pub table: [__u8; 14usize],
}
#[repr(C, packed)]
#[derive(Debug, Default, Copy, Clone, FromZeroes, FromBytes, AsBytes)]
pub struct olpc_ofw_header {
    pub ofw_magic: __u32,
    pub ofw_version: __u32,
    pub cif_handler: __u32,
    pub irq_desc_table: __u32,
}
#[repr(C)]
#[derive(Debug, Default, Copy, Clone, FromZeroes, FromBytes, AsBytes)]
pub struct efi_info {
    pub efi_loader_signature: __u32,
    pub efi_systab: __u32,
    pub efi_memdesc_size: __u32,
    pub efi_memdesc_version: __u32,
    pub efi_memmap: __u32,
    pub efi_memmap_size: __u32,
    pub efi_systab_hi: __u32,
    pub efi_memmap_hi: __u32,
}
#[repr(C, packed)]
#[derive(Debug, Default, Copy, Clone, FromZeroes, FromBytes, AsBytes)]
pub struct boot_e820_entry {
    pub addr: __u64,
    pub size: __u64,
    pub type_: __u32,
}
#[repr(C, packed)]
#[derive(Copy, Clone, FromZeroes, FromBytes, AsBytes)]
pub struct boot_params {
    pub screen_info: screen_info,
    pub apm_bios_info: apm_bios_info,
    pub _pad2: [__u8; 4usize],
    pub tboot_addr: __u64,
    pub ist_info: ist_info,
    pub acpi_rsdp_addr: __u64,
    pub _pad3: [__u8; 8usize],
    pub hd0_info: [__u8; 16usize],
    pub hd1_info: [__u8; 16usize],
    pub sys_desc_table: sys_desc_table,
    pub olpc_ofw_header: olpc_ofw_header,
    pub ext_ramdisk_image: __u32,
    pub ext_ramdisk_size: __u32,
    pub ext_cmd_line_ptr: __u32,
    pub _pad4: [__u8; 116usize],
    pub edid_info: edid_info,
    pub efi_info: efi_info,
    pub alt_mem_k: __u32,
    pub scratch: __u32,
    pub e820_entries: __u8,
    pub eddbuf_entries: __u8,
    pub edd_mbr_sig_buf_entries: __u8,
    pub kbd_status: __u8,
    pub secure_boot: __u8,
    pub _pad5: [__u8; 2usize],
    pub sentinel: __u8,
    pub _pad6: [__u8; 1usize],
    pub hdr: setup_header,
    pub _pad7: [__u8; 40usize],
    pub edd_mbr_sig_buffer: [__u32; 16usize],
    pub e820_table: [boot_e820_entry; 128usize],
    pub _pad8: [__u8; 48usize],
    pub eddbuf: [edd_info; 6usize],
    pub _pad9: [__u8; 276usize],
}
impl Default for boot_params {
    fn default() -> Self {
        unsafe { ::std::mem::zeroed() }
    }
}
