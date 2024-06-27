// Copyright 2020 The ChromiumOS Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

//! This file was generated by the following commands and modified manually.
//!
//! ```shell
//! $ bindgen virtio_video.h              \
//!     --allowlist-type "virtio_video.*" \
//!     --allowlist-var "VIRTIO_VIDEO_.*" \
//!     --with-derive-default            \
//!     --no-layout-tests                \
//!     --no-prepend-enum-name > protocol.rs
//! $ sed -i "s/__u/u/g" protocol.rs
//! $ sed -i "s/__le/Le/g" protocol.rs
//! ```
//!
//! The main points of the manual modifications are as follows:
//! * Removed `hdr` from each command struct so that we can read the header and a command body
//!   separately. (cf. [related discussion](https://markmail.org/message/tr5g6axqq2zzq64y))
//! * Derive implementations of AsBytes and FromBytes for each struct as needed.
//! * Added GET_PARAMS_EXT and SET_PARAMS_EXT to allow querying and changing the resource type
//!   dynamically.
//! * Moved some definitions such as virtio_video_config to device_constants to make them visible to
//!   vhost-user modules, and also pub-use them.

#![allow(dead_code, non_snake_case, non_camel_case_types)]

use data_model::Le32;
use data_model::Le64;
use zerocopy::AsBytes;
use zerocopy::FromBytes;
use zerocopy::FromZeroes;

