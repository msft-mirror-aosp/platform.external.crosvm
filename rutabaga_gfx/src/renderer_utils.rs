// Copyright 2020 The Chromium OS Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

//! renderer_utils: Utility functions and structs used by virgl_renderer and gfxstream.

use std::cell::RefCell;
use std::os::raw::c_void;
use std::rc::Rc;

use crate::generated::virgl_renderer_bindings::__va_list_tag;
use crate::rutabaga_utils::{RutabagaError, RutabagaResult};

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct VirglBox {
    pub x: u32,
    pub y: u32,
    pub z: u32,
    pub w: u32,
    pub h: u32,
    pub d: u32,
}

/*
 * automatically generated by rust-bindgen
 * $ bindgen /usr/include/stdio.h \
 *       --no-layout-tests \
 *       --whitelist-function vsnprintf \
 *       -o vsnprintf.rs
 */

#[allow(non_snake_case, non_camel_case_types)]
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
extern "C" {
    pub fn vsnprintf(
        __s: *mut ::std::os::raw::c_char,
        __maxlen: ::std::os::raw::c_ulong,
        __format: *const ::std::os::raw::c_char,
        __arg: *mut __va_list_tag,
    ) -> ::std::os::raw::c_int;
}

pub fn ret_to_res(ret: i32) -> RutabagaResult<()> {
    match ret {
        0 => Ok(()),
        _ => Err(RutabagaError::ComponentError(ret)),
    }
}

pub struct FenceState {
    pub latest_fence: u32,
}

impl FenceState {
    pub fn write(&mut self, latest_fence: u32) {
        if latest_fence > self.latest_fence {
            self.latest_fence = latest_fence;
        }
    }
}

pub struct VirglCookie {
    pub fence_state: Rc<RefCell<FenceState>>,
}

pub extern "C" fn write_fence(cookie: *mut c_void, fence: u32) {
    assert!(!cookie.is_null());
    let cookie = unsafe { &*(cookie as *mut VirglCookie) };

    // Track the most recent fence.
    let mut fence_state = cookie.fence_state.borrow_mut();
    fence_state.write(fence);
}
