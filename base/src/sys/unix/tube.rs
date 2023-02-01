// Copyright 2021 The ChromiumOS Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use std::io::IoSlice;
use std::io::IoSliceMut;
use std::os::unix::prelude::AsRawFd;
use std::os::unix::prelude::RawFd;
use std::time::Duration;

use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde::Serialize;

use crate::descriptor::AsRawDescriptor;
use crate::descriptor::FromRawDescriptor;
use crate::descriptor::SafeDescriptor;
use crate::platform::deserialize_with_descriptors;
use crate::platform::SerializeDescriptors;
use crate::tube::Error;
use crate::tube::RecvTube;
use crate::tube::Result;
use crate::tube::SendTube;
use crate::BlockingMode;
use crate::FramingMode;
use crate::RawDescriptor;
use crate::ReadNotifier;
use crate::ScmSocket;
use crate::StreamChannel;
use crate::UnixSeqpacket;

// This size matches the inline buffer size of CmsgBuffer.
const TUBE_MAX_FDS: usize = 32;

/// Bidirectional tube that support both send and recv.
#[derive(Serialize, Deserialize)]
pub struct Tube {
    socket: StreamChannel,
}

impl Tube {
    /// Create a pair of connected tubes. Request is sent in one direction while response is in the
    /// other direction.
    pub fn pair() -> Result<(Tube, Tube)> {
        let (socket1, socket2) = StreamChannel::pair(BlockingMode::Blocking, FramingMode::Message)
            .map_err(|errno| Error::Pair(std::io::Error::from(errno)))?;
        let tube1 = Tube::new(socket1)?;
        let tube2 = Tube::new(socket2)?;
        Ok((tube1, tube2))
    }

    /// Create a new `Tube` from a `StreamChannel`.
    /// The StreamChannel must use FramingMode::Message (meaning, must use a SOCK_SEQPACKET as the
    /// underlying socket type), otherwise, this method returns an error.
    pub fn new(socket: StreamChannel) -> Result<Tube> {
        match socket.get_framing_mode() {
            FramingMode::Message => Ok(Tube { socket }),
            FramingMode::Byte => Err(Error::InvalidFramingMode),
        }
    }

    /// Create a new `Tube` from a UnixSeqpacket. The StreamChannel is implicitly constructed to
    /// have the right FramingMode by being constructed from a UnixSeqpacket.
    pub fn new_from_unix_seqpacket(sock: UnixSeqpacket) -> Tube {
        Tube {
            socket: StreamChannel::from_unix_seqpacket(sock),
        }
    }

    /// DO NOT USE this method directly as it will become private soon (b/221484449). Use a
    /// directional Tube pair instead.
    #[deprecated]
    pub fn try_clone(&self) -> Result<Self> {
        self.socket
            .try_clone()
            .map(Tube::new)
            .map_err(Error::Clone)?
    }

    pub fn send<T: Serialize>(&self, msg: &T) -> Result<()> {
        let msg_serialize = SerializeDescriptors::new(&msg);
        let msg_json = serde_json::to_vec(&msg_serialize).map_err(Error::Json)?;
        let msg_descriptors = msg_serialize.into_descriptors();

        if msg_descriptors.len() > TUBE_MAX_FDS {
            return Err(Error::SendTooManyFds);
        }

        self.socket
            .send_with_fds(&[IoSlice::new(&msg_json)], &msg_descriptors)
            .map_err(Error::Send)?;
        Ok(())
    }

    pub fn recv<T: DeserializeOwned>(&self) -> Result<T> {
        let msg_size = self.socket.peek_size().map_err(Error::Recv)?;
        // This buffer is the right size, as the size received in peek_size() represents the size
        // of only the message itself and not the file descriptors. The descriptors are stored
        // separately in msghdr::msg_control.
        let mut msg_json = vec![0u8; msg_size];

        let mut msg_descriptors_full = [0; TUBE_MAX_FDS];

        let (msg_json_size, descriptor_size) = self
            .socket
            .recv_with_fds(IoSliceMut::new(&mut msg_json), &mut msg_descriptors_full)
            .map_err(Error::Send)?;

        if msg_json_size == 0 {
            return Err(Error::Disconnected);
        }

        let mut msg_descriptors_safe = msg_descriptors_full[..descriptor_size]
            .iter()
            .map(|v| {
                Some(unsafe {
                    // Safe because the socket returns new fds that are owned locally by this scope.
                    SafeDescriptor::from_raw_descriptor(*v)
                })
            })
            .collect();

        deserialize_with_descriptors(
            || serde_json::from_slice(&msg_json[0..msg_json_size]),
            &mut msg_descriptors_safe,
        )
        .map_err(Error::Json)
    }

    pub fn set_send_timeout(&self, timeout: Option<Duration>) -> Result<()> {
        self.socket
            .set_write_timeout(timeout)
            .map_err(Error::SetSendTimeout)
    }

    pub fn set_recv_timeout(&self, timeout: Option<Duration>) -> Result<()> {
        self.socket
            .set_read_timeout(timeout)
            .map_err(Error::SetRecvTimeout)
    }
}

impl AsRawDescriptor for Tube {
    fn as_raw_descriptor(&self) -> RawDescriptor {
        self.socket.as_raw_descriptor()
    }
}

impl AsRawFd for Tube {
    fn as_raw_fd(&self) -> RawFd {
        self.socket.as_raw_fd()
    }
}

impl ReadNotifier for Tube {
    fn get_read_notifier(&self) -> &dyn AsRawDescriptor {
        &self.socket
    }
}

impl FromRawDescriptor for Tube {
    /// # Safety:
    /// Requirements:
    /// (1) The caller owns rd.
    /// (2) When the call completes, ownership of rd has transferred to the returned value.
    unsafe fn from_raw_descriptor(rd: RawDescriptor) -> Self {
        Self {
            socket: StreamChannel::from_unix_seqpacket(UnixSeqpacket::from_raw_descriptor(rd)),
        }
    }
}

impl AsRawDescriptor for SendTube {
    fn as_raw_descriptor(&self) -> RawDescriptor {
        self.0.as_raw_descriptor()
    }
}

impl AsRawDescriptor for RecvTube {
    fn as_raw_descriptor(&self) -> RawDescriptor {
        self.0.as_raw_descriptor()
    }
}