pub const VIRTIO_VIDEO_MAX_PLANES: u32 = 8;
pub const VIRTIO_VIDEO_FORMAT_RAW_MIN: virtio_video_format = 1;
pub const VIRTIO_VIDEO_FORMAT_ARGB8888: virtio_video_format = 1;
pub const VIRTIO_VIDEO_FORMAT_BGRA8888: virtio_video_format = 2;
pub const VIRTIO_VIDEO_FORMAT_NV12: virtio_video_format = 3;
pub const VIRTIO_VIDEO_FORMAT_YUV420: virtio_video_format = 4;
pub const VIRTIO_VIDEO_FORMAT_YVU420: virtio_video_format = 5;
pub const VIRTIO_VIDEO_FORMAT_RAW_MAX: virtio_video_format = 5;
pub const VIRTIO_VIDEO_FORMAT_CODED_MIN: virtio_video_format = 4096;
pub const VIRTIO_VIDEO_FORMAT_MPEG2: virtio_video_format = 4096;
pub const VIRTIO_VIDEO_FORMAT_MPEG4: virtio_video_format = 4097;
pub const VIRTIO_VIDEO_FORMAT_H264: virtio_video_format = 4098;
pub const VIRTIO_VIDEO_FORMAT_HEVC: virtio_video_format = 4099;
pub const VIRTIO_VIDEO_FORMAT_VP8: virtio_video_format = 4100;
pub const VIRTIO_VIDEO_FORMAT_VP9: virtio_video_format = 4101;
pub const VIRTIO_VIDEO_FORMAT_CODED_MAX: virtio_video_format = 4101;
pub type virtio_video_format = u32;
pub const VIRTIO_VIDEO_PROFILE_H264_MIN: virtio_video_profile = 256;
pub const VIRTIO_VIDEO_PROFILE_H264_BASELINE: virtio_video_profile = 256;
pub const VIRTIO_VIDEO_PROFILE_H264_MAIN: virtio_video_profile = 257;
pub const VIRTIO_VIDEO_PROFILE_H264_EXTENDED: virtio_video_profile = 258;
pub const VIRTIO_VIDEO_PROFILE_H264_HIGH: virtio_video_profile = 259;
pub const VIRTIO_VIDEO_PROFILE_H264_HIGH10PROFILE: virtio_video_profile = 260;
pub const VIRTIO_VIDEO_PROFILE_H264_HIGH422PROFILE: virtio_video_profile = 261;
pub const VIRTIO_VIDEO_PROFILE_H264_HIGH444PREDICTIVEPROFILE: virtio_video_profile = 262;
pub const VIRTIO_VIDEO_PROFILE_H264_SCALABLEBASELINE: virtio_video_profile = 263;
pub const VIRTIO_VIDEO_PROFILE_H264_SCALABLEHIGH: virtio_video_profile = 264;
pub const VIRTIO_VIDEO_PROFILE_H264_STEREOHIGH: virtio_video_profile = 265;
pub const VIRTIO_VIDEO_PROFILE_H264_MULTIVIEWHIGH: virtio_video_profile = 266;
pub const VIRTIO_VIDEO_PROFILE_H264_MAX: virtio_video_profile = 266;
pub const VIRTIO_VIDEO_PROFILE_HEVC_MIN: virtio_video_profile = 512;
pub const VIRTIO_VIDEO_PROFILE_HEVC_MAIN: virtio_video_profile = 512;
pub const VIRTIO_VIDEO_PROFILE_HEVC_MAIN10: virtio_video_profile = 513;
pub const VIRTIO_VIDEO_PROFILE_HEVC_MAIN_STILL_PICTURE: virtio_video_profile = 514;
pub const VIRTIO_VIDEO_PROFILE_HEVC_MAX: virtio_video_profile = 514;
pub const VIRTIO_VIDEO_PROFILE_VP8_MIN: virtio_video_profile = 768;
pub const VIRTIO_VIDEO_PROFILE_VP8_PROFILE0: virtio_video_profile = 768;
pub const VIRTIO_VIDEO_PROFILE_VP8_PROFILE1: virtio_video_profile = 769;
pub const VIRTIO_VIDEO_PROFILE_VP8_PROFILE2: virtio_video_profile = 770;
pub const VIRTIO_VIDEO_PROFILE_VP8_PROFILE3: virtio_video_profile = 771;
pub const VIRTIO_VIDEO_PROFILE_VP8_MAX: virtio_video_profile = 771;
pub const VIRTIO_VIDEO_PROFILE_VP9_MIN: virtio_video_profile = 1024;
pub const VIRTIO_VIDEO_PROFILE_VP9_PROFILE0: virtio_video_profile = 1024;
pub const VIRTIO_VIDEO_PROFILE_VP9_PROFILE1: virtio_video_profile = 1025;
pub const VIRTIO_VIDEO_PROFILE_VP9_PROFILE2: virtio_video_profile = 1026;
pub const VIRTIO_VIDEO_PROFILE_VP9_PROFILE3: virtio_video_profile = 1027;
pub const VIRTIO_VIDEO_PROFILE_VP9_MAX: virtio_video_profile = 1027;
pub type virtio_video_profile = u32;
pub const VIRTIO_VIDEO_LEVEL_H264_MIN: virtio_video_level = 256;
pub const VIRTIO_VIDEO_LEVEL_H264_1_0: virtio_video_level = 256;
pub const VIRTIO_VIDEO_LEVEL_H264_1_1: virtio_video_level = 257;
pub const VIRTIO_VIDEO_LEVEL_H264_1_2: virtio_video_level = 258;
pub const VIRTIO_VIDEO_LEVEL_H264_1_3: virtio_video_level = 259;
pub const VIRTIO_VIDEO_LEVEL_H264_2_0: virtio_video_level = 260;
pub const VIRTIO_VIDEO_LEVEL_H264_2_1: virtio_video_level = 261;
pub const VIRTIO_VIDEO_LEVEL_H264_2_2: virtio_video_level = 262;
pub const VIRTIO_VIDEO_LEVEL_H264_3_0: virtio_video_level = 263;
pub const VIRTIO_VIDEO_LEVEL_H264_3_1: virtio_video_level = 264;
pub const VIRTIO_VIDEO_LEVEL_H264_3_2: virtio_video_level = 265;
pub const VIRTIO_VIDEO_LEVEL_H264_4_0: virtio_video_level = 266;
pub const VIRTIO_VIDEO_LEVEL_H264_4_1: virtio_video_level = 267;
pub const VIRTIO_VIDEO_LEVEL_H264_4_2: virtio_video_level = 268;
pub const VIRTIO_VIDEO_LEVEL_H264_5_0: virtio_video_level = 269;
pub const VIRTIO_VIDEO_LEVEL_H264_5_1: virtio_video_level = 270;
pub const VIRTIO_VIDEO_LEVEL_H264_MAX: virtio_video_level = 270;
pub type virtio_video_level = u32;
pub const VIRTIO_VIDEO_BITRATE_MODE_VBR: virtio_video_bitrate_mode = 0;
pub const VIRTIO_VIDEO_BITRATE_MODE_CBR: virtio_video_bitrate_mode = 1;
pub type virtio_video_bitrate_mode = u32;

