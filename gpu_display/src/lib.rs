// Copyright 2018 The ChromiumOS Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

//! Crate for displaying simple surfaces and GPU buffers over a low-level display backend such as
//! Wayland or X.

use std::collections::BTreeMap;
use std::io::Error as IoError;
use std::time::Duration;

use base::AsRawDescriptor;
use base::Error as BaseError;
use base::EventToken;
use base::EventType;
use base::VolatileSlice;
use base::WaitContext;
use remain::sorted;
use serde::Deserialize;
use serde::Serialize;
use thiserror::Error;

mod event_device;
mod gpu_display_stub;
#[cfg(windows)]
mod gpu_display_win;
#[cfg(any(target_os = "android", target_os = "linux"))]
mod gpu_display_wl;
#[cfg(feature = "x")]
mod gpu_display_x;
#[cfg(any(windows, feature = "x"))]
mod keycode_converter;
mod sys;

pub use event_device::EventDevice;
pub use event_device::EventDeviceKind;
#[cfg(windows)]
pub use gpu_display_win::DisplayProperties as WinDisplayProperties;
#[cfg(windows)]
pub use gpu_display_win::WindowProcedureThread;
#[cfg(windows)]
pub use gpu_display_win::WindowProcedureThreadBuilder;
use linux_input_sys::virtio_input_event;
use sys::SysDisplayT;
pub use sys::SysGpuDisplayExt;

/// An error generated by `GpuDisplay`.
#[sorted]
#[derive(Error, Debug)]
pub enum GpuDisplayError {
    /// An internal allocation failed.
    #[error("internal allocation failed")]
    Allocate,
    /// A base error occurred.
    #[error("received a base error: {0}")]
    BaseError(BaseError),
    /// Connecting to the compositor failed.
    #[error("failed to connect to compositor")]
    Connect,
    /// Connection to compositor has been broken.
    #[error("connection to compositor has been broken")]
    ConnectionBroken,
    /// Creating event file descriptor failed.
    #[error("failed to create event file descriptor")]
    CreateEvent,
    /// Failed to create a surface on the compositor.
    #[error("failed to crate surface on the compositor")]
    CreateSurface,
    /// Failed to import an event device.
    #[error("failed to import an event device: {0}")]
    FailedEventDeviceImport(String),
    #[error("failed to register an event device to listen for guest events: {0}")]
    FailedEventDeviceListen(base::TubeError),
    /// Failed to import a buffer to the compositor.
    #[error("failed to import a buffer to the compositor")]
    FailedImport,
    /// The import ID is invalid.
    #[error("invalid import ID")]
    InvalidImportId,
    /// The path is invalid.
    #[error("invalid path")]
    InvalidPath,
    /// The surface ID is invalid.
    #[error("invalid surface ID")]
    InvalidSurfaceId,
    /// An input/output error occured.
    #[error("an input/output error occur: {0}")]
    IoError(IoError),
    /// A required feature was missing.
    #[error("required feature was missing: {0}")]
    RequiredFeature(&'static str),
    /// The method is unsupported by the implementation.
    #[error("unsupported by the implementation")]
    Unsupported,
}

pub type GpuDisplayResult<T> = std::result::Result<T, GpuDisplayError>;

impl From<BaseError> for GpuDisplayError {
    fn from(e: BaseError) -> GpuDisplayError {
        GpuDisplayError::BaseError(e)
    }
}

impl From<IoError> for GpuDisplayError {
    fn from(e: IoError) -> GpuDisplayError {
        GpuDisplayError::IoError(e)
    }
}

/// A surface type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SurfaceType {
    /// Scanout surface
    Scanout,
    /// Mouse cursor surface
    Cursor,
}

/// Event token for display instances
#[derive(EventToken, Debug)]
pub enum DisplayEventToken {
    Display,
    EventDevice { event_device_id: u32 },
}

#[derive(Clone)]
pub struct GpuDisplayFramebuffer<'a> {
    framebuffer: VolatileSlice<'a>,
    slice: VolatileSlice<'a>,
    stride: u32,
    bytes_per_pixel: u32,
}

impl<'a> GpuDisplayFramebuffer<'a> {
    fn new(
        framebuffer: VolatileSlice<'a>,
        stride: u32,
        bytes_per_pixel: u32,
    ) -> GpuDisplayFramebuffer {
        GpuDisplayFramebuffer {
            framebuffer,
            slice: framebuffer,
            stride,
            bytes_per_pixel,
        }
    }

    fn sub_region(
        &self,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    ) -> Option<GpuDisplayFramebuffer<'a>> {
        let x_byte_offset = x.checked_mul(self.bytes_per_pixel)?;
        let y_byte_offset = y.checked_mul(self.stride)?;
        let byte_offset = x_byte_offset.checked_add(y_byte_offset)?;

