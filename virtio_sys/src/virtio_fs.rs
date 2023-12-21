/* automatically generated by tools/bindgen-all-the-things */

#![allow(clippy::missing_safety_doc)]
#![allow(clippy::upper_case_acronyms)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

// Added by virtio_sys/bindgen.sh
use data_model::Le32;
use zerocopy::AsBytes;
use zerocopy::FromBytes;
use zerocopy::FromZeroes;

pub const VIRTIO_FS_SHMCAP_ID_CACHE: u32 = 0;
#[repr(C, packed)]
#[derive(Debug, Copy, Clone, FromZeroes, FromBytes, AsBytes)]
pub struct virtio_fs_config {
    pub tag: [u8; 36usize],
    pub num_request_queues: Le32,
}
impl Default for virtio_fs_config {
    fn default() -> Self {
        let mut s = ::std::mem::MaybeUninit::<Self>::uninit();
        // SAFETY: Safe because s is aligned and is initialized in the block.
        unsafe {
            ::std::ptr::write_bytes(s.as_mut_ptr(), 0, 1);
            s.assume_init()
        }
    }
}