pub const VIRTIO_VIDEO_CMD_QUERY_CAPABILITY: virtio_video_cmd_type = 256;
pub const VIRTIO_VIDEO_CMD_STREAM_CREATE: virtio_video_cmd_type = 257;
pub const VIRTIO_VIDEO_CMD_STREAM_DESTROY: virtio_video_cmd_type = 258;
pub const VIRTIO_VIDEO_CMD_STREAM_DRAIN: virtio_video_cmd_type = 259;
pub const VIRTIO_VIDEO_CMD_RESOURCE_CREATE: virtio_video_cmd_type = 260;
pub const VIRTIO_VIDEO_CMD_RESOURCE_QUEUE: virtio_video_cmd_type = 261;
pub const VIRTIO_VIDEO_CMD_RESOURCE_DESTROY_ALL: virtio_video_cmd_type = 262;
pub const VIRTIO_VIDEO_CMD_QUEUE_CLEAR: virtio_video_cmd_type = 263;
pub const VIRTIO_VIDEO_CMD_GET_PARAMS: virtio_video_cmd_type = 264;
pub const VIRTIO_VIDEO_CMD_SET_PARAMS: virtio_video_cmd_type = 265;
pub const VIRTIO_VIDEO_CMD_QUERY_CONTROL: virtio_video_cmd_type = 266;
pub const VIRTIO_VIDEO_CMD_GET_CONTROL: virtio_video_cmd_type = 267;
pub const VIRTIO_VIDEO_CMD_SET_CONTROL: virtio_video_cmd_type = 268;
pub const VIRTIO_VIDEO_CMD_GET_PARAMS_EXT: virtio_video_cmd_type = 269;
pub const VIRTIO_VIDEO_CMD_SET_PARAMS_EXT: virtio_video_cmd_type = 270;
pub const VIRTIO_VIDEO_RESP_OK_NODATA: virtio_video_cmd_type = 512;
pub const VIRTIO_VIDEO_RESP_OK_QUERY_CAPABILITY: virtio_video_cmd_type = 513;
pub const VIRTIO_VIDEO_RESP_OK_RESOURCE_QUEUE: virtio_video_cmd_type = 514;
pub const VIRTIO_VIDEO_RESP_OK_GET_PARAMS: virtio_video_cmd_type = 515;
pub const VIRTIO_VIDEO_RESP_OK_QUERY_CONTROL: virtio_video_cmd_type = 516;
pub const VIRTIO_VIDEO_RESP_OK_GET_CONTROL: virtio_video_cmd_type = 517;
pub const VIRTIO_VIDEO_RESP_ERR_INVALID_OPERATION: virtio_video_cmd_type = 768;
pub const VIRTIO_VIDEO_RESP_ERR_OUT_OF_MEMORY: virtio_video_cmd_type = 769;
pub const VIRTIO_VIDEO_RESP_ERR_INVALID_STREAM_ID: virtio_video_cmd_type = 770;
pub const VIRTIO_VIDEO_RESP_ERR_INVALID_RESOURCE_ID: virtio_video_cmd_type = 771;
pub const VIRTIO_VIDEO_RESP_ERR_INVALID_PARAMETER: virtio_video_cmd_type = 772;
pub const VIRTIO_VIDEO_RESP_ERR_UNSUPPORTED_CONTROL: virtio_video_cmd_type = 773;
pub type virtio_video_cmd_type = u32;
#[repr(C)]
#[derive(Debug, Default, Copy, Clone, AsBytes, FromZeroes, FromBytes)]
pub struct virtio_video_cmd_hdr {
    pub type_: Le32,
    pub stream_id: Le32,
}

