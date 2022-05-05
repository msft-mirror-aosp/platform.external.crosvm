// Copyright 2022 The Chromium OS Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

pub mod tap;
pub use tap::Tap;

use crate::TapTCommon;
use base::FileReadWriteVolatile;

// TODO(b/159159958) implement FileReadWriteVolatile for slirp
pub trait TapT: FileReadWriteVolatile + TapTCommon {}

pub mod fakes {
    pub use super::tap::fakes::FakeTap;
}
