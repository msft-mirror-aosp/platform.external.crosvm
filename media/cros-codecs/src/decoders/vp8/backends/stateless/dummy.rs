// Copyright 2022 The ChromiumOS Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

// This file contains a dummy backend whose only purpose is to let the decoder
// run so we can test it in isolation.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use crate::decoders::vp8::backends::stateless::StatelessDecoderBackend;
use crate::decoders::vp8::backends::stateless::Vp8Picture;
use crate::decoders::VideoDecoderBackend;
use crate::utils::dummy::*;

impl StatelessDecoderBackend for Backend {
    type Handle = Handle<Vp8Picture<BackendHandle>>;

    fn new_sequence(&mut self, _: &crate::decoders::vp8::parser::Header) -> super::Result<()> {
        Ok(())
    }

    fn submit_picture(
        &mut self,
        picture: Vp8Picture<super::AsBackendHandle<Self::Handle>>,
        _: Option<&Self::Handle>,
        _: Option<&Self::Handle>,
        _: Option<&Self::Handle>,
        _: &dyn AsRef<[u8]>,
        _: &crate::decoders::vp8::parser::Parser,
        _: u64,
        _: bool,
    ) -> super::Result<Self::Handle> {
        Ok(Handle {
            handle: Rc::new(RefCell::new(picture)),
        })
    }

    fn poll(&mut self, _: super::BlockingMode) -> super::Result<VecDeque<Self::Handle>> {
        Ok(VecDeque::new())
    }

    fn handle_is_ready(&self, _: &Self::Handle) -> bool {
        true
    }

    fn block_on_handle(&mut self, _: &Self::Handle) -> super::Result<()> {
        Ok(())
    }

    fn as_video_decoder_backend_mut(&mut self) -> &mut dyn VideoDecoderBackend {
        self
    }

    fn as_video_decoder_backend(&self) -> &dyn VideoDecoderBackend {
        self
    }
}