pub const VIRTIO_VIDEO_QUEUE_TYPE_INPUT: virtio_video_queue_type = 256;
pub const VIRTIO_VIDEO_QUEUE_TYPE_OUTPUT: virtio_video_queue_type = 257;
pub type virtio_video_queue_type = u32;
#[repr(C)]
#[derive(Debug, Default, Copy, Clone, AsBytes, FromZeroes, FromBytes)]
pub struct virtio_video_query_capability {
    pub queue_type: Le32,
    pub padding: [u8; 4usize],
}

pub const VIRTIO_VIDEO_PLANES_LAYOUT_SINGLE_BUFFER: virtio_video_planes_layout_flag = 1;
pub const VIRTIO_VIDEO_PLANES_LAYOUT_PER_PLANE: virtio_video_planes_layout_flag = 2;
pub type virtio_video_planes_layout_flag = u32;
#[repr(C)]
#[derive(Debug, Default, Copy, Clone, AsBytes, FromZeroes, FromBytes)]
pub struct virtio_video_format_range {
    pub min: Le32,
    pub max: Le32,
    pub step: Le32,
    pub padding: [u8; 4usize],
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, AsBytes, FromZeroes, FromBytes)]
pub struct virtio_video_format_frame {
    pub width: virtio_video_format_range,
    pub height: virtio_video_format_range,
    pub num_rates: Le32,
    pub padding: [u8; 4usize],
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, AsBytes, FromZeroes, FromBytes)]
pub struct virtio_video_format_desc {
    pub mask: Le64,
    pub format: Le32,
    pub planes_layout: Le32,
    pub plane_align: Le32,
    pub num_frames: Le32,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, AsBytes, FromZeroes, FromBytes)]
pub struct virtio_video_query_capability_resp {
    pub hdr: virtio_video_cmd_hdr,
    pub num_descs: Le32,
    pub padding: [u8; 4usize],
}

pub const VIRTIO_VIDEO_MEM_TYPE_GUEST_PAGES: virtio_video_mem_type = 0;
pub const VIRTIO_VIDEO_MEM_TYPE_VIRTIO_OBJECT: virtio_video_mem_type = 1;
pub type virtio_video_mem_type = u32;
#[repr(C)]
#[derive(Copy, Clone, AsBytes, FromZeroes, FromBytes)]
pub struct virtio_video_stream_create {
    pub in_mem_type: Le32,
    pub out_mem_type: Le32,
    pub coded_format: Le32,
    pub padding: [u8; 4usize],
    pub tag: [u8; 64usize],
}
impl Default for virtio_video_stream_create {
    fn default() -> Self {
        // SAFETY: trivially safe
        unsafe { ::std::mem::zeroed() }
    }
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, AsBytes, FromZeroes, FromBytes)]
pub struct virtio_video_stream_destroy {}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, AsBytes, FromZeroes, FromBytes)]
pub struct virtio_video_stream_drain {}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, AsBytes, FromZeroes, FromBytes)]
pub struct virtio_video_mem_entry {
    pub addr: Le64,
    pub length: Le32,
    pub padding: [u8; 4usize],
}
#[repr(C)]
#[derive(Debug, Default, Copy, Clone, AsBytes, FromZeroes, FromBytes)]
pub struct virtio_video_object_entry {
    pub uuid: [u8; 16usize],
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, AsBytes, FromZeroes, FromBytes)]
pub struct virtio_video_resource_create {
    pub queue_type: Le32,
    pub resource_id: Le32,
    pub planes_layout: Le32,
    pub num_planes: Le32,
    pub plane_offsets: [Le32; 8usize],
    pub num_entries: [Le32; 8usize],
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, AsBytes, FromZeroes, FromBytes)]
pub struct virtio_video_resource_queue {
    pub queue_type: Le32,
    pub resource_id: Le32,
    pub timestamp: Le64,
    pub num_data_sizes: Le32,
    pub data_sizes: [Le32; 8usize],
    pub padding: [u8; 4usize],
}