        let width_bytes = width.checked_mul(self.bytes_per_pixel)?;
        let count = height
            .checked_mul(self.stride)?
            .checked_sub(self.stride)?
            .checked_add(width_bytes)?;
        let slice = self
            .framebuffer
            .sub_slice(byte_offset as usize, count as usize)
            .unwrap();

        Some(GpuDisplayFramebuffer { slice, ..*self })
    }

    pub fn as_volatile_slice(&self) -> VolatileSlice<'a> {
        self.slice
    }

    pub fn stride(&self) -> u32 {
        self.stride
    }
}

/// Empty trait, just used as a bounds for now
trait GpuDisplayImport {}

trait GpuDisplaySurface {
    /// Returns an unique ID associated with the surface.  This is typically generated by the
    /// compositor or cast of a raw pointer.
    fn surface_descriptor(&self) -> u64 {
        0
    }

    /// Returns the next framebuffer, allocating if necessary.
    fn framebuffer(&mut self) -> Option<GpuDisplayFramebuffer> {
        None
    }

    /// Returns true if the next buffer in the swapchain is already in use.
    fn next_buffer_in_use(&self) -> bool {
        false
    }

    /// Returns true if the surface should be closed.
    fn close_requested(&self) -> bool {
        false
    }

    /// Puts the next buffer on the screen, making it the current buffer.
    fn flip(&mut self) {
        // no-op
    }

    /// Puts the specified import_id on the screen.
    fn flip_to(&mut self, _import_id: u32) {
        // no-op
    }

    /// Commits the surface to the compositor.
    fn commit(&mut self) -> GpuDisplayResult<()> {
        Ok(())
    }

    /// Sets the position of the identified subsurface relative to its parent.
    fn set_position(&mut self, _x: u32, _y: u32) {
        // no-op
    }

    /// Returns the type of the completed buffer.
    fn buffer_completion_type(&self) -> u32 {
        0
    }

    /// Draws the current buffer on the screen.
    fn draw_current_buffer(&mut self) {
        // no-op
    }

    /// Handles a compositor-specific client event.
    fn on_client_message(&mut self, _client_data: u64) {
        // no-op
    }

    /// Handles a compositor-specific shared memory completion event.
    fn on_shm_completion(&mut self, _shm_complete: u64) {
        // no-op
    }

    /// Sets the scanout ID for the surface.
    fn set_scanout_id(&mut self, _scanout_id: u32) {
        // no-op
    }
}

struct GpuDisplayEvents {
    events: Vec<virtio_input_event>,
    device_type: EventDeviceKind,
}

trait DisplayT: AsRawDescriptor {
    /// Returns true if there are events that are on the queue.
    fn pending_events(&self) -> bool {
        false
    }

    /// Sends any pending commands to the compositor.
    fn flush(&self) {
        // no-op
    }

    /// Returns the surface descirptor associated with the current event
    fn next_event(&mut self) -> GpuDisplayResult<u64> {
        Ok(0)
    }

    /// Handles the event from the compositor, and returns an list of events
    fn handle_next_event(
        &mut self,
        _surface: &mut Box<dyn GpuDisplaySurface>,
    ) -> Option<GpuDisplayEvents> {
        None
    }

    /// Creates a surface with the given parameters.  The display backend is given a non-zero
    /// `surface_id` as a handle for subsequent operations.
    fn create_surface(
        &mut self,
        parent_surface_id: Option<u32>,
        surface_id: u32,
        width: u32,
        height: u32,
        surf_type: SurfaceType,
    ) -> GpuDisplayResult<Box<dyn GpuDisplaySurface>>;

    /// Imports memory into the display backend.  The display backend is given a non-zero
    /// `import_id` as a handle for subsequent operations.
    fn import_memory(
        &mut self,
        _import_id: u32,
        _descriptor: &dyn AsRawDescriptor,
        _offset: u32,
        _stride: u32,
        _modifiers: u64,
        _width: u32,
        _height: u32,
        _fourcc: u32,
    ) -> GpuDisplayResult<Box<dyn GpuDisplayImport>> {
        Err(GpuDisplayError::Unsupported)
    }
}

pub trait GpuDisplayExt {
    /// Imports the given `event_device` into the display, returning an event device id on success.
    /// This device may be used to dispatch input events to the guest.
    fn import_event_device(&mut self, event_device: EventDevice) -> GpuDisplayResult<u32>;

    /// Called when an event device is readable.
    fn handle_event_device(&mut self, event_device_id: u32);
}

