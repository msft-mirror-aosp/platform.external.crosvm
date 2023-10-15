/* automatically generated by tools/bindgen-all-the-things */

#![allow(clippy::missing_safety_doc)]
#![allow(clippy::upper_case_acronyms)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

// Added by virtio_sys/bindgen.sh
use zerocopy::AsBytes;
use zerocopy::FromBytes;
use zerocopy::FromZeroes;

pub const VIRTIO_SCSI_CDB_DEFAULT_SIZE: u32 = 32;
pub const VIRTIO_SCSI_SENSE_DEFAULT_SIZE: u32 = 96;
pub const VIRTIO_SCSI_CDB_SIZE: u32 = 32;
pub const VIRTIO_SCSI_SENSE_SIZE: u32 = 96;
pub const VIRTIO_SCSI_F_INOUT: u32 = 0;
pub const VIRTIO_SCSI_F_HOTPLUG: u32 = 1;
pub const VIRTIO_SCSI_F_CHANGE: u32 = 2;
pub const VIRTIO_SCSI_F_T10_PI: u32 = 3;
pub const VIRTIO_SCSI_S_OK: u32 = 0;
pub const VIRTIO_SCSI_S_OVERRUN: u32 = 1;
pub const VIRTIO_SCSI_S_ABORTED: u32 = 2;
pub const VIRTIO_SCSI_S_BAD_TARGET: u32 = 3;
pub const VIRTIO_SCSI_S_RESET: u32 = 4;
pub const VIRTIO_SCSI_S_BUSY: u32 = 5;
pub const VIRTIO_SCSI_S_TRANSPORT_FAILURE: u32 = 6;
pub const VIRTIO_SCSI_S_TARGET_FAILURE: u32 = 7;
pub const VIRTIO_SCSI_S_NEXUS_FAILURE: u32 = 8;
pub const VIRTIO_SCSI_S_FAILURE: u32 = 9;
pub const VIRTIO_SCSI_S_FUNCTION_SUCCEEDED: u32 = 10;
pub const VIRTIO_SCSI_S_FUNCTION_REJECTED: u32 = 11;
pub const VIRTIO_SCSI_S_INCORRECT_LUN: u32 = 12;
pub const VIRTIO_SCSI_T_TMF: u32 = 0;
pub const VIRTIO_SCSI_T_AN_QUERY: u32 = 1;
pub const VIRTIO_SCSI_T_AN_SUBSCRIBE: u32 = 2;
pub const VIRTIO_SCSI_T_TMF_ABORT_TASK: u32 = 0;
pub const VIRTIO_SCSI_T_TMF_ABORT_TASK_SET: u32 = 1;
pub const VIRTIO_SCSI_T_TMF_CLEAR_ACA: u32 = 2;
pub const VIRTIO_SCSI_T_TMF_CLEAR_TASK_SET: u32 = 3;
pub const VIRTIO_SCSI_T_TMF_I_T_NEXUS_RESET: u32 = 4;
pub const VIRTIO_SCSI_T_TMF_LOGICAL_UNIT_RESET: u32 = 5;
pub const VIRTIO_SCSI_T_TMF_QUERY_TASK: u32 = 6;
pub const VIRTIO_SCSI_T_TMF_QUERY_TASK_SET: u32 = 7;
pub const VIRTIO_SCSI_T_EVENTS_MISSED: u32 = 2147483648;
pub const VIRTIO_SCSI_T_NO_EVENT: u32 = 0;
pub const VIRTIO_SCSI_T_TRANSPORT_RESET: u32 = 1;
pub const VIRTIO_SCSI_T_ASYNC_NOTIFY: u32 = 2;
pub const VIRTIO_SCSI_T_PARAM_CHANGE: u32 = 3;
pub const VIRTIO_SCSI_EVT_RESET_HARD: u32 = 0;
pub const VIRTIO_SCSI_EVT_RESET_RESCAN: u32 = 1;
pub const VIRTIO_SCSI_EVT_RESET_REMOVED: u32 = 2;
pub const VIRTIO_SCSI_S_SIMPLE: u32 = 0;
pub const VIRTIO_SCSI_S_ORDERED: u32 = 1;
pub const VIRTIO_SCSI_S_HEAD: u32 = 2;
pub const VIRTIO_SCSI_S_ACA: u32 = 3;
pub type __virtio16 = u16;
pub type __virtio32 = u32;
pub type __virtio64 = u64;
#[repr(C, packed)]
#[derive(Debug, Default, Copy, Clone, FromZeroes, FromBytes, AsBytes)]
pub struct virtio_scsi_cmd_req {
    pub lun: [u8; 8usize],
    pub tag: __virtio64,
    pub task_attr: u8,
    pub prio: u8,
    pub crn: u8,
    pub cdb: [u8; 32usize],
}
#[repr(C, packed)]
#[derive(Debug, Copy, Clone, FromZeroes, FromBytes, AsBytes)]
pub struct virtio_scsi_cmd_resp {
    pub sense_len: __virtio32,
    pub resid: __virtio32,
    pub status_qualifier: __virtio16,
    pub status: u8,
    pub response: u8,
    pub sense: [u8; 96usize],
}
impl Default for virtio_scsi_cmd_resp {
    fn default() -> Self {
        let mut s = ::std::mem::MaybeUninit::<Self>::uninit();
        unsafe {
            ::std::ptr::write_bytes(s.as_mut_ptr(), 0, 1);
            s.assume_init()
        }
    }
}
#[repr(C, packed)]
#[derive(Debug, Default, Copy, Clone)]
pub struct virtio_scsi_event {
    pub event: __virtio32,
    pub lun: [u8; 8usize],
    pub reason: __virtio32,
}
#[repr(C, packed)]
#[derive(Debug, Default, Copy, Clone, FromZeroes, FromBytes, AsBytes)]
pub struct virtio_scsi_config {
    pub num_queues: __virtio32,
    pub seg_max: __virtio32,
    pub max_sectors: __virtio32,
    pub cmd_per_lun: __virtio32,
    pub event_info_size: __virtio32,
    pub sense_size: __virtio32,
    pub cdb_size: __virtio32,
    pub max_channel: __virtio16,
    pub max_target: __virtio16,
    pub max_lun: __virtio32,
}
