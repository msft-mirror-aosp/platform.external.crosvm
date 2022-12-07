// Copyright 2020 The ChromiumOS Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

//! Safe, cross-platform-compatible wrappers for system interfaces.

mod alloc;
mod clock;
pub mod descriptor;
pub mod descriptor_reflection;
mod errno;
mod event;
mod mmap;
mod notifiers;
mod shm;
pub mod syslog;
mod timer;
mod tube;
mod wait_context;
mod write_zeroes;

pub mod sys;
pub use alloc::LayoutAllocation;

pub use clock::Clock;
pub use clock::FakeClock;
pub use errno::errno_result;
pub use errno::Error;
pub use errno::Result;
pub use event::Event;
pub use event::EventWaitResult;
pub use mmap::ExternalMapping;
pub use mmap::MappedRegion;
pub use mmap::MemoryMapping;
pub use mmap::MemoryMappingBuilder;
pub use notifiers::CloseNotifier;
pub use notifiers::ReadNotifier;
pub use platform::ioctl::ioctl;
pub use platform::ioctl::ioctl_with_mut_ptr;
pub use platform::ioctl::ioctl_with_mut_ref;
pub use platform::ioctl::ioctl_with_ptr;
pub use platform::ioctl::ioctl_with_ref;
pub use platform::ioctl::ioctl_with_val;
pub use platform::ioctl::IoctlNr;
pub use shm::SharedMemory;
pub use sys::platform;
pub use timer::FakeTimer;
pub use timer::Timer;
pub use tube::Error as TubeError;
pub use tube::RecvTube;
pub use tube::Result as TubeResult;
pub use tube::SendTube;
pub use tube::Tube;
pub use wait_context::EventToken;
pub use wait_context::EventType;
pub use wait_context::TriggeredEvent;
pub use wait_context::WaitContext;
pub use write_zeroes::PunchHole;
pub use write_zeroes::WriteZeroesAt;

// TODO(b/233233301): reorganize platform specific exports under platform
// namespaces instead of exposing them directly in base::.
cfg_if::cfg_if! {
    if #[cfg(unix)] {
        pub use sys::unix;

        pub use unix::net;

        // File related exports.
        pub use platform::{FileFlags, get_max_open_files};

        // memory/mmap related exports.
        pub use platform::{
            MemfdSeals, MemoryMappingBuilderUnix, Unix as MemoryMappingUnix,
            SharedMemoryUnix,
        };

        // descriptor/fd related exports.
        pub use platform::{
            add_fd_flags, clear_fd_flags, clone_descriptor, safe_descriptor_from_path,
            validate_raw_descriptor, clear_descriptor_cloexec,
        };

        // Event/signal related exports.
        pub use platform::{
            block_signal, clear_signal, get_blocked_signals, new_pipe_full,
            register_rt_signal_handler, signal, unblock_signal, Killable, SIGRTMIN,
            AcpiNotifyEvent, NetlinkGenericSocket, SignalFd, Terminal,
        };

        pub use platform::{
            chown, drop_capabilities, iov_max, kernel_has_memfd, pipe, read_raw_stdin
        };
        pub use platform::{enable_core_scheduling, set_rt_prio_limit, set_rt_round_robin};
        pub use platform::{flock, FlockOperation};
        pub use platform::{getegid, geteuid};
        pub use platform::{gettid, kill_process_group, reap_child};
        pub use platform::{
            net::{UnixSeqpacket, UnixSeqpacketListener, UnlinkUnixSeqpacketListener},
            ScmSocket, UnlinkUnixListener, SCM_SOCKET_MAX_FD_COUNT,
        };
        pub use platform::EventExt;
    } else if #[cfg(windows)] {
        pub use platform::{EventTrigger, EventExt, WaitContextExt};
        pub use platform::MemoryMappingBuilderWindows;
        pub use platform::set_thread_priority;
        pub use platform::{give_foregrounding_permission, Console};
        pub use platform::{named_pipes, named_pipes::PipeConnection};
        pub use platform::{SafeMultimediaHandle, MAXIMUM_WAIT_OBJECTS};
        pub use crate::platform::win::{
            measure_timer_resolution, nt_query_timer_resolution, nt_set_timer_resolution,
            set_sparse_file, set_time_period,
        };
        pub use platform::ioctl::ioctl_with_ptr_sized;

        pub use tube::{
            deserialize_and_recv, serialize_and_send, set_alias_pid, set_duplicate_handle_tube,
            DuplicateHandleRequest, DuplicateHandleResponse, DuplicateHandleTube
        };
        #[cfg(feature = "kiwi")]
        pub use tube::ProtoTube;
        pub use platform::{set_audio_thread_priorities, thread};
    } else {
        compile_error!("Unsupported platform");
    }
}

pub use log::debug;
pub use log::error;
pub use log::info;
pub use log::trace;
pub use log::warn;
pub use mmap::Protection;
pub use platform::deserialize_with_descriptors;
pub(crate) use platform::file_punch_hole;
pub(crate) use platform::file_write_zeroes_at;
pub use platform::get_cpu_affinity;
pub use platform::get_filesystem_type;
pub use platform::getpid;
pub use platform::number_of_logical_cores;
pub use platform::open_file;
pub use platform::pagesize;
pub use platform::platform_timer_resolution::enable_high_res_timers;
pub use platform::round_up_to_page_size;
pub use platform::set_cpu_affinity;
pub use platform::with_as_descriptor;
pub use platform::with_raw_descriptor;
pub use platform::BlockingMode;
pub use platform::EventContext;
pub use platform::FileAllocate;
pub use platform::FileGetLen;
pub use platform::FileReadWriteAtVolatile;
pub use platform::FileReadWriteVolatile;
pub use platform::FileSerdeWrapper;
pub use platform::FileSetLen;
pub use platform::FileSync;
pub use platform::FramingMode;
pub use platform::MemoryMappingArena;
pub use platform::MmapError;
pub use platform::RawDescriptor;
pub use platform::SerializeDescriptors;
pub use platform::StreamChannel;
pub use platform::INVALID_DESCRIPTOR;
use uuid::Uuid;

pub use crate::descriptor::AsRawDescriptor;
pub use crate::descriptor::AsRawDescriptors;
pub use crate::descriptor::Descriptor;
pub use crate::descriptor::FromRawDescriptor;
pub use crate::descriptor::IntoRawDescriptor;
pub use crate::descriptor::SafeDescriptor;

/// An empty trait that helps reset timer resolution to its previous state.
// TODO(b:232103460): Maybe this needs to be thought through.
pub trait EnabledHighResTimer {}

/// Creates a UUID.
pub fn generate_uuid() -> String {
    let mut buf = Uuid::encode_buffer();
    Uuid::new_v4()
        .to_hyphenated()
        .encode_lower(&mut buf)
        .to_owned()
}

use serde::Deserialize;
use serde::Serialize;
#[derive(Clone, Copy, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum VmEventType {
    Exit,
    Reset,
    Crash,
    Panic(u8),
    WatchdogReset,
}
