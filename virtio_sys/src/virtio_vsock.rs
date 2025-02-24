/* automatically generated by tools/bindgen-all-the-things */

#![allow(clippy::missing_safety_doc)]
#![allow(clippy::undocumented_unsafe_blocks)]
#![allow(clippy::upper_case_acronyms)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

// Added by virtio_sys/bindgen.sh
use data_model::Le16;
use data_model::Le32;
use data_model::Le64;
use zerocopy::FromBytes;
use zerocopy::Immutable;
use zerocopy::IntoBytes;
use zerocopy::KnownLayout;

pub const VIRTIO_VSOCK_F_SEQPACKET: u32 = 1;
#[repr(C, packed)]
#[derive(Debug, Default, Copy, Clone)]
pub struct virtio_vsock_config {
    pub guest_cid: Le64,
}
pub const virtio_vsock_event_id_VIRTIO_VSOCK_EVENT_TRANSPORT_RESET: virtio_vsock_event_id = 0;
pub type virtio_vsock_event_id = ::std::os::raw::c_uint;
#[repr(C, packed)]
#[derive(Debug, Default, Copy, Clone, FromBytes, Immutable, IntoBytes, KnownLayout)]
pub struct virtio_vsock_event {
    pub id: Le32,
}
#[repr(C, packed)]
#[derive(Debug, Default, Copy, Clone)]
pub struct virtio_vsock_hdr {
    pub src_cid: Le64,
    pub dst_cid: Le64,
    pub src_port: Le32,
    pub dst_port: Le32,
    pub len: Le32,
    pub type_: Le16,
    pub op: Le16,
    pub flags: Le32,
    pub buf_alloc: Le32,
    pub fwd_cnt: Le32,
}
pub const virtio_vsock_type_VIRTIO_VSOCK_TYPE_STREAM: virtio_vsock_type = 1;
pub const virtio_vsock_type_VIRTIO_VSOCK_TYPE_SEQPACKET: virtio_vsock_type = 2;
pub type virtio_vsock_type = ::std::os::raw::c_uint;
pub const virtio_vsock_op_VIRTIO_VSOCK_OP_INVALID: virtio_vsock_op = 0;
pub const virtio_vsock_op_VIRTIO_VSOCK_OP_REQUEST: virtio_vsock_op = 1;
pub const virtio_vsock_op_VIRTIO_VSOCK_OP_RESPONSE: virtio_vsock_op = 2;
pub const virtio_vsock_op_VIRTIO_VSOCK_OP_RST: virtio_vsock_op = 3;
pub const virtio_vsock_op_VIRTIO_VSOCK_OP_SHUTDOWN: virtio_vsock_op = 4;
pub const virtio_vsock_op_VIRTIO_VSOCK_OP_RW: virtio_vsock_op = 5;
pub const virtio_vsock_op_VIRTIO_VSOCK_OP_CREDIT_UPDATE: virtio_vsock_op = 6;
pub const virtio_vsock_op_VIRTIO_VSOCK_OP_CREDIT_REQUEST: virtio_vsock_op = 7;
pub type virtio_vsock_op = ::std::os::raw::c_uint;
pub const virtio_vsock_shutdown_VIRTIO_VSOCK_SHUTDOWN_RCV: virtio_vsock_shutdown = 1;
pub const virtio_vsock_shutdown_VIRTIO_VSOCK_SHUTDOWN_SEND: virtio_vsock_shutdown = 2;
pub type virtio_vsock_shutdown = ::std::os::raw::c_uint;
pub const virtio_vsock_rw_VIRTIO_VSOCK_SEQ_EOM: virtio_vsock_rw = 1;
pub const virtio_vsock_rw_VIRTIO_VSOCK_SEQ_EOR: virtio_vsock_rw = 2;
pub type virtio_vsock_rw = ::std::os::raw::c_uint;
