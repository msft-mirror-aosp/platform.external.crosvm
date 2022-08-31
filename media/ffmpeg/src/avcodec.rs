// Copyright 2022 The Chromium OS Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

//! This module implements a lightweight and safe decoder interface over `libavcodec`. It is
//! designed to concentrate all calls to unsafe methods in one place, while providing the same
//! low-level access as the libavcodec functions do.

use std::ffi::CStr;
use std::fmt::Debug;
use std::fmt::Display;
use std::marker::PhantomData;
use std::mem::ManuallyDrop;
use std::ops::Deref;

use libc::c_char;
use libc::c_int;
use libc::c_void;
use thiserror::Error as ThisError;

use super::*;

/// An error returned by a low-level libavcodec function.
#[derive(Debug, ThisError)]
pub struct AvError(pub libc::c_int);

impl AvError {
    pub fn result(ret: c_int) -> Result<(), Self> {
        if ret >= 0 {
            Ok(())
        } else {
            Err(AvError(ret))
        }
    }
}

impl Display for AvError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut buffer = [0u8; 255];
        // Safe because we are passing valid bounds for the buffer.
        let ret = unsafe {
            ffi::av_strerror(
                self.0,
                buffer.as_mut_ptr() as *mut c_char,
                buffer.len() as ffi::size_t,
            )
        };
        match ret {
            ret if ret >= 0 => {
                let end_of_string = buffer.iter().position(|i| *i == 0).unwrap_or(buffer.len());
                let error_string = std::string::String::from_utf8_lossy(&buffer[..end_of_string]);
                f.write_str(&error_string)
            }
            _ => f.write_fmt(format_args!("Unknown avcodec error {}", self.0)),
        }
    }
}

/// Lightweight abstraction over libavcodec's `AVCodec` struct, allowing the query the capabilities
/// of supported codecs and opening a session to work with them.
///
/// `AVCodec` instances in libavcodec are all static, hence we can safely use a static reference
/// lifetime here.
pub struct AvCodec(&'static ffi::AVCodec);

#[derive(Debug, ThisError)]
pub enum AvCodecOpenError {
    #[error("failed to allocate AVContext object")]
    ContextAllocation,
    #[error("failed to open AVContext object")]
    ContextOpen,
}

impl AvCodec {
    /// Returns whether the codec is a decoder.
    pub fn is_decoder(&self) -> bool {
        // Safe because `av_codec_is_decoder` is called on a valid static `AVCodec` reference.
        (unsafe { ffi::av_codec_is_decoder(self.0) } != 0)
    }

    /// Returns whether the codec is an encoder.
    pub fn is_encoder(&self) -> bool {
        // Safe because `av_codec_is_decoder` is called on a valid static `AVCodec` reference.
        (unsafe { ffi::av_codec_is_encoder(self.0) } != 0)
    }

    /// Returns the name of the codec.
    pub fn name(&self) -> &'static str {
        const INVALID_CODEC_STR: &str = "invalid codec";

        // Safe because `CStr::from_ptr` is called on a valid zero-terminated C string.
        unsafe { CStr::from_ptr(self.0.name).to_str() }.unwrap_or(INVALID_CODEC_STR)
    }

    /// Returns the capabilities of the codec, as a mask of AV_CODEC_CAP_* bits.
    pub fn capabilities(&self) -> u32 {
        self.0.capabilities as u32
    }

    /// Returns an iterator over the profiles supported by this codec.
    pub fn profile_iter(&self) -> AvProfileIterator {
        AvProfileIterator(self.0.profiles)
    }

    /// Returns an iterator over the pixel formats supported by this codec.
    ///
    /// For a decoder, the returned array will likely be empty. This means that ffmpeg's native
    /// pixel format (YUV420) will be used.
    pub fn pixel_format_iter(&self) -> AvPixelFormatIterator {
        AvPixelFormatIterator(self.0.pix_fmts)
    }

    /// Obtain a context that can be used to decode using this codec.
    ///
    /// `get_buffer`'s first element is an optional function that decides which buffer is used to
    /// render a frame (see libavcodec's documentation for `get_buffer2` for more details). If
    /// provided, this function must be thread-safe. If none is provided, avcodec's default function
    /// is used. The second element is a pointer that will be passed as first argument to the
    /// function when it is called.
    pub fn open(
        &self,
        get_buffer: Option<(
            unsafe extern "C" fn(*mut ffi::AVCodecContext, *mut ffi::AVFrame, i32) -> i32,
            *mut libc::c_void,
        )>,
    ) -> Result<AvCodecContext, AvCodecOpenError> {
        // Safe because `self.0` is a valid static AVCodec reference.
        let mut context = unsafe { ffi::avcodec_alloc_context3(self.0).as_mut() }
            .ok_or(AvCodecOpenError::ContextAllocation)?;

        if let Some((get_buffer2, opaque)) = get_buffer {
            context.get_buffer2 = Some(get_buffer2);
            context.opaque = opaque;
            context.thread_safe_callbacks = 1;
        }

        // Safe because `self.0` is a valid static AVCodec reference, and `context` has been
        // successfully allocated above.
        if unsafe { ffi::avcodec_open2(context, self.0, std::ptr::null_mut()) } < 0 {
            return Err(AvCodecOpenError::ContextOpen);
        }

        Ok(AvCodecContext(context))
    }
}

