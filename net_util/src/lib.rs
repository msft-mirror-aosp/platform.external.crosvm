// Copyright 2017 The Chromium OS Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

pub mod sys;
use std::fmt::{self, Display};
use std::io::{Read, Write};
use std::net;
use std::num::ParseIntError;
use std::os::raw::*;
use std::str::FromStr;
pub use sys::TapT;

use base::Error as SysError;
use base::{AsRawDescriptor, RawDescriptor};
use remain::sorted;
use serde::{Deserialize, Serialize};
use thiserror::Error as ThisError;

#[sorted]
#[derive(ThisError, Debug)]
pub enum Error {
    /// Unable to clone tap interface.
    #[error("failed to clone tap interface: {0}")]
    CloneTap(SysError),
    /// Failed to create a socket.
    #[error("failed to create a socket: {0}")]
    CreateSocket(SysError),
    /// Unable to create tap interface.
    #[error("failed to create tap interface: {0}")]
    CreateTap(SysError),
    /// ioctl failed.
    #[error("ioctl failed: {0}")]
    IoctlError(SysError),
    /// Couldn't open /dev/net/tun.
    #[error("failed to open /dev/net/tun: {0}")]
    OpenTun(SysError),
}

pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    pub fn sys_error(&self) -> SysError {
        match *self {
            Error::CreateSocket(e) => e,
            Error::OpenTun(e) => e,
            Error::CreateTap(e) => e,
            Error::CloneTap(e) => e,
            Error::IoctlError(e) => e,
        }
    }
}

#[sorted]
#[derive(ThisError, Debug)]
pub enum MacAddressError {
    /// Invalid number of octets.
    #[error("invalid number of octets: {0}")]
    InvalidNumOctets(usize),
    /// Failed to parse octet.
    #[error("failed to parse octet: {0}")]
    ParseOctet(ParseIntError),
}

/// An Ethernet mac address. This struct is compatible with the C `struct sockaddr`.
#[repr(C)]
#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct MacAddress {
    family: net_sys::sa_family_t,
    addr: [u8; 6usize],
    __pad: [u8; 8usize],
}

impl MacAddress {
    pub fn octets(&self) -> [u8; 6usize] {
        self.addr
    }
}

impl FromStr for MacAddress {
    type Err = MacAddressError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let octets: Vec<&str> = s.split(':').collect();
        if octets.len() != 6usize {
            return Err(MacAddressError::InvalidNumOctets(octets.len()));
        }

        let mut result = MacAddress {
            family: net_sys::ARPHRD_ETHER,
            addr: [0; 6usize],
            __pad: [0; 8usize],
        };

        for (i, octet) in octets.iter().enumerate() {
            result.addr[i] = u8::from_str_radix(octet, 16).map_err(MacAddressError::ParseOctet)?;
        }

        Ok(result)
    }
}

impl Display for MacAddress {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
            self.addr[0], self.addr[1], self.addr[2], self.addr[3], self.addr[4], self.addr[5]
        )
    }
}

pub trait TapTCommon: Read + Write + AsRawDescriptor + Send + Sized {
    /// Create a new tap interface named `name`, or open it if it already exists with the same
    /// parameters.
    ///
    /// Set the `vnet_hdr` flag to true to allow offloading on this tap, which will add an extra 12
    /// byte virtio net header to incoming frames. Offloading cannot be used if `vnet_hdr` is false.
    /// Set 'multi_vq' to true, if tap have multi virt queue pairs
    fn new_with_name(name: &[u8], vnet_hdr: bool, multi_vq: bool) -> Result<Self>;

    /// Create a new tap interface. Set the `vnet_hdr` flag to true to allow offloading on this tap,
    /// which will add an extra 12 byte virtio net header to incoming frames. Offloading cannot
    /// be used if `vnet_hdr` is false. Set 'multi_vq' to true if tap has multi virt queue pairs.
    fn new(vnet_hdr: bool, multi_vq: bool) -> Result<Self>;

    /// Change the origin tap into multiqueue taps, this means create other taps based on the
    /// origin tap.
    fn into_mq_taps(self, vq_pairs: u16) -> Result<Vec<Self>>;

    /// Get the host-side IP address for the tap interface.
    fn ip_addr(&self) -> Result<net::Ipv4Addr>;

    /// Set the host-side IP address for the tap interface.
    fn set_ip_addr(&self, ip_addr: net::Ipv4Addr) -> Result<()>;

    /// Get the netmask for the tap interface's subnet.
    fn netmask(&self) -> Result<net::Ipv4Addr>;

    /// Set the netmask for the subnet that the tap interface will exist on.
    fn set_netmask(&self, netmask: net::Ipv4Addr) -> Result<()>;

    /// Get the MTU for the tap interface.
    fn mtu(&self) -> Result<u16>;

    /// Set the MTU for the tap interface.
    fn set_mtu(&self, mtu: u16) -> Result<()>;

    /// Get the mac address for the tap interface.
    fn mac_address(&self) -> Result<MacAddress>;

    /// Set the mac address for the tap interface.
    fn set_mac_address(&self, mac_addr: MacAddress) -> Result<()>;

    /// Set the offload flags for the tap interface.
    fn set_offload(&self, flags: c_uint) -> Result<()>;

    /// Enable the tap interface.
    fn enable(&self) -> Result<()>;

    /// Set the size of the vnet hdr.
    fn set_vnet_hdr_size(&self, size: c_int) -> Result<()>;

    fn get_ifreq(&self) -> net_sys::ifreq;

    /// Get the interface flags
    fn if_flags(&self) -> u32;

    /// Try to clone
    fn try_clone(&self) -> Result<Self>;

    /// Convert raw descriptor to
    unsafe fn from_raw_descriptor(descriptor: RawDescriptor) -> Result<Self>;
}