/// A connection to the compositor and associated collection of state.
///
/// The user of `GpuDisplay` can use `AsRawDescriptor` to poll on the compositor connection's file
/// descriptor. When the connection is readable, `dispatch_events` can be called to process it.
pub struct GpuDisplay {
    next_id: u32,
    event_devices: BTreeMap<u32, EventDevice>,
    surfaces: BTreeMap<u32, Box<dyn GpuDisplaySurface>>,
    imports: BTreeMap<u32, Box<dyn GpuDisplayImport>>,
    wait_ctx: WaitContext<DisplayEventToken>,
    // `inner` must be after `imports` and `surfaces` to ensure those objects are dropped before
    // the display context. The drop order for fields inside a struct is the order in which they
    // are declared [Rust RFC 1857].
    //
    // We also don't want to drop inner before wait_ctx because it contains references to the event
    // devices owned by inner.display_event_dispatcher.
    inner: Box<dyn SysDisplayT>,
}

impl GpuDisplay {
    /// Opens a connection to X server
    pub fn open_x(display_name: Option<&str>) -> GpuDisplayResult<GpuDisplay> {
        let _ = display_name;
        #[cfg(feature = "x")]
        {
            let display = gpu_display_x::DisplayX::open_display(display_name)?;

            let wait_ctx = WaitContext::new()?;
            wait_ctx.add(&display, DisplayEventToken::Display)?;

            Ok(GpuDisplay {
                inner: Box::new(display),
                next_id: 1,
                event_devices: Default::default(),
                surfaces: Default::default(),
                imports: Default::default(),
                wait_ctx,
            })
        }
        #[cfg(not(feature = "x"))]
        Err(GpuDisplayError::Unsupported)
    }

    pub fn open_stub() -> GpuDisplayResult<GpuDisplay> {
        let display = gpu_display_stub::DisplayStub::new()?;
        let wait_ctx = WaitContext::new()?;
        wait_ctx.add(&display, DisplayEventToken::Display)?;

        Ok(GpuDisplay {
            inner: Box::new(display),
            next_id: 1,
            event_devices: Default::default(),
            surfaces: Default::default(),
            imports: Default::default(),
            wait_ctx,
        })
    }

    // Leaves the `GpuDisplay` in a undefined state.
    //
    // TODO: Would be nice to change receiver from `&mut self` to `self`. Requires some refactoring
    // elsewhere.
    pub fn take_event_devices(&mut self) -> Vec<EventDevice> {
        std::mem::take(&mut self.event_devices)
            .into_values()
            .collect()
    }

    fn dispatch_display_events(&mut self) -> GpuDisplayResult<()> {
        self.inner.flush();
        while self.inner.pending_events() {
            let surface_descriptor = self.inner.next_event()?;

            for surface in self.surfaces.values_mut() {
                if surface_descriptor != surface.surface_descriptor() {
                    continue;
                }

                if let Some(gpu_display_events) = self.inner.handle_next_event(surface) {
                    for event_device in self.event_devices.values_mut() {
                        if event_device.kind() != gpu_display_events.device_type {
                            continue;
                        }

                        event_device.send_report(gpu_display_events.events.iter().cloned())?;
                    }
                }
            }
        }

        Ok(())
    }

    /// Dispatches internal events that were received from the compositor since the last call to
    /// `dispatch_events`.
    pub fn dispatch_events(&mut self) -> GpuDisplayResult<()> {
        let wait_events = self.wait_ctx.wait_timeout(Duration::default())?;

        if let Some(wait_event) = wait_events.iter().find(|e| e.is_hungup) {
            base::error!(
                "Display signaled with a hungup event for token {:?}",
                wait_event.token
            );
            self.wait_ctx = WaitContext::new().unwrap();
            return GpuDisplayResult::Err(GpuDisplayError::ConnectionBroken);
        }

        for wait_event in wait_events.iter().filter(|e| e.is_writable) {
            if let DisplayEventToken::EventDevice { event_device_id } = wait_event.token {
                if let Some(event_device) = self.event_devices.get_mut(&event_device_id) {
                    if !event_device.flush_buffered_events()? {
                        continue;
                    }
                    self.wait_ctx.modify(
                        event_device,
                        EventType::Read,
                        DisplayEventToken::EventDevice { event_device_id },
                    )?;
                }
            }
        }

        for wait_event in wait_events.iter().filter(|e| e.is_readable) {
            match wait_event.token {
                DisplayEventToken::Display => self.dispatch_display_events()?,
                DisplayEventToken::EventDevice { event_device_id } => {
                    self.handle_event_device(event_device_id)
                }
            }
        }

        Ok(())
    }

