// Copyright 2022 The Chromium OS Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use anyhow::{anyhow, Result};

use crate::bindings;

/// A wrapper over VAGenericValue so we can safely access the underlying union
/// members
#[derive(Debug)]
pub enum GenericValue {
    /// A wrapper over VAGenericValueTypeInteger
    Integer(i32),
    /// A wrapper over VAGenericValueTypeFloat
    Float(f32),
    /// A wrapper over VAGenericValueTypePointer
    Pointer(*mut std::os::raw::c_void),
    /// A wrapper over VAGenericValueTypeFunc
    Func(bindings::VAGenericFunc),
}

impl TryFrom<bindings::VAGenericValue> for GenericValue {
    type Error = anyhow::Error;

    fn try_from(value: bindings::VAGenericValue) -> Result<Self, Self::Error> {
        // Safe because we check the type before accessing the union.
        match value.type_ {
            // Safe because we check the type before accessing the union.
            bindings::VAGenericValueType::VAGenericValueTypeInteger => {
                Ok(Self::Integer(unsafe { value.value.i }))
            }
            bindings::VAGenericValueType::VAGenericValueTypeFloat => {
                Ok(Self::Float(unsafe { value.value.f }))
            }
            bindings::VAGenericValueType::VAGenericValueTypePointer => {
                Ok(Self::Pointer(unsafe { value.value.p }))
            }
            bindings::VAGenericValueType::VAGenericValueTypeFunc => {
                Ok(Self::Func(unsafe { value.value.fn_ }))
            }
            other => {
                return Err(anyhow!(
                    "Conversion failed for unexpected VAGenericValueType: {}",
                    other
                ))
            }
        }
    }
}