/// Lightweight abstraction over libavcodec's `av_codec_iterate` function that can be used to
/// enumerate all the supported codecs.
pub struct AvCodecIterator(*mut libc::c_void);

impl AvCodecIterator {
    pub fn new() -> Self {
        Self(std::ptr::null_mut())
    }
}

impl Iterator for AvCodecIterator {
    type Item = AvCodec;

    fn next(&mut self) -> Option<Self::Item> {
        // Safe because our pointer was initialized to `NULL` and we only use it with
        // `av_codec_iterate`, which will update it to a valid value.
        unsafe { ffi::av_codec_iterate(&mut self.0 as *mut *mut libc::c_void).as_ref() }
            .map(AvCodec)
    }
}

/// Simple wrapper over `AVProfile` that provides helpful methods.
pub struct AvProfile(&'static ffi::AVProfile);

impl AvProfile {
    /// Return the profile id, which can be matched against FF_PROFILE_*.
    pub fn profile(&self) -> u32 {
        self.0.profile as u32
    }

    /// Return the name of this profile.
    pub fn name(&self) -> &'static str {
        const INVALID_PROFILE_STR: &str = "invalid profile";

        // Safe because `CStr::from_ptr` is called on a valid zero-terminated C string.
        unsafe { CStr::from_ptr(self.0.name).to_str() }.unwrap_or(INVALID_PROFILE_STR)
    }
}

impl Display for AvProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

impl Debug for AvProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}

/// Lightweight abstraction over the array of supported profiles for a given codec.
pub struct AvProfileIterator(*const ffi::AVProfile);

impl Iterator for AvProfileIterator {
    type Item = AvProfile;

    fn next(&mut self) -> Option<Self::Item> {
        // Safe because the contract of `new` stipulates we have received a valid `AVCodec`
        // reference, thus the `profiles` pointer must either be NULL or point to a valid array
        // or `VAProfile`s.
        match unsafe { self.0.as_ref() } {
            None => None,
            Some(profile) => {
                match profile.profile {
                    ffi::FF_PROFILE_UNKNOWN => None,
                    _ => {
                        // Safe because we have been initialized to a static, valid profiles array
                        // which is terminated by FF_PROFILE_UNKNOWN.
                        self.0 = unsafe { self.0.offset(1) };
                        Some(AvProfile(profile))
                    }
                }
            }
        }
    }
}

/// Simple wrapper over `AVPixelFormat` that provides helpful methods.
pub struct AvPixelFormat(ffi::AVPixelFormat);

impl AvPixelFormat {
    /// Return the name of this pixel format.
    pub fn name(&self) -> &'static str {
        const INVALID_FORMAT_STR: &str = "invalid pixel format";

        // Safe because `av_get_pix_fmt_name` returns either NULL or a valid C string.
        let pix_fmt_name = unsafe { ffi::av_get_pix_fmt_name(self.0) };
        // Safe because `pix_fmt_name` is a valid pointer to a C string.
        match unsafe {
            pix_fmt_name
                .as_ref()
                .and_then(|s| CStr::from_ptr(s).to_str().ok())
        } {
            None => INVALID_FORMAT_STR,
            Some(string) => string,
        }
    }

    /// Return the avcodec profile id, which can be matched against AV_PIX_FMT_*.
    ///
    /// Note that this is **not** the same as a fourcc.
    pub fn pix_fmt(&self) -> ffi::AVPixelFormat {
        self.0
    }

    /// Return the fourcc of the pixel format, or a series of zeros if its fourcc is unknown.
    pub fn fourcc(&self) -> [u8; 4] {
        // Safe because `avcodec_pix_fmt_to_codec_tag` does not take any pointer as input and
        // handles any value passed as argument.
        unsafe { ffi::avcodec_pix_fmt_to_codec_tag(self.0) }.to_le_bytes()
    }
}

impl Display for AvPixelFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

