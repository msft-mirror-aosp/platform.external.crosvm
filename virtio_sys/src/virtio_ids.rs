/* automatically generated by tools/bindgen-all-the-things */

#![allow(clippy::missing_safety_doc)]
#![allow(clippy::undocumented_unsafe_blocks)]
#![allow(clippy::upper_case_acronyms)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

//! This file defines virtio device IDs. IDs with large values (counting down
//! from 63) are nonstandard and not defined by the virtio specification.

// Added by virtio_sys/bindgen.sh - do not edit the generated file.
// TODO(b/236144983): Fix this id when an official virtio-id is assigned to this device.
pub const VIRTIO_ID_PVCLOCK: u32 = 61;
// TODO: Remove this once the ID is included in the Linux headers.
pub const VIRTIO_ID_MEDIA: u32 = 49;

pub const VIRTIO_ID_NET: u32 = 1;
pub const VIRTIO_ID_BLOCK: u32 = 2;
pub const VIRTIO_ID_CONSOLE: u32 = 3;
pub const VIRTIO_ID_RNG: u32 = 4;
pub const VIRTIO_ID_BALLOON: u32 = 5;
pub const VIRTIO_ID_IOMEM: u32 = 6;
pub const VIRTIO_ID_RPMSG: u32 = 7;
pub const VIRTIO_ID_SCSI: u32 = 8;
pub const VIRTIO_ID_9P: u32 = 9;
pub const VIRTIO_ID_MAC80211_WLAN: u32 = 10;
pub const VIRTIO_ID_RPROC_SERIAL: u32 = 11;
pub const VIRTIO_ID_CAIF: u32 = 12;
pub const VIRTIO_ID_MEMORY_BALLOON: u32 = 13;
pub const VIRTIO_ID_GPU: u32 = 16;
pub const VIRTIO_ID_CLOCK: u32 = 17;
pub const VIRTIO_ID_INPUT: u32 = 18;
pub const VIRTIO_ID_VSOCK: u32 = 19;
pub const VIRTIO_ID_CRYPTO: u32 = 20;
pub const VIRTIO_ID_SIGNAL_DIST: u32 = 21;
pub const VIRTIO_ID_PSTORE: u32 = 22;
pub const VIRTIO_ID_IOMMU: u32 = 23;
pub const VIRTIO_ID_MEM: u32 = 24;
pub const VIRTIO_ID_SOUND: u32 = 25;
pub const VIRTIO_ID_FS: u32 = 26;
pub const VIRTIO_ID_PMEM: u32 = 27;
pub const VIRTIO_ID_RPMB: u32 = 28;
pub const VIRTIO_ID_MAC80211_HWSIM: u32 = 29;
pub const VIRTIO_ID_VIDEO_ENCODER: u32 = 30;
pub const VIRTIO_ID_VIDEO_DECODER: u32 = 31;
pub const VIRTIO_ID_SCMI: u32 = 32;
pub const VIRTIO_ID_NITRO_SEC_MOD: u32 = 33;
pub const VIRTIO_ID_I2C_ADAPTER: u32 = 34;
pub const VIRTIO_ID_WATCHDOG: u32 = 35;
pub const VIRTIO_ID_CAN: u32 = 36;
pub const VIRTIO_ID_DMABUF: u32 = 37;
pub const VIRTIO_ID_PARAM_SERV: u32 = 38;
pub const VIRTIO_ID_AUDIO_POLICY: u32 = 39;
pub const VIRTIO_ID_BT: u32 = 40;
pub const VIRTIO_ID_GPIO: u32 = 41;
pub const VIRTIO_ID_WL: u32 = 63;
pub const VIRTIO_ID_TPM: u32 = 62;
