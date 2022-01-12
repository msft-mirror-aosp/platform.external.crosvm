// Copyright 2020 The Chromium OS Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#![allow(
    dead_code,
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals
)]

/*
automatically generated by rust-bindgen

generated with the command:
cd ${CHROMEOS_DIR}/src/platform2/ && \
bindgen arc/vm/libvda/libvda_decode.h \
  -o ../platform/crosvm/libvda/src/decode/bindings.rs \
  --raw-line 'pub use crate::bindings::*;' \
  --allowlist-function "initialize" \
  --allowlist-function "deinitialize" \
  --allowlist-function "get_vda_capabilities" \
  --allowlist-function "init_decode_session" \
  --allowlist-function "close_decode_session" \
  --allowlist-function "vda_.*" \
  --allowlist-type "vda_.*" \
  --blocklist-type "video_.*" \
  -- \
  -I .
*/

pub use crate::bindings::*;

pub type __int32_t = ::std::os::raw::c_int;
pub type __uint32_t = ::std::os::raw::c_uint;
pub const vda_impl_type_FAKE: vda_impl_type = 0;
pub const vda_impl_type_GAVDA: vda_impl_type = 1;
pub const vda_impl_type_GAVD: vda_impl_type = 2;
pub type vda_impl_type = u32;
pub use self::vda_impl_type as vda_impl_type_t;
pub const vda_result_SUCCESS: vda_result = 0;
pub const vda_result_ILLEGAL_STATE: vda_result = 1;
pub const vda_result_INVALID_ARGUMENT: vda_result = 2;
pub const vda_result_UNREADABLE_INPUT: vda_result = 3;
pub const vda_result_PLATFORM_FAILURE: vda_result = 4;
pub const vda_result_INSUFFICIENT_RESOURCES: vda_result = 5;
pub const vda_result_CANCELLED: vda_result = 6;
pub type vda_result = u32;
pub use self::vda_result as vda_result_t;
pub use self::video_codec_profile_t as vda_profile_t;
pub use self::video_pixel_format_t as vda_pixel_format_t;
pub const vda_event_type_UNKNOWN: vda_event_type = 0;
pub const vda_event_type_PROVIDE_PICTURE_BUFFERS: vda_event_type = 1;
pub const vda_event_type_PICTURE_READY: vda_event_type = 2;
pub const vda_event_type_NOTIFY_END_OF_BITSTREAM_BUFFER: vda_event_type = 3;
pub const vda_event_type_NOTIFY_ERROR: vda_event_type = 4;
pub const vda_event_type_RESET_RESPONSE: vda_event_type = 5;
pub const vda_event_type_FLUSH_RESPONSE: vda_event_type = 6;
pub type vda_event_type = u32;
pub use self::vda_event_type as vda_event_type_t;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct provide_picture_buffers_event_data {
    pub min_num_buffers: u32,
    pub width: i32,
    pub height: i32,
    pub visible_rect_left: i32,
    pub visible_rect_top: i32,
    pub visible_rect_right: i32,
    pub visible_rect_bottom: i32,
}
#[test]
fn bindgen_test_layout_provide_picture_buffers_event_data() {
    assert_eq!(
        ::std::mem::size_of::<provide_picture_buffers_event_data>(),
        28usize,
        concat!("Size of: ", stringify!(provide_picture_buffers_event_data))
    );
    assert_eq!(
        ::std::mem::align_of::<provide_picture_buffers_event_data>(),
        4usize,
        concat!(
            "Alignment of ",
            stringify!(provide_picture_buffers_event_data)
        )
    );
    assert_eq!(
        unsafe {
            &(*(::std::ptr::null::<provide_picture_buffers_event_data>())).min_num_buffers
                as *const _ as usize
        },
        0usize,
        concat!(
            "Offset of field: ",
            stringify!(provide_picture_buffers_event_data),
            "::",
            stringify!(min_num_buffers)
        )
    );
    assert_eq!(
        unsafe {
            &(*(::std::ptr::null::<provide_picture_buffers_event_data>())).width as *const _
                as usize
        },
        4usize,
        concat!(
            "Offset of field: ",
            stringify!(provide_picture_buffers_event_data),
            "::",
            stringify!(width)
        )
    );
    assert_eq!(
        unsafe {
            &(*(::std::ptr::null::<provide_picture_buffers_event_data>())).height as *const _
                as usize
        },
        8usize,
        concat!(
            "Offset of field: ",
            stringify!(provide_picture_buffers_event_data),
            "::",
            stringify!(height)
        )
    );
    assert_eq!(
        unsafe {
            &(*(::std::ptr::null::<provide_picture_buffers_event_data>())).visible_rect_left
                as *const _ as usize
        },
        12usize,
        concat!(
            "Offset of field: ",
            stringify!(provide_picture_buffers_event_data),
            "::",
            stringify!(visible_rect_left)
        )
    );
    assert_eq!(
        unsafe {
            &(*(::std::ptr::null::<provide_picture_buffers_event_data>())).visible_rect_top
                as *const _ as usize
        },
        16usize,
        concat!(
            "Offset of field: ",
            stringify!(provide_picture_buffers_event_data),
            "::",
            stringify!(visible_rect_top)
        )
    );
    assert_eq!(
        unsafe {
            &(*(::std::ptr::null::<provide_picture_buffers_event_data>())).visible_rect_right
                as *const _ as usize
        },
        20usize,
        concat!(
            "Offset of field: ",
            stringify!(provide_picture_buffers_event_data),
            "::",
            stringify!(visible_rect_right)
        )
    );
    assert_eq!(
        unsafe {
            &(*(::std::ptr::null::<provide_picture_buffers_event_data>())).visible_rect_bottom
                as *const _ as usize
        },
        24usize,
        concat!(
            "Offset of field: ",
            stringify!(provide_picture_buffers_event_data),
            "::",
            stringify!(visible_rect_bottom)
        )
    );
}
pub type provide_picture_buffers_event_data_t = provide_picture_buffers_event_data;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct picture_ready_event_data {
    pub picture_buffer_id: i32,
    pub bitstream_id: i32,
    pub crop_left: i32,
    pub crop_top: i32,
    pub crop_right: i32,
    pub crop_bottom: i32,
}
#[test]
fn bindgen_test_layout_picture_ready_event_data() {
    assert_eq!(
        ::std::mem::size_of::<picture_ready_event_data>(),
        24usize,
        concat!("Size of: ", stringify!(picture_ready_event_data))
    );
    assert_eq!(
        ::std::mem::align_of::<picture_ready_event_data>(),
        4usize,
        concat!("Alignment of ", stringify!(picture_ready_event_data))
    );
    assert_eq!(
        unsafe {
            &(*(::std::ptr::null::<picture_ready_event_data>())).picture_buffer_id as *const _
                as usize
        },
        0usize,
        concat!(
            "Offset of field: ",
            stringify!(picture_ready_event_data),
            "::",
            stringify!(picture_buffer_id)
        )
    );
    assert_eq!(
        unsafe {
            &(*(::std::ptr::null::<picture_ready_event_data>())).bitstream_id as *const _ as usize
        },
        4usize,
        concat!(
            "Offset of field: ",
            stringify!(picture_ready_event_data),
            "::",
            stringify!(bitstream_id)
        )
    );
    assert_eq!(
        unsafe {
            &(*(::std::ptr::null::<picture_ready_event_data>())).crop_left as *const _ as usize
        },
        8usize,
        concat!(
            "Offset of field: ",
            stringify!(picture_ready_event_data),
            "::",
            stringify!(crop_left)
        )
    );
    assert_eq!(
        unsafe {
            &(*(::std::ptr::null::<picture_ready_event_data>())).crop_top as *const _ as usize
        },
        12usize,
        concat!(
            "Offset of field: ",
            stringify!(picture_ready_event_data),
            "::",
            stringify!(crop_top)
        )
    );
    assert_eq!(
        unsafe {
            &(*(::std::ptr::null::<picture_ready_event_data>())).crop_right as *const _ as usize
        },
        16usize,
        concat!(
            "Offset of field: ",
            stringify!(picture_ready_event_data),
            "::",
            stringify!(crop_right)
        )
    );
    assert_eq!(
        unsafe {
            &(*(::std::ptr::null::<picture_ready_event_data>())).crop_bottom as *const _ as usize
        },
        20usize,
        concat!(
            "Offset of field: ",
            stringify!(picture_ready_event_data),
            "::",
            stringify!(crop_bottom)
        )
    );
}
pub type picture_ready_event_data_t = picture_ready_event_data;
#[repr(C)]
#[derive(Copy, Clone)]
pub union vda_event_data {
    pub provide_picture_buffers: provide_picture_buffers_event_data_t,
    pub picture_ready: picture_ready_event_data_t,
    pub bitstream_id: i32,
    pub result: vda_result_t,
    _bindgen_union_align: [u32; 7usize],
}
#[test]
fn bindgen_test_layout_vda_event_data() {
    assert_eq!(
        ::std::mem::size_of::<vda_event_data>(),
        28usize,
        concat!("Size of: ", stringify!(vda_event_data))
    );
    assert_eq!(
        ::std::mem::align_of::<vda_event_data>(),
        4usize,
        concat!("Alignment of ", stringify!(vda_event_data))
    );
    assert_eq!(
        unsafe {
            &(*(::std::ptr::null::<vda_event_data>())).provide_picture_buffers as *const _ as usize
        },
        0usize,
        concat!(
            "Offset of field: ",
            stringify!(vda_event_data),
            "::",
            stringify!(provide_picture_buffers)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<vda_event_data>())).picture_ready as *const _ as usize },
        0usize,
        concat!(
            "Offset of field: ",
            stringify!(vda_event_data),
            "::",
            stringify!(picture_ready)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<vda_event_data>())).bitstream_id as *const _ as usize },
        0usize,
        concat!(
            "Offset of field: ",
            stringify!(vda_event_data),
            "::",
            stringify!(bitstream_id)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<vda_event_data>())).result as *const _ as usize },
        0usize,
        concat!(
            "Offset of field: ",
            stringify!(vda_event_data),
            "::",
            stringify!(result)
        )
    );
}
pub type vda_event_data_t = vda_event_data;
#[repr(C)]
pub struct vda_input_format {
    pub profile: vda_profile_t,
    pub min_width: u32,
    pub min_height: u32,
    pub max_width: u32,
    pub max_height: u32,
}
#[test]
fn bindgen_test_layout_vda_input_format() {
    assert_eq!(
        ::std::mem::size_of::<vda_input_format>(),
        20usize,
        concat!("Size of: ", stringify!(vda_input_format))
    );
    assert_eq!(
        ::std::mem::align_of::<vda_input_format>(),
        4usize,
        concat!("Alignment of ", stringify!(vda_input_format))
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<vda_input_format>())).profile as *const _ as usize },
        0usize,
        concat!(
            "Offset of field: ",
            stringify!(vda_input_format),
            "::",
            stringify!(profile)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<vda_input_format>())).min_width as *const _ as usize },
        4usize,
        concat!(
            "Offset of field: ",
            stringify!(vda_input_format),
            "::",
            stringify!(min_width)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<vda_input_format>())).min_height as *const _ as usize },
        8usize,
        concat!(
            "Offset of field: ",
            stringify!(vda_input_format),
            "::",
            stringify!(min_height)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<vda_input_format>())).max_width as *const _ as usize },
        12usize,
        concat!(
            "Offset of field: ",
            stringify!(vda_input_format),
            "::",
            stringify!(max_width)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<vda_input_format>())).max_height as *const _ as usize },
        16usize,
        concat!(
            "Offset of field: ",
            stringify!(vda_input_format),
            "::",
            stringify!(max_height)
        )
    );
}
pub type vda_input_format_t = vda_input_format;
#[repr(C)]
#[derive(Copy, Clone)]
pub struct vda_event {
    pub event_type: vda_event_type_t,
    pub event_data: vda_event_data_t,
}
#[test]
fn bindgen_test_layout_vda_event() {
    assert_eq!(
        ::std::mem::size_of::<vda_event>(),
        32usize,
        concat!("Size of: ", stringify!(vda_event))
    );
    assert_eq!(
        ::std::mem::align_of::<vda_event>(),
        4usize,
        concat!("Alignment of ", stringify!(vda_event))
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<vda_event>())).event_type as *const _ as usize },
        0usize,
        concat!(
            "Offset of field: ",
            stringify!(vda_event),
            "::",
            stringify!(event_type)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<vda_event>())).event_data as *const _ as usize },
        4usize,
        concat!(
            "Offset of field: ",
            stringify!(vda_event),
            "::",
            stringify!(event_data)
        )
    );
}
pub type vda_event_t = vda_event;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct vda_capabilities {
    pub num_input_formats: usize,
    pub input_formats: *const vda_input_format_t,
    pub num_output_formats: usize,
    pub output_formats: *const vda_pixel_format_t,
}
#[test]
fn bindgen_test_layout_vda_capabilities() {
    assert_eq!(
        ::std::mem::size_of::<vda_capabilities>(),
        32usize,
        concat!("Size of: ", stringify!(vda_capabilities))
    );
    assert_eq!(
        ::std::mem::align_of::<vda_capabilities>(),
        8usize,
        concat!("Alignment of ", stringify!(vda_capabilities))
    );
    assert_eq!(
        unsafe {
            &(*(::std::ptr::null::<vda_capabilities>())).num_input_formats as *const _ as usize
        },
        0usize,
        concat!(
            "Offset of field: ",
            stringify!(vda_capabilities),
            "::",
            stringify!(num_input_formats)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<vda_capabilities>())).input_formats as *const _ as usize },
        8usize,
        concat!(
            "Offset of field: ",
            stringify!(vda_capabilities),
            "::",
            stringify!(input_formats)
        )
    );
    assert_eq!(
        unsafe {
            &(*(::std::ptr::null::<vda_capabilities>())).num_output_formats as *const _ as usize
        },
        16usize,
        concat!(
            "Offset of field: ",
            stringify!(vda_capabilities),
            "::",
            stringify!(num_output_formats)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<vda_capabilities>())).output_formats as *const _ as usize },
        24usize,
        concat!(
            "Offset of field: ",
            stringify!(vda_capabilities),
            "::",
            stringify!(output_formats)
        )
    );
}
pub type vda_capabilities_t = vda_capabilities;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct vda_session_info {
    pub ctx: *mut ::std::os::raw::c_void,
    pub event_pipe_fd: ::std::os::raw::c_int,
}
#[test]
fn bindgen_test_layout_vda_session_info() {
    assert_eq!(
        ::std::mem::size_of::<vda_session_info>(),
        16usize,
        concat!("Size of: ", stringify!(vda_session_info))
    );
    assert_eq!(
        ::std::mem::align_of::<vda_session_info>(),
        8usize,
        concat!("Alignment of ", stringify!(vda_session_info))
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<vda_session_info>())).ctx as *const _ as usize },
        0usize,
        concat!(
            "Offset of field: ",
            stringify!(vda_session_info),
            "::",
            stringify!(ctx)
        )
    );
    assert_eq!(
        unsafe { &(*(::std::ptr::null::<vda_session_info>())).event_pipe_fd as *const _ as usize },
        8usize,
        concat!(
            "Offset of field: ",
            stringify!(vda_session_info),
            "::",
            stringify!(event_pipe_fd)
        )
    );
}
pub type vda_session_info_t = vda_session_info;
extern "C" {
    pub fn initialize(impl_type: vda_impl_type_t) -> *mut ::std::os::raw::c_void;
}
extern "C" {
    pub fn deinitialize(impl_: *mut ::std::os::raw::c_void);
}
extern "C" {
    pub fn get_vda_capabilities(impl_: *mut ::std::os::raw::c_void) -> *const vda_capabilities_t;
}
extern "C" {
    pub fn init_decode_session(
        impl_: *mut ::std::os::raw::c_void,
        profile: vda_profile_t,
    ) -> *mut vda_session_info_t;
}
extern "C" {
    pub fn close_decode_session(
        impl_: *mut ::std::os::raw::c_void,
        session_info: *mut vda_session_info_t,
    );
}
extern "C" {
    pub fn vda_decode(
        ctx: *mut ::std::os::raw::c_void,
        bitstream_id: i32,
        fd: ::std::os::raw::c_int,
        offset: u32,
        bytes_used: u32,
    ) -> vda_result_t;
}
extern "C" {
    pub fn vda_set_output_buffer_count(
        ctx: *mut ::std::os::raw::c_void,
        num_output_buffers: usize,
    ) -> vda_result_t;
}
extern "C" {
    pub fn vda_use_output_buffer(
        ctx: *mut ::std::os::raw::c_void,
        picture_buffer_id: i32,
        format: vda_pixel_format_t,
        fd: ::std::os::raw::c_int,
        num_planes: usize,
        planes: *mut video_frame_plane_t,
        modifier: u64,
    ) -> vda_result_t;
}
extern "C" {
    pub fn vda_reuse_output_buffer(
        ctx: *mut ::std::os::raw::c_void,
        picture_buffer_id: i32,
    ) -> vda_result_t;
}
extern "C" {
    pub fn vda_flush(ctx: *mut ::std::os::raw::c_void) -> vda_result_t;
}
extern "C" {
    pub fn vda_reset(ctx: *mut ::std::os::raw::c_void) -> vda_result_t;
}