impl Debug for AvPixelFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let fourcc = self.fourcc();
        f.write_fmt(format_args!(
            "{}{}{}{}",
            fourcc[0] as char, fourcc[1] as char, fourcc[2] as char, fourcc[3] as char
        ))
    }
}

/// Lightweight abstraction over the array of supported pixel formats for a given codec.
pub struct AvPixelFormatIterator(*const ffi::AVPixelFormat);

impl Iterator for AvPixelFormatIterator {
    type Item = AvPixelFormat;

    fn next(&mut self) -> Option<Self::Item> {
        // Safe because the contract of `AvCodec::new` and `AvCodec::pixel_format_iter` guarantees
        // that we have been built from a valid `AVCodec` reference, which `pix_fmts` pointer
        // must either be NULL or point to a valid array or `VAPixelFormat`s.
        match unsafe { self.0.as_ref() } {
            None => None,
            Some(&pixfmt) => {
                match pixfmt {
                    // Array of pixel formats is terminated by AV_PIX_FMT_NONE.
                    ffi::AVPixelFormat_AV_PIX_FMT_NONE => None,
                    _ => {
                        // Safe because we have been initialized to a static, valid profiles array
                        // which is terminated by AV_PIX_FMT_NONE.
                        self.0 = unsafe { self.0.offset(1) };
                        Some(AvPixelFormat(pixfmt))
                    }
                }
            }
        }
    }
}

/// A codec context from which decoding can be performed.
pub struct AvCodecContext(*mut ffi::AVCodecContext);

impl Drop for AvCodecContext {
    fn drop(&mut self) {
        // Safe because our context member is properly initialized, fully owned by us, and has not
        // leaked in any form.
        unsafe { ffi::avcodec_free_context(&mut self.0) };
    }
}

impl AsRef<ffi::AVCodecContext> for AvCodecContext {
    fn as_ref(&self) -> &ffi::AVCodecContext {
        // Safe because our context member is properly initialized and fully owned by us.
        unsafe { &*self.0 }
    }
}

pub enum TryReceiveFrameResult {
    Received,
    TryAgain,
    FlushCompleted,
}

impl AvCodecContext {
    /// Send a packet to be decoded to the codec.
    ///
    /// Returns `true` if the packet has been accepted and will be decoded, `false` if the codec can
    /// not accept frames at the moment - in this case `try_receive_frame` must be called before
    /// the packet can be submitted again.
    ///
    /// Error codes are the same as those returned by `avcodec_send_packet` with the exception of
    /// EAGAIN which is converted into `Ok(false)` as it is not actually an error.
    pub fn try_send_packet<'a>(&mut self, packet: &AvPacket<'a>) -> Result<bool, AvError> {
        // Safe because the context is valid through the life of this object, and `packet`'s
        // lifetime properties ensures its memory area is readable.
        match unsafe { ffi::avcodec_send_packet(self.0, &packet.packet) } {
            AVERROR_EAGAIN => Ok(false),
            ret if ret >= 0 => Ok(true),
            err => Err(AvError(err)),
        }
    }

    /// Attempt to write a decoded frame in `frame` if the codec has enough data to do so.
    ///
    /// Returned `Received` if `frame` has been filled with the next decoded frame, `TryAgain` if
    /// no frame could be returned at that time (in which case `try_send_packet` should be called to
    /// submit more input to decode), or `FlushCompleted` to signal that a previous flush triggered
    /// by calling the `flush` method has completed.
    ///
    /// Error codes are the same as those returned by `avcodec_receive_frame`.
    pub fn try_receive_frame(
        &mut self,
        frame: &mut AvFrame,
    ) -> Result<TryReceiveFrameResult, AvError> {
        // Safe because the context is valid through the life of this object, and `avframe` is
        // guaranteed to contain a properly initialized frame.
        match unsafe { ffi::avcodec_receive_frame(self.0, frame.0) } {
            AVERROR_EAGAIN => Ok(TryReceiveFrameResult::TryAgain),
            AVERROR_EOF => Ok(TryReceiveFrameResult::FlushCompleted),
            ret if ret >= 0 => Ok(TryReceiveFrameResult::Received),
            err => Err(AvError(err)),
        }
    }

    /// Reset the internal codec state/flush internal buffers.
    /// Should be called e.g. when seeking or switching to a different stream.
    pub fn reset(&mut self) {
        // Safe because the context is valid through the life of this object.
        unsafe { ffi::avcodec_flush_buffers(self.0) }
    }

    /// Ask the context to start flushing, i.e. to process all pending input packets and produce
    /// frames for them.
    ///
    /// The flush process is complete when `try_receive_frame` returns `FlushCompleted`,
    pub fn flush(&mut self) -> Result<(), AvError> {
        // Safe because the context is valid through the life of this object.
        AvError::result(unsafe { ffi::avcodec_send_packet(self.0, std::ptr::null()) })
    }
}

