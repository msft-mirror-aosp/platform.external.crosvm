/* automatically generated by tools/bindgen-all-the-things */

#![allow(clippy::missing_safety_doc)]
#![allow(clippy::upper_case_acronyms)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

pub const VA_MAJOR_VERSION: u32 = 1;
pub const VA_MINOR_VERSION: u32 = 15;
pub const VA_MICRO_VERSION: u32 = 0;
pub const VA_VERSION_S: &[u8; 7usize] = b"1.15.0\0";
pub const VA_VERSION_HEX: u32 = 17760256;
pub const VA_STATUS_SUCCESS: u32 = 0;
pub const VA_STATUS_ERROR_OPERATION_FAILED: u32 = 1;
pub const VA_STATUS_ERROR_ALLOCATION_FAILED: u32 = 2;
pub const VA_STATUS_ERROR_INVALID_DISPLAY: u32 = 3;
pub const VA_STATUS_ERROR_INVALID_CONFIG: u32 = 4;
pub const VA_STATUS_ERROR_INVALID_CONTEXT: u32 = 5;
pub const VA_STATUS_ERROR_INVALID_SURFACE: u32 = 6;
pub const VA_STATUS_ERROR_INVALID_BUFFER: u32 = 7;
pub const VA_STATUS_ERROR_INVALID_IMAGE: u32 = 8;
pub const VA_STATUS_ERROR_INVALID_SUBPICTURE: u32 = 9;
pub const VA_STATUS_ERROR_ATTR_NOT_SUPPORTED: u32 = 10;
pub const VA_STATUS_ERROR_MAX_NUM_EXCEEDED: u32 = 11;
pub const VA_STATUS_ERROR_UNSUPPORTED_PROFILE: u32 = 12;
pub const VA_STATUS_ERROR_UNSUPPORTED_ENTRYPOINT: u32 = 13;
pub const VA_STATUS_ERROR_UNSUPPORTED_RT_FORMAT: u32 = 14;
pub const VA_STATUS_ERROR_UNSUPPORTED_BUFFERTYPE: u32 = 15;
pub const VA_STATUS_ERROR_SURFACE_BUSY: u32 = 16;
pub const VA_STATUS_ERROR_FLAG_NOT_SUPPORTED: u32 = 17;
pub const VA_STATUS_ERROR_INVALID_PARAMETER: u32 = 18;
pub const VA_STATUS_ERROR_RESOLUTION_NOT_SUPPORTED: u32 = 19;
pub const VA_STATUS_ERROR_UNIMPLEMENTED: u32 = 20;
pub const VA_STATUS_ERROR_SURFACE_IN_DISPLAYING: u32 = 21;
pub const VA_STATUS_ERROR_INVALID_IMAGE_FORMAT: u32 = 22;
pub const VA_STATUS_ERROR_DECODING_ERROR: u32 = 23;
pub const VA_STATUS_ERROR_ENCODING_ERROR: u32 = 24;
pub const VA_STATUS_ERROR_INVALID_VALUE: u32 = 25;
pub const VA_STATUS_ERROR_UNSUPPORTED_FILTER: u32 = 32;
pub const VA_STATUS_ERROR_INVALID_FILTER_CHAIN: u32 = 33;
pub const VA_STATUS_ERROR_HW_BUSY: u32 = 34;
pub const VA_STATUS_ERROR_UNSUPPORTED_MEMORY_TYPE: u32 = 36;
pub const VA_STATUS_ERROR_NOT_ENOUGH_BUFFER: u32 = 37;
pub const VA_STATUS_ERROR_TIMEDOUT: u32 = 38;
pub const VA_STATUS_ERROR_UNKNOWN: u32 = 4294967295;
pub const VA_FRAME_PICTURE: u32 = 0;
pub const VA_TOP_FIELD: u32 = 1;
pub const VA_BOTTOM_FIELD: u32 = 2;
pub const VA_TOP_FIELD_FIRST: u32 = 4;
pub const VA_BOTTOM_FIELD_FIRST: u32 = 8;
pub const VA_ENABLE_BLEND: u32 = 4;
pub const VA_CLEAR_DRAWABLE: u32 = 8;
pub const VA_SRC_COLOR_MASK: u32 = 240;
pub const VA_SRC_BT601: u32 = 16;
pub const VA_SRC_BT709: u32 = 32;
pub const VA_SRC_SMPTE_240: u32 = 64;
pub const VA_FILTER_SCALING_DEFAULT: u32 = 0;
pub const VA_FILTER_SCALING_FAST: u32 = 256;
pub const VA_FILTER_SCALING_HQ: u32 = 512;
pub const VA_FILTER_SCALING_NL_ANAMORPHIC: u32 = 768;
pub const VA_FILTER_SCALING_MASK: u32 = 3840;
pub const VA_FILTER_INTERPOLATION_DEFAULT: u32 = 0;
pub const VA_FILTER_INTERPOLATION_NEAREST_NEIGHBOR: u32 = 4096;
pub const VA_FILTER_INTERPOLATION_BILINEAR: u32 = 8192;
pub const VA_FILTER_INTERPOLATION_ADVANCED: u32 = 12288;
pub const VA_FILTER_INTERPOLATION_MASK: u32 = 61440;
pub const VA_PADDING_LOW: u32 = 4;
pub const VA_PADDING_MEDIUM: u32 = 8;
pub const VA_PADDING_HIGH: u32 = 16;
pub const VA_PADDING_LARGE: u32 = 32;
pub const VA_EXEC_SYNC: u32 = 0;
pub const VA_EXEC_ASYNC: u32 = 1;
pub const VA_EXEC_MODE_DEFAULT: u32 = 0;
pub const VA_EXEC_MODE_POWER_SAVING: u32 = 1;
pub const VA_EXEC_MODE_PERFORMANCE: u32 = 2;
pub const VA_FEATURE_NOT_SUPPORTED: u32 = 0;
pub const VA_FEATURE_SUPPORTED: u32 = 1;
pub const VA_FEATURE_REQUIRED: u32 = 2;
pub const VA_RT_FORMAT_YUV420: u32 = 1;
pub const VA_RT_FORMAT_YUV422: u32 = 2;
pub const VA_RT_FORMAT_YUV444: u32 = 4;
pub const VA_RT_FORMAT_YUV411: u32 = 8;
pub const VA_RT_FORMAT_YUV400: u32 = 16;
pub const VA_RT_FORMAT_YUV420_10: u32 = 256;
pub const VA_RT_FORMAT_YUV422_10: u32 = 512;
pub const VA_RT_FORMAT_YUV444_10: u32 = 1024;
pub const VA_RT_FORMAT_YUV420_12: u32 = 4096;
pub const VA_RT_FORMAT_YUV422_12: u32 = 8192;
pub const VA_RT_FORMAT_YUV444_12: u32 = 16384;
pub const VA_RT_FORMAT_RGB16: u32 = 65536;
pub const VA_RT_FORMAT_RGB32: u32 = 131072;
pub const VA_RT_FORMAT_RGBP: u32 = 1048576;
pub const VA_RT_FORMAT_RGB32_10: u32 = 2097152;
pub const VA_RT_FORMAT_PROTECTED: u32 = 2147483648;
pub const VA_RT_FORMAT_RGB32_10BPP: u32 = 2097152;
pub const VA_RT_FORMAT_YUV420_10BPP: u32 = 256;
pub const VA_RC_NONE: u32 = 1;
pub const VA_RC_CBR: u32 = 2;
pub const VA_RC_VBR: u32 = 4;
pub const VA_RC_VCM: u32 = 8;
pub const VA_RC_CQP: u32 = 16;
pub const VA_RC_VBR_CONSTRAINED: u32 = 32;
pub const VA_RC_ICQ: u32 = 64;
pub const VA_RC_MB: u32 = 128;
pub const VA_RC_CFS: u32 = 256;
pub const VA_RC_PARALLEL: u32 = 512;
pub const VA_RC_QVBR: u32 = 1024;
pub const VA_RC_AVBR: u32 = 2048;
pub const VA_RC_TCBRC: u32 = 4096;
pub const VA_DEC_SLICE_MODE_NORMAL: u32 = 1;
pub const VA_DEC_SLICE_MODE_BASE: u32 = 2;
pub const VA_DEC_PROCESSING_NONE: u32 = 0;
pub const VA_DEC_PROCESSING: u32 = 1;
pub const VA_ENC_PACKED_HEADER_NONE: u32 = 0;
pub const VA_ENC_PACKED_HEADER_SEQUENCE: u32 = 1;
pub const VA_ENC_PACKED_HEADER_PICTURE: u32 = 2;
pub const VA_ENC_PACKED_HEADER_SLICE: u32 = 4;
pub const VA_ENC_PACKED_HEADER_MISC: u32 = 8;
pub const VA_ENC_PACKED_HEADER_RAW_DATA: u32 = 16;
pub const VA_ENC_INTERLACED_NONE: u32 = 0;
pub const VA_ENC_INTERLACED_FRAME: u32 = 1;
pub const VA_ENC_INTERLACED_FIELD: u32 = 2;
pub const VA_ENC_INTERLACED_MBAFF: u32 = 4;
pub const VA_ENC_INTERLACED_PAFF: u32 = 8;
pub const VA_ENC_SLICE_STRUCTURE_POWER_OF_TWO_ROWS: u32 = 1;
pub const VA_ENC_SLICE_STRUCTURE_ARBITRARY_MACROBLOCKS: u32 = 2;
pub const VA_ENC_SLICE_STRUCTURE_EQUAL_ROWS: u32 = 4;
pub const VA_ENC_SLICE_STRUCTURE_MAX_SLICE_SIZE: u32 = 8;
pub const VA_ENC_SLICE_STRUCTURE_ARBITRARY_ROWS: u32 = 16;
pub const VA_ENC_SLICE_STRUCTURE_EQUAL_MULTI_ROWS: u32 = 32;
pub const VA_ENC_QUANTIZATION_NONE: u32 = 0;
pub const VA_ENC_QUANTIZATION_TRELLIS_SUPPORTED: u32 = 1;
pub const VA_PREDICTION_DIRECTION_PREVIOUS: u32 = 1;
pub const VA_PREDICTION_DIRECTION_FUTURE: u32 = 2;
pub const VA_PREDICTION_DIRECTION_BI_NOT_EMPTY: u32 = 4;
pub const VA_ENC_INTRA_REFRESH_NONE: u32 = 0;
pub const VA_ENC_INTRA_REFRESH_ROLLING_COLUMN: u32 = 1;
pub const VA_ENC_INTRA_REFRESH_ROLLING_ROW: u32 = 2;
pub const VA_ENC_INTRA_REFRESH_ADAPTIVE: u32 = 16;
pub const VA_ENC_INTRA_REFRESH_CYCLIC: u32 = 32;
pub const VA_ENC_INTRA_REFRESH_P_FRAME: u32 = 65536;
pub const VA_ENC_INTRA_REFRESH_B_FRAME: u32 = 131072;
pub const VA_ENC_INTRA_REFRESH_MULTI_REF: u32 = 262144;
pub const VA_PC_CIPHER_AES: u32 = 1;
pub const VA_PC_BLOCK_SIZE_128: u32 = 1;
pub const VA_PC_BLOCK_SIZE_192: u32 = 2;
pub const VA_PC_BLOCK_SIZE_256: u32 = 4;
pub const VA_PC_CIPHER_MODE_ECB: u32 = 1;
pub const VA_PC_CIPHER_MODE_CBC: u32 = 2;
pub const VA_PC_CIPHER_MODE_CTR: u32 = 4;
pub const VA_PC_SAMPLE_TYPE_FULLSAMPLE: u32 = 1;
pub const VA_PC_SAMPLE_TYPE_SUBSAMPLE: u32 = 2;
pub const VA_PC_USAGE_DEFAULT: u32 = 0;
pub const VA_PC_USAGE_WIDEVINE: u32 = 1;
pub const VA_PROCESSING_RATE_NONE: u32 = 0;
pub const VA_PROCESSING_RATE_ENCODE: u32 = 1;
pub const VA_PROCESSING_RATE_DECODE: u32 = 2;
pub const VA_ATTRIB_NOT_SUPPORTED: u32 = 2147483648;
pub const VA_INVALID_ID: u32 = 4294967295;
pub const VA_INVALID_SURFACE: u32 = 4294967295;
pub const VA_SURFACE_ATTRIB_NOT_SUPPORTED: u32 = 0;
pub const VA_SURFACE_ATTRIB_GETTABLE: u32 = 1;
pub const VA_SURFACE_ATTRIB_SETTABLE: u32 = 2;
pub const VA_SURFACE_ATTRIB_MEM_TYPE_VA: u32 = 1;
pub const VA_SURFACE_ATTRIB_MEM_TYPE_V4L2: u32 = 2;
pub const VA_SURFACE_ATTRIB_MEM_TYPE_USER_PTR: u32 = 4;
pub const VA_SURFACE_EXTBUF_DESC_ENABLE_TILING: u32 = 1;
pub const VA_SURFACE_EXTBUF_DESC_CACHED: u32 = 2;
pub const VA_SURFACE_EXTBUF_DESC_UNCACHED: u32 = 4;
pub const VA_SURFACE_EXTBUF_DESC_WC: u32 = 8;
pub const VA_SURFACE_EXTBUF_DESC_PROTECTED: u32 = 2147483648;
pub const VA_SURFACE_ATTRIB_USAGE_HINT_GENERIC: u32 = 0;
pub const VA_SURFACE_ATTRIB_USAGE_HINT_DECODER: u32 = 1;
pub const VA_SURFACE_ATTRIB_USAGE_HINT_ENCODER: u32 = 2;
pub const VA_SURFACE_ATTRIB_USAGE_HINT_VPP_READ: u32 = 4;
pub const VA_SURFACE_ATTRIB_USAGE_HINT_VPP_WRITE: u32 = 8;
pub const VA_SURFACE_ATTRIB_USAGE_HINT_DISPLAY: u32 = 16;
pub const VA_SURFACE_ATTRIB_USAGE_HINT_EXPORT: u32 = 32;
pub const VA_PROGRESSIVE: u32 = 1;
pub const VA_ENCRYPTION_TYPE_FULLSAMPLE_CTR: u32 = 1;
pub const VA_ENCRYPTION_TYPE_FULLSAMPLE_CBC: u32 = 2;
pub const VA_ENCRYPTION_TYPE_SUBSAMPLE_CTR: u32 = 4;
pub const VA_ENCRYPTION_TYPE_SUBSAMPLE_CBC: u32 = 8;
pub const VA_SLICE_DATA_FLAG_ALL: u32 = 0;
pub const VA_SLICE_DATA_FLAG_BEGIN: u32 = 1;
pub const VA_SLICE_DATA_FLAG_MIDDLE: u32 = 2;
pub const VA_SLICE_DATA_FLAG_END: u32 = 4;
pub const VA_MB_TYPE_MOTION_FORWARD: u32 = 2;
pub const VA_MB_TYPE_MOTION_BACKWARD: u32 = 4;
pub const VA_MB_TYPE_MOTION_PATTERN: u32 = 8;
pub const VA_MB_TYPE_MOTION_INTRA: u32 = 16;
pub const VA_PICTURE_H264_INVALID: u32 = 1;
pub const VA_PICTURE_H264_TOP_FIELD: u32 = 2;
pub const VA_PICTURE_H264_BOTTOM_FIELD: u32 = 4;
pub const VA_PICTURE_H264_SHORT_TERM_REFERENCE: u32 = 8;
pub const VA_PICTURE_H264_LONG_TERM_REFERENCE: u32 = 16;
pub const VA_CODED_BUF_STATUS_PICTURE_AVE_QP_MASK: u32 = 255;
pub const VA_CODED_BUF_STATUS_LARGE_SLICE_MASK: u32 = 256;
pub const VA_CODED_BUF_STATUS_SLICE_OVERFLOW_MASK: u32 = 512;
pub const VA_CODED_BUF_STATUS_BITRATE_OVERFLOW: u32 = 1024;
pub const VA_CODED_BUF_STATUS_BITRATE_HIGH: u32 = 2048;
pub const VA_CODED_BUF_STATUS_FRAME_SIZE_OVERFLOW: u32 = 4096;
pub const VA_CODED_BUF_STATUS_BAD_BITSTREAM: u32 = 32768;
pub const VA_CODED_BUF_STATUS_AIR_MB_OVER_THRESHOLD: u32 = 16711680;
pub const VA_CODED_BUF_STATUS_NUMBER_PASSES_MASK: u32 = 251658240;
pub const VA_CODED_BUF_STATUS_SINGLE_NALU: u32 = 268435456;
pub const VA_EXPORT_SURFACE_READ_ONLY: u32 = 1;
pub const VA_EXPORT_SURFACE_WRITE_ONLY: u32 = 2;
pub const VA_EXPORT_SURFACE_READ_WRITE: u32 = 3;
pub const VA_EXPORT_SURFACE_SEPARATE_LAYERS: u32 = 4;
pub const VA_EXPORT_SURFACE_COMPOSED_LAYERS: u32 = 8;
pub const VA_TIMEOUT_INFINITE: i32 = -1;
pub const VA_FOURCC_NV12: u32 = 842094158;
pub const VA_FOURCC_NV21: u32 = 825382478;
pub const VA_FOURCC_AI44: u32 = 875839817;
pub const VA_FOURCC_RGBA: u32 = 1094862674;
pub const VA_FOURCC_RGBX: u32 = 1480738642;
pub const VA_FOURCC_BGRA: u32 = 1095911234;
pub const VA_FOURCC_BGRX: u32 = 1481787202;
pub const VA_FOURCC_ARGB: u32 = 1111970369;
pub const VA_FOURCC_XRGB: u32 = 1111970392;
pub const VA_FOURCC_ABGR: u32 = 1380401729;
pub const VA_FOURCC_XBGR: u32 = 1380401752;
pub const VA_FOURCC_UYVY: u32 = 1498831189;
pub const VA_FOURCC_YUY2: u32 = 844715353;
pub const VA_FOURCC_AYUV: u32 = 1448433985;
pub const VA_FOURCC_NV11: u32 = 825316942;
pub const VA_FOURCC_YV12: u32 = 842094169;
pub const VA_FOURCC_P208: u32 = 942682704;
pub const VA_FOURCC_I420: u32 = 808596553;
pub const VA_FOURCC_YV24: u32 = 875714137;
pub const VA_FOURCC_YV32: u32 = 842225241;
pub const VA_FOURCC_Y800: u32 = 808466521;
pub const VA_FOURCC_IMC3: u32 = 860048713;
pub const VA_FOURCC_411P: u32 = 1345401140;
pub const VA_FOURCC_411R: u32 = 1378955572;
pub const VA_FOURCC_422H: u32 = 1211249204;
pub const VA_FOURCC_422V: u32 = 1446130228;
pub const VA_FOURCC_444P: u32 = 1345598516;
pub const VA_FOURCC_RGBP: u32 = 1346520914;
pub const VA_FOURCC_BGRP: u32 = 1347569474;
pub const VA_FOURCC_RGB565: u32 = 909199186;
pub const VA_FOURCC_BGR565: u32 = 909199170;
pub const VA_FOURCC_Y210: u32 = 808530521;
pub const VA_FOURCC_Y212: u32 = 842084953;
pub const VA_FOURCC_Y216: u32 = 909193817;
pub const VA_FOURCC_Y410: u32 = 808531033;
pub const VA_FOURCC_Y412: u32 = 842085465;
pub const VA_FOURCC_Y416: u32 = 909194329;
pub const VA_FOURCC_YV16: u32 = 909203033;
pub const VA_FOURCC_P010: u32 = 808530000;
pub const VA_FOURCC_P012: u32 = 842084432;
pub const VA_FOURCC_P016: u32 = 909193296;
pub const VA_FOURCC_I010: u32 = 808529993;
pub const VA_FOURCC_IYUV: u32 = 1448433993;
pub const VA_FOURCC_A2R10G10B10: u32 = 808669761;
pub const VA_FOURCC_A2B10G10R10: u32 = 808665665;
pub const VA_FOURCC_X2R10G10B10: u32 = 808669784;
pub const VA_FOURCC_X2B10G10R10: u32 = 808665688;
pub const VA_FOURCC_Y8: u32 = 538982489;
pub const VA_FOURCC_Y16: u32 = 540422489;
pub const VA_FOURCC_VYUY: u32 = 1498765654;
pub const VA_FOURCC_YVYU: u32 = 1431918169;
pub const VA_FOURCC_ARGB64: u32 = 877089345;
pub const VA_FOURCC_ABGR64: u32 = 877085249;
pub const VA_FOURCC_XYUV: u32 = 1448434008;
pub const VA_LSB_FIRST: u32 = 1;
pub const VA_MSB_FIRST: u32 = 2;
pub const VA_SUBPICTURE_CHROMA_KEYING: u32 = 1;
pub const VA_SUBPICTURE_GLOBAL_ALPHA: u32 = 2;
pub const VA_SUBPICTURE_DESTINATION_IS_SCREEN_COORD: u32 = 4;
pub const VA_ROTATION_NONE: u32 = 0;
pub const VA_ROTATION_90: u32 = 1;
pub const VA_ROTATION_180: u32 = 2;
pub const VA_ROTATION_270: u32 = 3;
pub const VA_MIRROR_NONE: u32 = 0;
pub const VA_MIRROR_HORIZONTAL: u32 = 1;
pub const VA_MIRROR_VERTICAL: u32 = 2;
pub const VA_OOL_DEBLOCKING_FALSE: u32 = 0;
pub const VA_OOL_DEBLOCKING_TRUE: u32 = 1;
pub const VA_RENDER_MODE_UNDEFINED: u32 = 0;
pub const VA_RENDER_MODE_LOCAL_OVERLAY: u32 = 1;
pub const VA_RENDER_MODE_LOCAL_GPU: u32 = 2;
pub const VA_RENDER_MODE_EXTERNAL_OVERLAY: u32 = 4;
pub const VA_RENDER_MODE_EXTERNAL_GPU: u32 = 8;
pub const VA_RENDER_DEVICE_UNDEFINED: u32 = 0;
pub const VA_RENDER_DEVICE_LOCAL: u32 = 1;
pub const VA_RENDER_DEVICE_EXTERNAL: u32 = 2;
pub const VA_DISPLAY_ATTRIB_NOT_SUPPORTED: u32 = 0;
pub const VA_DISPLAY_ATTRIB_GETTABLE: u32 = 1;
pub const VA_DISPLAY_ATTRIB_SETTABLE: u32 = 2;
pub const VA_PICTURE_HEVC_INVALID: u32 = 1;
pub const VA_PICTURE_HEVC_FIELD_PIC: u32 = 2;
pub const VA_PICTURE_HEVC_BOTTOM_FIELD: u32 = 4;
pub const VA_PICTURE_HEVC_LONG_TERM_REFERENCE: u32 = 8;
pub const VA_PICTURE_HEVC_RPS_ST_CURR_BEFORE: u32 = 16;
pub const VA_PICTURE_HEVC_RPS_ST_CURR_AFTER: u32 = 32;
pub const VA_PICTURE_HEVC_RPS_LT_CURR: u32 = 64;
pub const VA_FEI_FUNCTION_ENC: u32 = 1;
pub const VA_FEI_FUNCTION_PAK: u32 = 2;
pub const VA_FEI_FUNCTION_ENC_PAK: u32 = 4;
pub const VA_PICTURE_STATS_INVALID: u32 = 1;
pub const VA_PICTURE_STATS_PROGRESSIVE: u32 = 0;
pub const VA_PICTURE_STATS_TOP_FIELD: u32 = 2;
pub const VA_PICTURE_STATS_BOTTOM_FIELD: u32 = 4;
pub const VA_PICTURE_STATS_CONTENT_UPDATED: u32 = 16;
pub const VA_MB_PRED_AVAIL_TOP_LEFT: u32 = 4;
pub const VA_MB_PRED_AVAIL_TOP: u32 = 16;
pub const VA_MB_PRED_AVAIL_TOP_RIGHT: u32 = 8;
pub const VA_MB_PRED_AVAIL_LEFT: u32 = 64;
pub const VA_AV1_MAX_SEGMENTS: u32 = 8;
pub const VA_AV1_SEG_LVL_MAX: u32 = 8;
pub const VA_BLEND_GLOBAL_ALPHA: u32 = 1;
pub const VA_BLEND_PREMULTIPLIED_ALPHA: u32 = 2;
pub const VA_BLEND_LUMA_KEY: u32 = 16;
pub const VA_PROC_PIPELINE_SUBPICTURES: u32 = 1;
pub const VA_PROC_PIPELINE_FAST: u32 = 2;
pub const VA_PROC_FILTER_MANDATORY: u32 = 1;
pub const VA_PIPELINE_FLAG_END: u32 = 4;
pub const VA_CHROMA_SITING_UNKNOWN: u32 = 0;
pub const VA_CHROMA_SITING_VERTICAL_TOP: u32 = 1;
pub const VA_CHROMA_SITING_VERTICAL_CENTER: u32 = 2;
pub const VA_CHROMA_SITING_VERTICAL_BOTTOM: u32 = 3;
pub const VA_CHROMA_SITING_HORIZONTAL_LEFT: u32 = 4;
pub const VA_CHROMA_SITING_HORIZONTAL_CENTER: u32 = 8;
pub const VA_SOURCE_RANGE_UNKNOWN: u32 = 0;
pub const VA_SOURCE_RANGE_REDUCED: u32 = 1;
pub const VA_SOURCE_RANGE_FULL: u32 = 2;
pub const VA_TONE_MAPPING_HDR_TO_HDR: u32 = 1;
pub const VA_TONE_MAPPING_HDR_TO_SDR: u32 = 2;
pub const VA_TONE_MAPPING_HDR_TO_EDR: u32 = 4;
pub const VA_TONE_MAPPING_SDR_TO_HDR: u32 = 8;
pub const VA_DEINTERLACING_BOTTOM_FIELD_FIRST: u32 = 1;
pub const VA_DEINTERLACING_BOTTOM_FIELD: u32 = 2;
pub const VA_DEINTERLACING_ONE_FIELD: u32 = 4;
pub const VA_DEINTERLACING_FMD_ENABLE: u32 = 8;
pub const VA_DEINTERLACING_SCD_ENABLE: u32 = 16;
pub const VA_PROC_HVS_DENOISE_DEFAULT: u32 = 0;
pub const VA_PROC_HVS_DENOISE_AUTO_BDRATE: u32 = 1;
pub const VA_PROC_HVS_DENOISE_AUTO_SUBJECTIVE: u32 = 2;
pub const VA_PROC_HVS_DENOISE_MANUAL: u32 = 3;
pub const VA_3DLUT_CHANNEL_UNKNOWN: u32 = 0;
pub const VA_3DLUT_CHANNEL_RGB_RGB: u32 = 1;
pub const VA_3DLUT_CHANNEL_YUV_RGB: u32 = 2;
pub const VA_3DLUT_CHANNEL_VUY_RGB: u32 = 4;