pub const VIRTIO_VIDEO_BUFFER_FLAG_ERR: virtio_video_buffer_flag = 1;
pub const VIRTIO_VIDEO_BUFFER_FLAG_EOS: virtio_video_buffer_flag = 2;
pub const VIRTIO_VIDEO_BUFFER_FLAG_IFRAME: virtio_video_buffer_flag = 4;
pub const VIRTIO_VIDEO_BUFFER_FLAG_PFRAME: virtio_video_buffer_flag = 8;
pub const VIRTIO_VIDEO_BUFFER_FLAG_BFRAME: virtio_video_buffer_flag = 16;
pub type virtio_video_buffer_flag = u32;
#[repr(C)]
#[derive(Debug, Default, Copy, Clone, AsBytes, FromZeroes, FromBytes)]
pub struct virtio_video_resource_queue_resp {
    pub hdr: virtio_video_cmd_hdr,
    pub timestamp: Le64,
    pub flags: Le32,
    pub size: Le32,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, AsBytes, FromZeroes, FromBytes)]
pub struct virtio_video_resource_destroy_all {
    pub queue_type: Le32,
    pub padding: [u8; 4usize],
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, AsBytes, FromZeroes, FromBytes)]
pub struct virtio_video_queue_clear {
    pub queue_type: Le32,
    pub padding: [u8; 4usize],
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, AsBytes, FromZeroes, FromBytes)]
pub struct virtio_video_plane_format {
    pub plane_size: Le32,
    pub stride: Le32,
}
#[repr(C)]
#[derive(Debug, Default, Copy, Clone, AsBytes, FromZeroes, FromBytes)]
pub struct virtio_video_crop {
    pub left: Le32,
    pub top: Le32,
    pub width: Le32,
    pub height: Le32,
}
#[repr(C)]
#[derive(Debug, Default, Copy, Clone, AsBytes, FromZeroes, FromBytes)]
pub struct virtio_video_params {
    pub queue_type: Le32,
    pub format: Le32,
    pub frame_width: Le32,
    pub frame_height: Le32,
    pub min_buffers: Le32,
    pub max_buffers: Le32,
    pub crop: virtio_video_crop,
    pub frame_rate: Le32,
    pub num_planes: Le32,
    pub plane_formats: [virtio_video_plane_format; 8usize],
}
#[repr(C)]
#[derive(Debug, Default, Copy, Clone, AsBytes, FromZeroes, FromBytes)]
pub struct virtio_video_get_params {
    pub queue_type: Le32,
    pub padding: [u8; 4usize],
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, AsBytes, FromZeroes, FromBytes)]
pub struct virtio_video_get_params_resp {
    pub hdr: virtio_video_cmd_hdr,
    pub params: virtio_video_params,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, AsBytes, FromZeroes, FromBytes)]
pub struct virtio_video_set_params {
    pub params: virtio_video_params,
}

/// Extension of the {GET,SET}_PARAMS data to also include the resource type. Not including it
/// was an oversight and the {GET,SET}_PARAMS_EXT commands use this structure to fix it, while
/// the older {GET,SET}_PARAMS commands are kept for backward compatibility.
#[repr(C)]
#[derive(Debug, Default, Copy, Clone, AsBytes, FromZeroes, FromBytes)]
pub struct virtio_video_params_ext {
    pub base: virtio_video_params,
    pub resource_type: Le32,
    pub padding: [u8; 4usize],
}
#[repr(C)]
#[derive(Debug, Default, Copy, Clone, AsBytes, FromZeroes, FromBytes)]
pub struct virtio_video_get_params_ext {
    pub queue_type: Le32,
    pub padding: [u8; 4usize],
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, AsBytes, FromZeroes, FromBytes)]
pub struct virtio_video_get_params_ext_resp {
    pub hdr: virtio_video_cmd_hdr,
    pub params: virtio_video_params_ext,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, AsBytes, FromZeroes, FromBytes)]
pub struct virtio_video_set_params_ext {
    pub params: virtio_video_params_ext,
}