/// Trait for types that can be used as data provider for a `AVBuffer`.
///
/// `AVBuffer` is an owned buffer type, so all the type needs to do is being able to provide a
/// stable pointer to its own data as well as its length. Implementors need to be sendable across
/// threads because avcodec is allowed to use threads in its codec implementations.
pub trait AvBufferSource: Send {
    fn as_ptr(&self) -> *const u8;
    fn as_mut_ptr(&mut self) -> *mut u8 {
        self.as_ptr() as *mut u8
    }
    fn len(&self) -> usize;
}

/// Wrapper around `AVBuffer` and `AVBufferRef`.
///
/// libavcodec can manage its own memory for input and output data. Doing so implies a transparent
/// copy of user-provided data (packets or frames) from and to this memory, which is wasteful.
///
/// This copy can be avoided by explicitly providing our own buffers to libavcodec using
/// `AVBufferRef`. Doing so means that the lifetime of these buffers becomes managed by avcodec.
/// This struct helps make this process safe by taking full ownership of an `AvBufferSource` and
/// dropping it when libavcodec is done with it.
struct AvBuffer(*mut ffi::AVBufferRef);

impl AvBuffer {
    /// Create a new `AvBuffer` from an `AvBufferSource`.
    ///
    /// Ownership of `source` is transferred to libavcodec, which will drop it when the number of
    /// references to this buffer reaches zero.
    ///
    /// Returns `None` if the buffer could not be created due to an error in libavcodec.
    fn new<D: AvBufferSource>(source: D) -> Option<Self> {
        // Move storage to the heap so we find it at the same place in `avbuffer_free`
        let mut storage = Box::new(source);

        extern "C" fn avbuffer_free<D>(opaque: *mut c_void, _data: *mut u8) {
            // Safe because `opaque` has been created from `Box::into_raw`. `storage` will be
            // dropped immediately which will release any resources held by the storage.
            let _ = unsafe { Box::from_raw(opaque as *mut D) };
        }

        // Safe because storage points to valid data and we are checking the return value against
        // NULL, which signals an error.
        Some(Self(unsafe {
            ffi::av_buffer_create(
                storage.as_mut_ptr(),
                storage.len() as ffi::size_t,
                Some(avbuffer_free::<D>),
                Box::into_raw(storage) as *mut c_void,
                0,
            )
            .as_mut()?
        }))
    }

    /// Return a slice to the data contained in this buffer.
    fn as_mut_slice(&mut self) -> &mut [u8] {
        // Safe because the data has been initialized from valid storage in the constructor.
        unsafe { std::slice::from_raw_parts_mut((*self.0).data, (*self.0).size as usize) }
    }

    /// Consumes the `AVBuffer`, returning a `AVBufferRef` that can be used in `AVFrame`, `AVPacket`
    /// and others.
    ///
    /// After calling, the caller is responsible for unref-ing the returned AVBufferRef, either
    /// directly or through one of the automatic management facilities in `AVFrame`, `AVPacket` or
    /// others.
    fn into_raw(self) -> *mut ffi::AVBufferRef {
        ManuallyDrop::new(self).0
    }
}

impl Drop for AvBuffer {
    fn drop(&mut self) {
        // Safe because `self.0` is a valid pointer to an AVBufferRef.
        unsafe { ffi::av_buffer_unref(&mut self.0) };
    }
}

/// An encoded input packet that can be submitted to `AvCodecContext::try_send_packet`.
pub struct AvPacket<'a> {
    packet: ffi::AVPacket,
    _buffer_data: PhantomData<&'a ()>,
}

impl<'a> Drop for AvPacket<'a> {
    fn drop(&mut self) {
        // Safe because `self.packet` is a valid `AVPacket` instance.
        unsafe {
            ffi::av_packet_unref(&mut self.packet);
        }
    }
}

#[derive(Debug, ThisError)]
pub enum AvPacketError {
    #[error("failed to create an AvBuffer from the input buffer")]
    AvBufferCreationError,
}

