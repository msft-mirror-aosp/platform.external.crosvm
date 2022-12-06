// Copyright 2022 The ChromiumOS Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

//! This file contains a dummy backend whose only purpose is to let the decoder
//! run so we can test it in isolation.

use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

use crate::decoders::DecodedHandle;
use crate::decoders::DynPicture;
use crate::decoders::FrameInfo;
use crate::decoders::MappableHandle;
use crate::decoders::Picture;
use crate::decoders::Result;
use crate::decoders::VideoDecoderBackend;
use crate::DecodedFormat;
use crate::Resolution;

pub struct BackendHandle;

impl MappableHandle for BackendHandle {
    fn read(&mut self, _: &mut [u8]) -> Result<()> {
        Ok(())
    }

    fn image_size(&mut self) -> usize {
        1
    }
}

impl<CodecData: FrameInfo> DynPicture for Picture<CodecData, BackendHandle> {
    fn dyn_mappable_handle_mut<'a>(&'a mut self) -> Box<dyn MappableHandle + 'a> {
        Box::new(BackendHandle)
    }
}

pub struct Handle<T> {
    pub handle: Rc<RefCell<T>>,
}

impl<T> Clone for Handle<T> {
    fn clone(&self) -> Self {
        Self {
            handle: Rc::clone(&self.handle),
        }
    }
}

impl<T: FrameInfo> DecodedHandle for Handle<Picture<T, BackendHandle>> {
    type CodecData = T;
    type BackendHandle = BackendHandle;

    fn picture_container(&self) -> &Rc<RefCell<Picture<Self::CodecData, Self::BackendHandle>>> {
        &self.handle
    }

    fn display_resolution(&self) -> Resolution {
        self.picture().data.display_resolution()
    }

    fn display_order(&self) -> Option<u64> {
        None
    }

    fn set_display_order(&mut self, _: u64) {}
}

/// Dummy backend that can be used for any codec.
pub struct Backend;

impl VideoDecoderBackend for Backend {
    fn num_resources_total(&self) -> usize {
        1
    }

    fn num_resources_left(&self) -> usize {
        1
    }

    fn format(&self) -> Option<DecodedFormat> {
        None
    }

    fn try_format(&mut self, _: DecodedFormat) -> crate::decoders::Result<()> {
        Ok(())
    }

    fn supported_formats_for_stream(&self) -> crate::decoders::Result<HashSet<DecodedFormat>> {
        Ok(HashSet::new())
    }

    fn coded_resolution(&self) -> Option<Resolution> {
        None
    }

    fn display_resolution(&self) -> Option<Resolution> {
        None
    }
}