pub const VIRTIO_VIDEO_CONTROL_BITRATE: virtio_video_control_type = 1;
pub const VIRTIO_VIDEO_CONTROL_PROFILE: virtio_video_control_type = 2;
pub const VIRTIO_VIDEO_CONTROL_LEVEL: virtio_video_control_type = 3;
pub const VIRTIO_VIDEO_CONTROL_FORCE_KEYFRAME: virtio_video_control_type = 4;
pub const VIRTIO_VIDEO_CONTROL_BITRATE_MODE: virtio_video_control_type = 5;
pub const VIRTIO_VIDEO_CONTROL_BITRATE_PEAK: virtio_video_control_type = 6;
pub const VIRTIO_VIDEO_CONTROL_PREPEND_SPSPPS_TO_IDR: virtio_video_control_type = 7;
pub type virtio_video_control_type = u32;
#[repr(C)]
#[derive(Debug, Default, Copy, Clone, AsBytes, FromZeroes, FromBytes)]
pub struct virtio_video_query_control_profile {
    pub format: Le32,
    pub padding: [u8; 4usize],
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, AsBytes, FromZeroes, FromBytes)]
pub struct virtio_video_query_control_level {
    pub format: Le32,
    pub padding: [u8; 4usize],
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, AsBytes, FromZeroes, FromBytes)]
pub struct virtio_video_query_control {
    pub control: Le32,
    pub padding: [u8; 4usize],
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, AsBytes, FromZeroes, FromBytes)]
pub struct virtio_video_query_control_resp_profile {
    pub num: Le32,
    pub padding: [u8; 4usize],
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, AsBytes, FromZeroes, FromBytes)]
pub struct virtio_video_query_control_resp_level {
    pub num: Le32,
    pub padding: [u8; 4usize],
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, AsBytes, FromZeroes, FromBytes)]
pub struct virtio_video_query_control_resp {
    pub hdr: virtio_video_cmd_hdr,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, AsBytes, FromZeroes, FromBytes)]
pub struct virtio_video_get_control {
    pub control: Le32,
    pub padding: [u8; 4usize],
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, AsBytes, FromZeroes, FromBytes)]
pub struct virtio_video_control_val_bitrate {
    pub bitrate: Le32,
    pub padding: [u8; 4usize],
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, AsBytes, FromZeroes, FromBytes)]
pub struct virtio_video_control_val_bitrate_peak {
    pub bitrate_peak: Le32,
    pub padding: [u8; 4usize],
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, AsBytes, FromZeroes, FromBytes)]
pub struct virtio_video_control_val_bitrate_mode {
    pub bitrate_mode: Le32,
    pub padding: [u8; 4usize],
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, AsBytes, FromZeroes, FromBytes)]
pub struct virtio_video_control_val_profile {
    pub profile: Le32,
    pub padding: [u8; 4usize],
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, AsBytes, FromZeroes, FromBytes)]
pub struct virtio_video_control_val_level {
    pub level: Le32,
    pub padding: [u8; 4usize],
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, AsBytes, FromZeroes, FromBytes)]
pub struct virtio_video_control_val_prepend_spspps_to_idr {
    pub prepend_spspps_to_idr: Le32,
    pub padding: [u8; 4usize],
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, AsBytes, FromZeroes, FromBytes)]
pub struct virtio_video_get_control_resp {
    pub hdr: virtio_video_cmd_hdr,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, AsBytes, FromZeroes, FromBytes)]
pub struct virtio_video_set_control {
    pub control: Le32,
    pub padding: [u8; 4usize],
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, AsBytes, FromZeroes, FromBytes)]
pub struct virtio_video_set_control_resp {
    pub hdr: virtio_video_cmd_hdr,
}

pub const VIRTIO_VIDEO_EVENT_ERROR: virtio_video_event_type = 256;
pub const VIRTIO_VIDEO_EVENT_DECODER_RESOLUTION_CHANGED: virtio_video_event_type = 512;
pub type virtio_video_event_type = u32;
#[repr(C)]
#[derive(Debug, Default, Copy, Clone, AsBytes, FromZeroes, FromBytes)]
pub struct virtio_video_event {
    pub event_type: Le32,
    pub stream_id: Le32,
}