impl<'a> AvPacket<'a> {
    /// Create a new AvPacket that borrows the `input_data`.
    ///
    /// The returned `AvPacket` will hold a reference to `input_data`, meaning that libavcodec might
    /// perform a copy from/to it.
    pub fn new<T: AvBufferSource>(pts: i64, input_data: &'a mut T) -> Self {
        Self {
            packet: ffi::AVPacket {
                buf: std::ptr::null_mut(),
                pts,
                dts: AV_NOPTS_VALUE as i64,
                data: input_data.as_mut_ptr(),
                size: input_data.len() as c_int,
                side_data: std::ptr::null_mut(),
                pos: -1,
                // Safe because all the other elements of this struct can be zeroed.
                ..unsafe { std::mem::zeroed() }
            },
            _buffer_data: PhantomData,
        }
    }

    /// Create a new AvPacket that owns the `input_data`.
    ///
    /// The returned `AvPacket` will have a `'static` lifetime and will keep `input_data` alive for
    /// as long as libavcodec needs it.
    pub fn new_owned<T: AvBufferSource>(pts: i64, input_data: T) -> Result<Self, AvPacketError> {
        let mut av_buffer =
            AvBuffer::new(input_data).ok_or(AvPacketError::AvBufferCreationError)?;
        let data_slice = av_buffer.as_mut_slice();
        let data = data_slice.as_mut_ptr();
        let size = data_slice.len() as i32;

        let ret = Self {
            packet: ffi::AVPacket {
                buf: av_buffer.into_raw(),
                pts,
                dts: AV_NOPTS_VALUE as i64,
                data,
                size,
                side_data: std::ptr::null_mut(),
                pos: -1,
                // Safe because all the other elements of this struct can be zeroed.
                ..unsafe { std::mem::zeroed() }
            },
            _buffer_data: PhantomData,
        };

        Ok(ret)
    }
}

/// An owned AVFrame, i.e. one decoded frame from libavcodec that can be converted into a
/// destination buffer.
pub struct AvFrame(*mut ffi::AVFrame);

#[derive(Debug, ThisError)]
pub enum AvFrameError {
    #[error("failed to allocate AVFrame object")]
    FrameAllocationFailed,
}

impl AvFrame {
    /// Create a new AvFrame. The frame's parameters and backing memory will be assigned when it is
    /// decoded into.
    pub fn new() -> Result<Self, AvFrameError> {
        Ok(Self(
            // Safe because `av_frame_alloc` does not take any input.
            unsafe { ffi::av_frame_alloc().as_mut() }.ok_or(AvFrameError::FrameAllocationFailed)?,
        ))
    }
}

impl AsRef<ffi::AVFrame> for AvFrame {
    fn as_ref(&self) -> &ffi::AVFrame {
        // Safe because the AVFrame has been properly initialized during construction.
        unsafe { &*self.0 }
    }
}

impl Deref for AvFrame {
    type Target = ffi::AVFrame;

    fn deref(&self) -> &Self::Target {
        // Safe because the AVFrame has been properly initialized during construction.
        unsafe { self.0.as_ref().unwrap() }
    }
}

impl Drop for AvFrame {
    fn drop(&mut self) {
        // Safe because the AVFrame is valid through the life of this object and fully owned by us.
        unsafe { ffi::av_frame_free(&mut self.0) };
    }
}

#[cfg(test)]
mod tests {
    use std::ptr;
    use std::sync::atomic::AtomicBool;
    use std::sync::atomic::Ordering;
    use std::sync::Arc;

    use super::*;

    #[test]
    fn test_averror() {
        // Just test that the error is wrapper properly. The bindings test module already checks
        // that the error bindings correspond to the right ffmpeg errors.
        let averror = AvError(AVERROR_EOF);
        let msg = format!("{}", averror);
        assert_eq!(msg, "End of file");

        let averror = AvError(0);
        let msg = format!("{}", averror);
        assert_eq!(msg, "Success");

        let averror = AvError(10);
        let msg = format!("{}", averror);
        assert_eq!(msg, "Unknown avcodec error 10");
    }

    // Test that the AVPacket wrapper frees the owned AVBuffer on drop.
    #[test]
    fn test_avpacket_drop() {
        struct DropTestBufferSource {
            dropped: Arc<AtomicBool>,
        }
        impl Drop for DropTestBufferSource {
            fn drop(&mut self) {
                self.dropped.store(true, Ordering::SeqCst);
            }
        }
        impl AvBufferSource for DropTestBufferSource {
            fn as_ptr(&self) -> *const u8 {
                ptr::null()
            }

            fn len(&self) -> usize {
                0
            }
        }

        let dropped = Arc::new(AtomicBool::new(false));

        let pkt = AvPacket::new_owned(
            0,
            DropTestBufferSource {
                dropped: dropped.clone(),
            },
        )
        .unwrap();
        assert!(!dropped.load(Ordering::SeqCst));
        drop(pkt);
        assert!(dropped.load(Ordering::SeqCst));
    }
}
