/* automatically generated by tools/bindgen-all-the-things */

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

#[repr(C)]
#[derive(Default)]
pub struct __IncompleteArrayField<T>(::std::marker::PhantomData<T>, [T; 0]);
impl<T> __IncompleteArrayField<T> {
    #[inline]
    pub const fn new() -> Self {
        __IncompleteArrayField(::std::marker::PhantomData, [])
    }
    #[inline]
    pub fn as_ptr(&self) -> *const T {
        self as *const _ as *const T
    }
    #[inline]
    pub fn as_mut_ptr(&mut self) -> *mut T {
        self as *mut _ as *mut T
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
pub const VRING_DESC_F_NEXT: u32 = 1;
pub const VRING_DESC_F_WRITE: u32 = 2;
pub const VRING_DESC_F_INDIRECT: u32 = 4;
pub const VRING_PACKED_DESC_F_AVAIL: u32 = 7;
pub const VRING_PACKED_DESC_F_USED: u32 = 15;
pub const VRING_USED_F_NO_NOTIFY: u32 = 1;
pub const VRING_AVAIL_F_NO_INTERRUPT: u32 = 1;
pub const VRING_PACKED_EVENT_FLAG_ENABLE: u32 = 0;
pub const VRING_PACKED_EVENT_FLAG_DISABLE: u32 = 1;
pub const VRING_PACKED_EVENT_FLAG_DESC: u32 = 2;
pub const VRING_PACKED_EVENT_F_WRAP_CTR: u32 = 15;
pub const VIRTIO_RING_F_INDIRECT_DESC: u32 = 28;
pub const VIRTIO_RING_F_EVENT_IDX: u32 = 29;
pub const VRING_AVAIL_ALIGN_SIZE: u32 = 2;
pub const VRING_USED_ALIGN_SIZE: u32 = 4;
pub const VRING_DESC_ALIGN_SIZE: u32 = 16;
pub type __le16 = u16;
pub type __le32 = u32;
pub type __le64 = u64;
pub type __virtio16 = u16;
pub type __virtio32 = u32;
pub type __virtio64 = u64;
#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
pub struct vring_desc {
    pub addr: __virtio64,
    pub len: __virtio32,
    pub flags: __virtio16,
    pub next: __virtio16,
}
#[repr(C)]
#[derive(Debug, Default)]
pub struct vring_avail {
    pub flags: __virtio16,
    pub idx: __virtio16,
    pub ring: __IncompleteArrayField<__virtio16>,
}
#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
pub struct vring_used_elem {
    pub id: __virtio32,
    pub len: __virtio32,
}
pub type vring_used_elem_t = vring_used_elem;
#[repr(C)]
#[derive(Debug, Default)]
pub struct vring_used {
    pub flags: __virtio16,
    pub idx: __virtio16,
    pub ring: __IncompleteArrayField<vring_used_elem_t>,
}
pub type vring_desc_t = vring_desc;
pub type vring_avail_t = vring_avail;
pub type vring_used_t = vring_used;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct vring {
    pub num: ::std::os::raw::c_uint,
    pub desc: *mut vring_desc_t,
    pub avail: *mut vring_avail_t,
    pub used: *mut vring_used_t,
}
impl Default for vring {
    fn default() -> Self {
        let mut s = ::std::mem::MaybeUninit::<Self>::uninit();
        unsafe {
            ::std::ptr::write_bytes(s.as_mut_ptr(), 0, 1);
            s.assume_init()
        }
    }
}
#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
pub struct vring_packed_desc_event {
    pub off_wrap: __le16,
    pub flags: __le16,
}
#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
pub struct vring_packed_desc {
    pub addr: __le64,
    pub len: __le32,
    pub id: __le16,
    pub flags: __le16,
}
