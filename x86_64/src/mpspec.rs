// Copyright 2019 The ChromiumOS Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#![allow(warnings)]

/* automatically generated by rust-bindgen */

// manually added, needs to be included when this crate gets automated bindgen
use zerocopy::AsBytes;
use zerocopy::FromBytes;
use zerocopy::FromZeroes;

pub const MPC_SIGNATURE: &'static [u8; 5usize] = b"PCMP\x00";
pub const MP_PROCESSOR: ::std::os::raw::c_uint = 0;
pub const MP_BUS: ::std::os::raw::c_uint = 1;
pub const MP_IOAPIC: ::std::os::raw::c_uint = 2;
pub const MP_INTSRC: ::std::os::raw::c_uint = 3;
pub const MP_LINTSRC: ::std::os::raw::c_uint = 4;
pub const MP_TRANSLATION: ::std::os::raw::c_uint = 192;
pub const CPU_ENABLED: ::std::os::raw::c_uint = 1;
pub const CPU_BOOTPROCESSOR: ::std::os::raw::c_uint = 2;
pub const CPU_STEPPING_MASK: ::std::os::raw::c_uint = 15;
pub const CPU_MODEL_MASK: ::std::os::raw::c_uint = 240;
pub const CPU_FAMILY_MASK: ::std::os::raw::c_uint = 3840;
pub const BUSTYPE_EISA: &'static [u8; 5usize] = b"EISA\x00";
pub const BUSTYPE_ISA: &'static [u8; 4usize] = b"ISA\x00";
pub const BUSTYPE_INTERN: &'static [u8; 7usize] = b"INTERN\x00";
pub const BUSTYPE_MCA: &'static [u8; 4usize] = b"MCA\x00";
pub const BUSTYPE_VL: &'static [u8; 3usize] = b"VL\x00";
pub const BUSTYPE_PCI: &'static [u8; 4usize] = b"PCI\x00";
pub const BUSTYPE_PCMCIA: &'static [u8; 7usize] = b"PCMCIA\x00";
pub const BUSTYPE_CBUS: &'static [u8; 5usize] = b"CBUS\x00";
pub const BUSTYPE_CBUSII: &'static [u8; 7usize] = b"CBUSII\x00";
pub const BUSTYPE_FUTURE: &'static [u8; 7usize] = b"FUTURE\x00";
pub const BUSTYPE_MBI: &'static [u8; 4usize] = b"MBI\x00";
pub const BUSTYPE_MBII: &'static [u8; 5usize] = b"MBII\x00";
pub const BUSTYPE_MPI: &'static [u8; 4usize] = b"MPI\x00";
pub const BUSTYPE_MPSA: &'static [u8; 5usize] = b"MPSA\x00";
pub const BUSTYPE_NUBUS: &'static [u8; 6usize] = b"NUBUS\x00";
pub const BUSTYPE_TC: &'static [u8; 3usize] = b"TC\x00";
pub const BUSTYPE_VME: &'static [u8; 4usize] = b"VME\x00";
pub const BUSTYPE_XPRESS: &'static [u8; 7usize] = b"XPRESS\x00";
pub const MPC_APIC_USABLE: ::std::os::raw::c_uint = 1;
pub const MP_IRQDIR_DEFAULT: ::std::os::raw::c_uint = 0;
pub const MP_IRQDIR_HIGH: ::std::os::raw::c_uint = 1;
pub const MP_IRQDIR_LOW: ::std::os::raw::c_uint = 3;
pub const MP_LEVEL_TRIGGER: ::std::os::raw::c_uint = 0xc;
pub const MP_APIC_ALL: ::std::os::raw::c_uint = 255;
pub const MPC_OEM_SIGNATURE: &'static [u8; 5usize] = b"_OEM\x00";
#[repr(C)]
#[derive(Debug, Default, Copy, FromZeroes, FromBytes, AsBytes)]
pub struct mpf_intel {
    pub signature: [::std::os::raw::c_char; 4usize],
    pub physptr: ::std::os::raw::c_uint,
    pub length: ::std::os::raw::c_uchar,
    pub specification: ::std::os::raw::c_uchar,
    pub checksum: ::std::os::raw::c_uchar,
    pub feature1: ::std::os::raw::c_uchar,
    pub feature2: ::std::os::raw::c_uchar,
    pub feature3: ::std::os::raw::c_uchar,
    pub feature4: ::std::os::raw::c_uchar,
    pub feature5: ::std::os::raw::c_uchar,
}
#[test]
fn bindgen_test_layout_mpf_intel() {
    assert_eq!(
        ::std::mem::size_of::<mpf_intel>(),
        16usize,
        concat!("Size of: ", stringify!(mpf_intel))
    );
    assert_eq!(
        ::std::mem::align_of::<mpf_intel>(),
        4usize,
        concat!("Alignment of ", stringify!(mpf_intel))
    );
    assert_eq!(
        // SAFETY: trivially safe
        unsafe { &(*(0 as *const mpf_intel)).signature as *const _ as usize },
        0usize,
        concat!(
            "Alignment of field: ",
            stringify!(mpf_intel),
            "::",
            stringify!(signature)
        )
    );
    assert_eq!(
        // SAFETY: trivially safe
        unsafe { &(*(0 as *const mpf_intel)).physptr as *const _ as usize },
        4usize,
        concat!(
            "Alignment of field: ",
            stringify!(mpf_intel),
            "::",
            stringify!(physptr)
        )
    );
    assert_eq!(
        // SAFETY: trivially safe
        unsafe { &(*(0 as *const mpf_intel)).length as *const _ as usize },
        8usize,
        concat!(
            "Alignment of field: ",
            stringify!(mpf_intel),
            "::",
            stringify!(length)
        )
    );
    assert_eq!(
        // SAFETY: trivially safe
        unsafe { &(*(0 as *const mpf_intel)).specification as *const _ as usize },
        9usize,
        concat!(
            "Alignment of field: ",
            stringify!(mpf_intel),
            "::",
            stringify!(specification)
        )
    );
    assert_eq!(
        // SAFETY: trivially safe
        unsafe { &(*(0 as *const mpf_intel)).checksum as *const _ as usize },
        10usize,
        concat!(
            "Alignment of field: ",
            stringify!(mpf_intel),
            "::",
            stringify!(checksum)
        )
    );
    assert_eq!(
        // SAFETY: trivially safe
        unsafe { &(*(0 as *const mpf_intel)).feature1 as *const _ as usize },
        11usize,
        concat!(
            "Alignment of field: ",
            stringify!(mpf_intel),
            "::",
            stringify!(feature1)
        )
    );
    assert_eq!(
        // SAFETY: trivially safe
        unsafe { &(*(0 as *const mpf_intel)).feature2 as *const _ as usize },
        12usize,
        concat!(
            "Alignment of field: ",
            stringify!(mpf_intel),
            "::",
            stringify!(feature2)
        )
    );
    assert_eq!(
        // SAFETY: trivially safe
        unsafe { &(*(0 as *const mpf_intel)).feature3 as *const _ as usize },
        13usize,
        concat!(
            "Alignment of field: ",
            stringify!(mpf_intel),
            "::",
            stringify!(feature3)
        )
    );
    assert_eq!(
        // SAFETY: trivially safe
        unsafe { &(*(0 as *const mpf_intel)).feature4 as *const _ as usize },
        14usize,
        concat!(
            "Alignment of field: ",
            stringify!(mpf_intel),
            "::",
            stringify!(feature4)
        )
    );
    assert_eq!(
        // SAFETY: trivially safe
        unsafe { &(*(0 as *const mpf_intel)).feature5 as *const _ as usize },
        15usize,
        concat!(
            "Alignment of field: ",
            stringify!(mpf_intel),
            "::",
            stringify!(feature5)
        )
    );
}
impl Clone for mpf_intel {
    fn clone(&self) -> Self {
        *self
    }
}
#[repr(C)]
#[derive(Debug, Default, Copy, FromZeroes, FromBytes, AsBytes)]
pub struct mpc_table {
    pub signature: [::std::os::raw::c_char; 4usize],
    pub length: ::std::os::raw::c_ushort,
    pub spec: ::std::os::raw::c_char,
    pub checksum: ::std::os::raw::c_char,
    pub oem: [::std::os::raw::c_char; 8usize],
    pub productid: [::std::os::raw::c_char; 12usize],
    pub oemptr: ::std::os::raw::c_uint,
    pub oemsize: ::std::os::raw::c_ushort,
    pub oemcount: ::std::os::raw::c_ushort,
    pub lapic: ::std::os::raw::c_uint,
    pub reserved: ::std::os::raw::c_uint,
}
#[test]
fn bindgen_test_layout_mpc_table() {
    assert_eq!(
        ::std::mem::size_of::<mpc_table>(),
        44usize,
        concat!("Size of: ", stringify!(mpc_table))
    );
    assert_eq!(
        ::std::mem::align_of::<mpc_table>(),
        4usize,
        concat!("Alignment of ", stringify!(mpc_table))
    );
    assert_eq!(
        // SAFETY: trivially safe
        unsafe { &(*(0 as *const mpc_table)).signature as *const _ as usize },
        0usize,
        concat!(
            "Alignment of field: ",
            stringify!(mpc_table),
            "::",
            stringify!(signature)
        )
    );
    assert_eq!(
        // SAFETY: trivially safe
        unsafe { &(*(0 as *const mpc_table)).length as *const _ as usize },
        4usize,
        concat!(
            "Alignment of field: ",
            stringify!(mpc_table),
            "::",
            stringify!(length)
        )
    );
    assert_eq!(
        // SAFETY: trivially safe
        unsafe { &(*(0 as *const mpc_table)).spec as *const _ as usize },
        6usize,
        concat!(
            "Alignment of field: ",
            stringify!(mpc_table),
            "::",
            stringify!(spec)
        )
    );
    assert_eq!(
        // SAFETY: trivially safe
        unsafe { &(*(0 as *const mpc_table)).checksum as *const _ as usize },
        7usize,
        concat!(
            "Alignment of field: ",
            stringify!(mpc_table),
            "::",
            stringify!(checksum)
        )
    );
    assert_eq!(
        // SAFETY: trivially safe
        unsafe { &(*(0 as *const mpc_table)).oem as *const _ as usize },
        8usize,
        concat!(
            "Alignment of field: ",
            stringify!(mpc_table),
            "::",
            stringify!(oem)
        )
    );
    assert_eq!(
        // SAFETY: trivially safe
        unsafe { &(*(0 as *const mpc_table)).productid as *const _ as usize },
        16usize,
        concat!(
            "Alignment of field: ",
            stringify!(mpc_table),
            "::",
            stringify!(productid)
        )
    );
    assert_eq!(
        // SAFETY: trivially safe
        unsafe { &(*(0 as *const mpc_table)).oemptr as *const _ as usize },
        28usize,
        concat!(
            "Alignment of field: ",
            stringify!(mpc_table),
            "::",
            stringify!(oemptr)
        )
    );
    assert_eq!(
        // SAFETY: trivially safe
        unsafe { &(*(0 as *const mpc_table)).oemsize as *const _ as usize },
        32usize,
        concat!(
            "Alignment of field: ",
            stringify!(mpc_table),
            "::",
            stringify!(oemsize)
        )
    );
    assert_eq!(
        // SAFETY: trivially safe
        unsafe { &(*(0 as *const mpc_table)).oemcount as *const _ as usize },
        34usize,
        concat!(
            "Alignment of field: ",
            stringify!(mpc_table),
            "::",
            stringify!(oemcount)
        )
    );
    assert_eq!(
        // SAFETY: trivially safe
        unsafe { &(*(0 as *const mpc_table)).lapic as *const _ as usize },
        36usize,
        concat!(
            "Alignment of field: ",
            stringify!(mpc_table),
            "::",
            stringify!(lapic)
        )
    );
    assert_eq!(
        // SAFETY: trivially safe
        unsafe { &(*(0 as *const mpc_table)).reserved as *const _ as usize },
        40usize,
        concat!(
            "Alignment of field: ",
            stringify!(mpc_table),
            "::",
            stringify!(reserved)
        )
    );
}
impl Clone for mpc_table {
    fn clone(&self) -> Self {
        *self
    }
}
#[repr(C)]
#[derive(Debug, Default, Copy, FromZeroes, FromBytes, AsBytes)]
pub struct mpc_cpu {
    pub type_: ::std::os::raw::c_uchar,
    pub apicid: ::std::os::raw::c_uchar,
    pub apicver: ::std::os::raw::c_uchar,
    pub cpuflag: ::std::os::raw::c_uchar,
    pub cpufeature: ::std::os::raw::c_uint,
    pub featureflag: ::std::os::raw::c_uint,
    pub reserved: [::std::os::raw::c_uint; 2usize],
}
#[test]
fn bindgen_test_layout_mpc_cpu() {
    assert_eq!(
        ::std::mem::size_of::<mpc_cpu>(),
        20usize,
        concat!("Size of: ", stringify!(mpc_cpu))
    );
    assert_eq!(
        ::std::mem::align_of::<mpc_cpu>(),
        4usize,
        concat!("Alignment of ", stringify!(mpc_cpu))
    );
    assert_eq!(
        // SAFETY: trivially safe
        unsafe { &(*(0 as *const mpc_cpu)).type_ as *const _ as usize },
        0usize,
        concat!(
            "Alignment of field: ",
            stringify!(mpc_cpu),
            "::",
            stringify!(type_)
        )
    );
    assert_eq!(
        // SAFETY: trivially safe
        unsafe { &(*(0 as *const mpc_cpu)).apicid as *const _ as usize },
        1usize,
        concat!(
            "Alignment of field: ",
            stringify!(mpc_cpu),
            "::",
            stringify!(apicid)
        )
    );
    assert_eq!(
        // SAFETY: trivially safe
        unsafe { &(*(0 as *const mpc_cpu)).apicver as *const _ as usize },
        2usize,
        concat!(
            "Alignment of field: ",
            stringify!(mpc_cpu),
            "::",
            stringify!(apicver)
        )
    );
    assert_eq!(
        // SAFETY: trivially safe
        unsafe { &(*(0 as *const mpc_cpu)).cpuflag as *const _ as usize },
        3usize,
        concat!(
            "Alignment of field: ",
            stringify!(mpc_cpu),
            "::",
            stringify!(cpuflag)
        )
    );
    assert_eq!(
        // SAFETY: trivially safe
        unsafe { &(*(0 as *const mpc_cpu)).cpufeature as *const _ as usize },
        4usize,
        concat!(
            "Alignment of field: ",
            stringify!(mpc_cpu),
            "::",
            stringify!(cpufeature)
        )
    );
    assert_eq!(
        // SAFETY: trivially safe
        unsafe { &(*(0 as *const mpc_cpu)).featureflag as *const _ as usize },
        8usize,
        concat!(
            "Alignment of field: ",
            stringify!(mpc_cpu),
            "::",
            stringify!(featureflag)
        )
    );
    assert_eq!(
        // SAFETY: trivially safe
        unsafe { &(*(0 as *const mpc_cpu)).reserved as *const _ as usize },
        12usize,
        concat!(
            "Alignment of field: ",
            stringify!(mpc_cpu),
            "::",
            stringify!(reserved)
        )
    );
}
impl Clone for mpc_cpu {
    fn clone(&self) -> Self {
        *self
    }
}
#[repr(C)]
#[derive(Debug, Default, Copy, FromZeroes, FromBytes, AsBytes)]
pub struct mpc_bus {
    pub type_: ::std::os::raw::c_uchar,
    pub busid: ::std::os::raw::c_uchar,
    pub bustype: [::std::os::raw::c_uchar; 6usize],
}
#[test]
fn bindgen_test_layout_mpc_bus() {
    assert_eq!(
        ::std::mem::size_of::<mpc_bus>(),
        8usize,
        concat!("Size of: ", stringify!(mpc_bus))
    );
    assert_eq!(
        ::std::mem::align_of::<mpc_bus>(),
        1usize,
        concat!("Alignment of ", stringify!(mpc_bus))
    );
    assert_eq!(
        // SAFETY: trivially safe
        unsafe { &(*(0 as *const mpc_bus)).type_ as *const _ as usize },
        0usize,
        concat!(
            "Alignment of field: ",
            stringify!(mpc_bus),
            "::",
            stringify!(type_)
        )
    );
    assert_eq!(
        // SAFETY: trivially safe
        unsafe { &(*(0 as *const mpc_bus)).busid as *const _ as usize },
        1usize,
        concat!(
            "Alignment of field: ",
            stringify!(mpc_bus),
            "::",
            stringify!(busid)
        )
    );
    assert_eq!(
        // SAFETY: trivially safe
        unsafe { &(*(0 as *const mpc_bus)).bustype as *const _ as usize },
        2usize,
        concat!(
            "Alignment of field: ",
            stringify!(mpc_bus),
            "::",
            stringify!(bustype)
        )
    );
}
impl Clone for mpc_bus {
    fn clone(&self) -> Self {
        *self
    }
}
#[repr(C)]
#[derive(Debug, Default, Copy, FromZeroes, FromBytes, AsBytes)]
pub struct mpc_ioapic {
    pub type_: ::std::os::raw::c_uchar,
    pub apicid: ::std::os::raw::c_uchar,
    pub apicver: ::std::os::raw::c_uchar,
    pub flags: ::std::os::raw::c_uchar,
    pub apicaddr: ::std::os::raw::c_uint,
}
#[test]
fn bindgen_test_layout_mpc_ioapic() {
    assert_eq!(
        ::std::mem::size_of::<mpc_ioapic>(),
        8usize,
        concat!("Size of: ", stringify!(mpc_ioapic))
    );
    assert_eq!(
        ::std::mem::align_of::<mpc_ioapic>(),
        4usize,
        concat!("Alignment of ", stringify!(mpc_ioapic))
    );
    assert_eq!(
        // SAFETY: trivially safe
        unsafe { &(*(0 as *const mpc_ioapic)).type_ as *const _ as usize },
        0usize,
        concat!(
            "Alignment of field: ",
            stringify!(mpc_ioapic),
            "::",
            stringify!(type_)
        )
    );
    assert_eq!(
        // SAFETY: trivially safe
        unsafe { &(*(0 as *const mpc_ioapic)).apicid as *const _ as usize },
        1usize,
        concat!(
            "Alignment of field: ",
            stringify!(mpc_ioapic),
            "::",
            stringify!(apicid)
        )
    );
    assert_eq!(
        // SAFETY: trivially safe
        unsafe { &(*(0 as *const mpc_ioapic)).apicver as *const _ as usize },
        2usize,
        concat!(
            "Alignment of field: ",
            stringify!(mpc_ioapic),
            "::",
            stringify!(apicver)
        )
    );
    assert_eq!(
        // SAFETY: trivially safe
        unsafe { &(*(0 as *const mpc_ioapic)).flags as *const _ as usize },
        3usize,
        concat!(
            "Alignment of field: ",
            stringify!(mpc_ioapic),
            "::",
            stringify!(flags)
        )
    );
    assert_eq!(
        // SAFETY: trivially safe
        unsafe { &(*(0 as *const mpc_ioapic)).apicaddr as *const _ as usize },
        4usize,
        concat!(
            "Alignment of field: ",
            stringify!(mpc_ioapic),
            "::",
            stringify!(apicaddr)
        )
    );
}
impl Clone for mpc_ioapic {
    fn clone(&self) -> Self {
        *self
    }
}
#[repr(C)]
#[derive(Debug, Default, Copy, FromZeroes, FromBytes, AsBytes)]
pub struct mpc_intsrc {
    pub type_: ::std::os::raw::c_uchar,
    pub irqtype: ::std::os::raw::c_uchar,
    pub irqflag: ::std::os::raw::c_ushort,
    pub srcbus: ::std::os::raw::c_uchar,
    pub srcbusirq: ::std::os::raw::c_uchar,
    pub dstapic: ::std::os::raw::c_uchar,
    pub dstirq: ::std::os::raw::c_uchar,
}
#[test]
fn bindgen_test_layout_mpc_intsrc() {
    assert_eq!(
        ::std::mem::size_of::<mpc_intsrc>(),
        8usize,
        concat!("Size of: ", stringify!(mpc_intsrc))
    );
    assert_eq!(
        ::std::mem::align_of::<mpc_intsrc>(),
        2usize,
        concat!("Alignment of ", stringify!(mpc_intsrc))
    );
    assert_eq!(
        // SAFETY: trivially safe
        unsafe { &(*(0 as *const mpc_intsrc)).type_ as *const _ as usize },
        0usize,
        concat!(
            "Alignment of field: ",
            stringify!(mpc_intsrc),
            "::",
            stringify!(type_)
        )
    );
    assert_eq!(
        // SAFETY: trivially safe
        unsafe { &(*(0 as *const mpc_intsrc)).irqtype as *const _ as usize },
        1usize,
        concat!(
            "Alignment of field: ",
            stringify!(mpc_intsrc),
            "::",
            stringify!(irqtype)
        )
    );
    assert_eq!(
        // SAFETY: trivially safe
        unsafe { &(*(0 as *const mpc_intsrc)).irqflag as *const _ as usize },
        2usize,
        concat!(
            "Alignment of field: ",
            stringify!(mpc_intsrc),
            "::",
            stringify!(irqflag)
        )
    );
    assert_eq!(
        // SAFETY: trivially safe
        unsafe { &(*(0 as *const mpc_intsrc)).srcbus as *const _ as usize },
        4usize,
        concat!(
            "Alignment of field: ",
            stringify!(mpc_intsrc),
            "::",
            stringify!(srcbus)
        )
    );
    assert_eq!(
        // SAFETY: trivially safe
        unsafe { &(*(0 as *const mpc_intsrc)).srcbusirq as *const _ as usize },
        5usize,
        concat!(
            "Alignment of field: ",
            stringify!(mpc_intsrc),
            "::",
            stringify!(srcbusirq)
        )
    );
    assert_eq!(
        // SAFETY: trivially safe
        unsafe { &(*(0 as *const mpc_intsrc)).dstapic as *const _ as usize },
        6usize,
        concat!(
            "Alignment of field: ",
            stringify!(mpc_intsrc),
            "::",
            stringify!(dstapic)
        )
    );
    assert_eq!(
        // SAFETY: trivially safe
        unsafe { &(*(0 as *const mpc_intsrc)).dstirq as *const _ as usize },
        7usize,
        concat!(
            "Alignment of field: ",
            stringify!(mpc_intsrc),
            "::",
            stringify!(dstirq)
        )
    );
}
impl Clone for mpc_intsrc {
    fn clone(&self) -> Self {
        *self
    }
}
pub const mp_irq_source_types_mp_INT: mp_irq_source_types = 0;
pub const mp_irq_source_types_mp_NMI: mp_irq_source_types = 1;
pub const mp_irq_source_types_mp_SMI: mp_irq_source_types = 2;
pub const mp_irq_source_types_mp_ExtINT: mp_irq_source_types = 3;
pub type mp_irq_source_types = ::std::os::raw::c_uint;
#[repr(C)]
#[derive(Debug, Default, Copy, FromZeroes, FromBytes, AsBytes)]
pub struct mpc_lintsrc {
    pub type_: ::std::os::raw::c_uchar,
    pub irqtype: ::std::os::raw::c_uchar,
    pub irqflag: ::std::os::raw::c_ushort,
    pub srcbusid: ::std::os::raw::c_uchar,
    pub srcbusirq: ::std::os::raw::c_uchar,
    pub destapic: ::std::os::raw::c_uchar,
    pub destapiclint: ::std::os::raw::c_uchar,
}
#[test]
fn bindgen_test_layout_mpc_lintsrc() {
    assert_eq!(
        ::std::mem::size_of::<mpc_lintsrc>(),
        8usize,
        concat!("Size of: ", stringify!(mpc_lintsrc))
    );
    assert_eq!(
        ::std::mem::align_of::<mpc_lintsrc>(),
        2usize,
        concat!("Alignment of ", stringify!(mpc_lintsrc))
    );
    assert_eq!(
        // SAFETY: trivially safe
        unsafe { &(*(0 as *const mpc_lintsrc)).type_ as *const _ as usize },
        0usize,
        concat!(
            "Alignment of field: ",
            stringify!(mpc_lintsrc),
            "::",
            stringify!(type_)
        )
    );
    assert_eq!(
        // SAFETY: trivially safe
        unsafe { &(*(0 as *const mpc_lintsrc)).irqtype as *const _ as usize },
        1usize,
        concat!(
            "Alignment of field: ",
            stringify!(mpc_lintsrc),
            "::",
            stringify!(irqtype)
        )
    );
    assert_eq!(
        // SAFETY: trivially safe
        unsafe { &(*(0 as *const mpc_lintsrc)).irqflag as *const _ as usize },
        2usize,
        concat!(
            "Alignment of field: ",
            stringify!(mpc_lintsrc),
            "::",
            stringify!(irqflag)
        )
    );
    assert_eq!(
        // SAFETY: trivially safe
        unsafe { &(*(0 as *const mpc_lintsrc)).srcbusid as *const _ as usize },
        4usize,
        concat!(
            "Alignment of field: ",
            stringify!(mpc_lintsrc),
            "::",
            stringify!(srcbusid)
        )
    );
    assert_eq!(
        // SAFETY: trivially safe
        unsafe { &(*(0 as *const mpc_lintsrc)).srcbusirq as *const _ as usize },
        5usize,
        concat!(
            "Alignment of field: ",
            stringify!(mpc_lintsrc),
            "::",
            stringify!(srcbusirq)
        )
    );
    assert_eq!(
        // SAFETY: trivially safe
        unsafe { &(*(0 as *const mpc_lintsrc)).destapic as *const _ as usize },
        6usize,
        concat!(
            "Alignment of field: ",
            stringify!(mpc_lintsrc),
            "::",
            stringify!(destapic)
        )
    );
    assert_eq!(
        // SAFETY: trivially safe
        unsafe { &(*(0 as *const mpc_lintsrc)).destapiclint as *const _ as usize },
        7usize,
        concat!(
            "Alignment of field: ",
            stringify!(mpc_lintsrc),
            "::",
            stringify!(destapiclint)
        )
    );
}
impl Clone for mpc_lintsrc {
    fn clone(&self) -> Self {
        *self
    }
}
#[repr(C)]
#[derive(Debug, Default, Copy)]
pub struct mpc_oemtable {
    pub signature: [::std::os::raw::c_char; 4usize],
    pub length: ::std::os::raw::c_ushort,
    pub rev: ::std::os::raw::c_char,
    pub checksum: ::std::os::raw::c_char,
    pub mpc: [::std::os::raw::c_char; 8usize],
}
#[test]
fn bindgen_test_layout_mpc_oemtable() {
    assert_eq!(
        ::std::mem::size_of::<mpc_oemtable>(),
        16usize,
        concat!("Size of: ", stringify!(mpc_oemtable))
    );
    assert_eq!(
        ::std::mem::align_of::<mpc_oemtable>(),
        2usize,
        concat!("Alignment of ", stringify!(mpc_oemtable))
    );
    assert_eq!(
        // SAFETY: trivially safe
        unsafe { &(*(0 as *const mpc_oemtable)).signature as *const _ as usize },
        0usize,
        concat!(
            "Alignment of field: ",
            stringify!(mpc_oemtable),
            "::",
            stringify!(signature)
        )
    );
    assert_eq!(
        // SAFETY: trivially safe
        unsafe { &(*(0 as *const mpc_oemtable)).length as *const _ as usize },
        4usize,
        concat!(
            "Alignment of field: ",
            stringify!(mpc_oemtable),
            "::",
            stringify!(length)
        )
    );
    assert_eq!(
        // SAFETY: trivially safe
        unsafe { &(*(0 as *const mpc_oemtable)).rev as *const _ as usize },
        6usize,
        concat!(
            "Alignment of field: ",
            stringify!(mpc_oemtable),
            "::",
            stringify!(rev)
        )
    );
    assert_eq!(
        // SAFETY: trivially safe
        unsafe { &(*(0 as *const mpc_oemtable)).checksum as *const _ as usize },
        7usize,
        concat!(
            "Alignment of field: ",
            stringify!(mpc_oemtable),
            "::",
            stringify!(checksum)
        )
    );
    assert_eq!(
        // SAFETY: trivially safe
        unsafe { &(*(0 as *const mpc_oemtable)).mpc as *const _ as usize },
        8usize,
        concat!(
            "Alignment of field: ",
            stringify!(mpc_oemtable),
            "::",
            stringify!(mpc)
        )
    );
}
impl Clone for mpc_oemtable {
    fn clone(&self) -> Self {
        *self
    }
}
pub const mp_bustype_MP_BUS_ISA: mp_bustype = 1;
pub const mp_bustype_MP_BUS_EISA: mp_bustype = 2;
pub const mp_bustype_MP_BUS_PCI: mp_bustype = 3;
pub type mp_bustype = ::std::os::raw::c_uint;