    /// Creates a surface on the the compositor as either a top level window, or child of another
    /// surface, returning a handle to the new surface.
    pub fn create_surface(
        &mut self,
        parent_surface_id: Option<u32>,
        width: u32,
        height: u32,
        surf_type: SurfaceType,
    ) -> GpuDisplayResult<u32> {
        if let Some(parent_id) = parent_surface_id {
            if !self.surfaces.contains_key(&parent_id) {
                return Err(GpuDisplayError::InvalidSurfaceId);
            }
        }

        let new_surface_id = self.next_id;
        let new_surface = self.inner.create_surface(
            parent_surface_id,
            new_surface_id,
            width,
            height,
            surf_type,
        )?;

        self.next_id += 1;
        self.surfaces.insert(new_surface_id, new_surface);
        Ok(new_surface_id)
    }

    /// Releases a previously created surface identified by the given handle.
    pub fn release_surface(&mut self, surface_id: u32) {
        self.surfaces.remove(&surface_id);
    }

    /// Gets a reference to an unused framebuffer for the identified surface.
    pub fn framebuffer(&mut self, surface_id: u32) -> Option<GpuDisplayFramebuffer> {
        let surface = self.surfaces.get_mut(&surface_id)?;
        surface.framebuffer()
    }

    /// Gets a reference to an unused framebuffer for the identified surface.
    pub fn framebuffer_region(
        &mut self,
        surface_id: u32,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    ) -> Option<GpuDisplayFramebuffer> {
        let framebuffer = self.framebuffer(surface_id)?;
        framebuffer.sub_region(x, y, width, height)
    }

    /// Returns true if the next buffer in the buffer queue for the given surface is currently in
    /// use.
    ///
    /// If the next buffer is in use, the memory returned from `framebuffer_memory` should not be
    /// written to.
    pub fn next_buffer_in_use(&self, surface_id: u32) -> bool {
        self.surfaces
            .get(&surface_id)
            .map(|s| s.next_buffer_in_use())
            .unwrap_or(false)
    }

    /// Changes the visible contents of the identified surface to the contents of the framebuffer
    /// last returned by `framebuffer_memory` for this surface.
    pub fn flip(&mut self, surface_id: u32) {
        if let Some(surface) = self.surfaces.get_mut(&surface_id) {
            surface.flip()
        }
    }

    /// Returns true if the identified top level surface has been told to close by the compositor,
    /// and by extension the user.
    pub fn close_requested(&self, surface_id: u32) -> bool {
        self.surfaces
            .get(&surface_id)
            .map(|s| s.close_requested())
            .unwrap_or(true)
    }

    /// Imports memory to the compositor for use as a surface buffer and returns a handle
    /// to it.
    pub fn import_memory(
        &mut self,
        descriptor: &dyn AsRawDescriptor,
        offset: u32,
        stride: u32,
        modifiers: u64,
        width: u32,
        height: u32,
        fourcc: u32,
    ) -> GpuDisplayResult<u32> {
        let import_id = self.next_id;

        let gpu_display_memory = self.inner.import_memory(
            import_id, descriptor, offset, stride, modifiers, width, height, fourcc,
        )?;

        self.next_id += 1;
        self.imports.insert(import_id, gpu_display_memory);
        Ok(import_id)
    }

    /// Releases a previously imported memory identified by the given handle.
    pub fn release_import(&mut self, import_id: u32) {
        self.imports.remove(&import_id);
    }

    /// Commits any pending state for the identified surface.
    pub fn commit(&mut self, surface_id: u32) -> GpuDisplayResult<()> {
        let surface = self
            .surfaces
            .get_mut(&surface_id)
            .ok_or(GpuDisplayError::InvalidSurfaceId)?;

        surface.commit()
    }

    /// Changes the visible contents of the identified surface to that of the identified imported
    /// buffer.
    pub fn flip_to(&mut self, surface_id: u32, import_id: u32) -> GpuDisplayResult<()> {
        let surface = self
            .surfaces
            .get_mut(&surface_id)
            .ok_or(GpuDisplayError::InvalidSurfaceId)?;

        if !self.imports.contains_key(&import_id) {
            return Err(GpuDisplayError::InvalidImportId);
        }

        surface.flip_to(import_id);
        Ok(())
    }

    /// Sets the position of the identified subsurface relative to its parent.
    ///
    /// The change in position will not be visible until `commit` is called for the parent surface.
    pub fn set_position(&mut self, surface_id: u32, x: u32, y: u32) -> GpuDisplayResult<()> {
        let surface = self
            .surfaces
            .get_mut(&surface_id)
            .ok_or(GpuDisplayError::InvalidSurfaceId)?;

        surface.set_position(x, y);
        Ok(())
    }

    /// Associates the scanout id with the given surface.
    pub fn set_scanout_id(&mut self, surface_id: u32, scanout_id: u32) -> GpuDisplayResult<()> {
        let surface = self
            .surfaces
            .get_mut(&surface_id)
            .ok_or(GpuDisplayError::InvalidSurfaceId)?;

        surface.set_scanout_id(scanout_id);
        Ok(())
    }
}
