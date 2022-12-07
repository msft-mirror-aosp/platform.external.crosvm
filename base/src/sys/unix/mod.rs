// Copyright 2017 The ChromiumOS Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

//! Small system utility modules for usage by other modules.

#[cfg(target_os = "android")]
mod android;
#[cfg(target_os = "android")]
use android as target_os;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
use linux as target_os;
use log::warn;
#[macro_use]
pub mod handle_eintr;
#[macro_use]
pub mod ioctl;
#[macro_use]
pub mod syslog;
mod acpi_event;
mod capabilities;
mod descriptor;
mod event;
mod file;
mod file_flags;
pub mod file_traits;
mod get_filesystem_type;
mod mmap;
pub mod net;
mod netlink;
mod notifiers;
pub mod panic_handler;
pub mod platform_timer_resolution;
mod poll;
mod priority;
pub mod process;
mod sched;
pub mod scoped_signal_handler;
mod shm;
pub mod signal;
mod signalfd;
mod sock_ctrl_msg;
mod stream_channel;
mod terminal;
mod timer;
pub mod vsock;
mod write_zeroes;

use std::ffi::CStr;
use std::fs::remove_file;
use std::fs::File;
use std::fs::OpenOptions;
use std::mem;
use std::ops::Deref;
use std::os::unix::io::AsRawFd;
use std::os::unix::io::FromRawFd;
use std::os::unix::io::RawFd;
use std::os::unix::net::UnixDatagram;
use std::os::unix::net::UnixListener;
use std::os::unix::process::ExitStatusExt;
use std::path::Path;
use std::process::ExitStatus;
use std::ptr;
use std::time::Duration;

pub use acpi_event::*;
pub use capabilities::drop_capabilities;
pub use descriptor::*;
pub use event::EventExt;
pub(crate) use event::PlatformEvent;
pub use file::FileDataIterator;
pub use file_flags::*;
pub use file_traits::AsRawFds;
pub use file_traits::FileAllocate;
pub use file_traits::FileGetLen;
pub use file_traits::FileReadWriteAtVolatile;
pub use file_traits::FileReadWriteVolatile;
pub use file_traits::FileSetLen;
pub use file_traits::FileSync;
pub use get_filesystem_type::*;
pub use ioctl::*;
use libc::c_int;
use libc::c_long;
use libc::fcntl;
use libc::pipe2;
use libc::syscall;
use libc::sysconf;
use libc::waitpid;
use libc::SYS_getpid;
use libc::SYS_getppid;
use libc::SYS_gettid;
use libc::EINVAL;
use libc::F_GETFL;
use libc::F_SETFL;
use libc::O_CLOEXEC;
pub(crate) use libc::PROT_READ;
pub(crate) use libc::PROT_WRITE;
use libc::SIGKILL;
use libc::WNOHANG;
use libc::_SC_IOV_MAX;
use libc::_SC_PAGESIZE;
pub use mmap::Error as MmapError;
pub use mmap::*;
pub use netlink::*;
pub use poll::EventContext;
pub use priority::*;
pub use sched::*;
pub use scoped_signal_handler::*;
pub use shm::kernel_has_memfd;
pub use shm::MemfdSeals;
pub use shm::SharedMemory;
pub use shm::Unix as SharedMemoryUnix;
pub use signal::*;
pub use signalfd::Error as SignalFdError;
pub use signalfd::*;
pub use sock_ctrl_msg::*;
pub use stream_channel::*;
pub use terminal::*;
pub use timer::*;
pub(crate) use write_zeroes::file_punch_hole;
pub(crate) use write_zeroes::file_write_zeroes_at;

use crate::descriptor::FromRawDescriptor;
use crate::descriptor::SafeDescriptor;
pub use crate::descriptor_reflection::deserialize_with_descriptors;
pub use crate::descriptor_reflection::with_as_descriptor;
pub use crate::descriptor_reflection::with_raw_descriptor;
pub use crate::descriptor_reflection::FileSerdeWrapper;
pub use crate::descriptor_reflection::SerializeDescriptors;
pub use crate::errno::Error;
pub use crate::errno::Result;
pub use crate::errno::*;

/// Re-export libc types that are part of the API.
pub type Pid = libc::pid_t;
pub type Uid = libc::uid_t;
pub type Gid = libc::gid_t;
pub type Mode = libc::mode_t;

#[macro_export]
macro_rules! syscall {
    ($e:expr) => {{
        let res = $e;
        if res < 0 {
            $crate::platform::errno_result()
        } else {
            Ok(res)
        }
    }};
}

/// Safe wrapper for `sysconf(_SC_PAGESIZE)`.
#[inline(always)]
pub fn pagesize() -> usize {
    // Trivially safe
    unsafe { sysconf(_SC_PAGESIZE) as usize }
}

/// Safe wrapper for `sysconf(_SC_IOV_MAX)`.
pub fn iov_max() -> usize {
    // Trivially safe
    unsafe { sysconf(_SC_IOV_MAX) as usize }
}

/// Uses the system's page size in bytes to round the given value up to the nearest page boundary.
#[inline(always)]
pub fn round_up_to_page_size(v: usize) -> usize {
    let page_mask = pagesize() - 1;
    (v + page_mask) & !page_mask
}

/// This bypasses `libc`'s caching `getpid(2)` wrapper which can be invalid if a raw clone was used
/// elsewhere.
#[inline(always)]
pub fn getpid() -> Pid {
    // Safe because this syscall can never fail and we give it a valid syscall number.
    unsafe { syscall(SYS_getpid as c_long) as Pid }
}

/// Safe wrapper for the geppid Linux systemcall.
#[inline(always)]
pub fn getppid() -> Pid {
    // Safe because this syscall can never fail and we give it a valid syscall number.
    unsafe { syscall(SYS_getppid as c_long) as Pid }
}

/// Safe wrapper for the gettid Linux systemcall.
pub fn gettid() -> Pid {
    // Calling the gettid() sycall is always safe.
    unsafe { syscall(SYS_gettid as c_long) as Pid }
}

/// Safe wrapper for `getsid(2)`.
pub fn getsid(pid: Option<Pid>) -> Result<Pid> {
    // Calling the getsid() sycall is always safe.
    syscall!(unsafe { libc::getsid(pid.unwrap_or(0)) } as Pid)
}

/// Wrapper for `setsid(2)`.
pub fn setsid() -> Result<Pid> {
    // Safe because the return code is checked.
    syscall!(unsafe { libc::setsid() as Pid })
}

/// Safe wrapper for `geteuid(2)`.
#[inline(always)]
pub fn geteuid() -> Uid {
    // trivially safe
    unsafe { libc::geteuid() }
}

/// Safe wrapper for `getegid(2)`.
#[inline(always)]
pub fn getegid() -> Gid {
    // trivially safe
    unsafe { libc::getegid() }
}

/// Safe wrapper for chown(2).
#[inline(always)]
pub fn chown(path: &CStr, uid: Uid, gid: Gid) -> Result<()> {
    // Safe since we pass in a valid string pointer and check the return value.
    syscall!(unsafe { libc::chown(path.as_ptr(), uid, gid) }).map(|_| ())
}

/// Safe wrapper for fchmod(2).
#[inline(always)]
pub fn fchmod<A: AsRawFd>(fd: &A, mode: Mode) -> Result<()> {
    // Safe since the function does not operate on pointers and check the return value.
    syscall!(unsafe { libc::fchmod(fd.as_raw_fd(), mode) }).map(|_| ())
}

/// Safe wrapper for fchown(2).
#[inline(always)]
pub fn fchown<A: AsRawFd>(fd: &A, uid: Uid, gid: Gid) -> Result<()> {
    // Safe since the function does not operate on pointers and check the return value.
    syscall!(unsafe { libc::fchown(fd.as_raw_fd(), uid, gid) }).map(|_| ())
}

/// The operation to perform with `flock`.
pub enum FlockOperation {
    LockShared,
    LockExclusive,
    Unlock,
}

/// Safe wrapper for flock(2) with the operation `op` and optionally `nonblocking`. The lock will be
/// dropped automatically when `file` is dropped.
#[inline(always)]
pub fn flock(file: &dyn AsRawFd, op: FlockOperation, nonblocking: bool) -> Result<()> {
    let mut operation = match op {
        FlockOperation::LockShared => libc::LOCK_SH,
        FlockOperation::LockExclusive => libc::LOCK_EX,
        FlockOperation::Unlock => libc::LOCK_UN,
    };

    if nonblocking {
        operation |= libc::LOCK_NB;
    }

    // Safe since we pass in a valid fd and flock operation, and check the return value.
    syscall!(unsafe { libc::flock(file.as_raw_fd(), operation) }).map(|_| ())
}

/// The operation to perform with `fallocate`.
pub enum FallocateMode {
    PunchHole,
    ZeroRange,
    Allocate,
}

/// Safe wrapper for `fallocate()`.
pub fn fallocate(
    file: &dyn AsRawFd,
    mode: FallocateMode,
    keep_size: bool,
    offset: u64,
    len: u64,
) -> Result<()> {
    let offset = if offset > libc::off64_t::max_value() as u64 {
        return Err(Error::new(libc::EINVAL));
    } else {
        offset as libc::off64_t
    };

    let len = if len > libc::off64_t::max_value() as u64 {
        return Err(Error::new(libc::EINVAL));
    } else {
        len as libc::off64_t
    };

    let mut mode = match mode {
        FallocateMode::PunchHole => libc::FALLOC_FL_PUNCH_HOLE,
        FallocateMode::ZeroRange => libc::FALLOC_FL_ZERO_RANGE,
        FallocateMode::Allocate => 0,
    };

    if keep_size {
        mode |= libc::FALLOC_FL_KEEP_SIZE;
    }

    // Safe since we pass in a valid fd and fallocate mode, validate offset and len,
    // and check the return value.
    syscall!(unsafe { libc::fallocate64(file.as_raw_fd(), mode, offset, len) }).map(|_| ())
}

/// A trait used to abstract types that provide a process id that can be operated on.
pub trait AsRawPid {
    fn as_raw_pid(&self) -> Pid;
}

impl AsRawPid for Pid {
    fn as_raw_pid(&self) -> Pid {
        *self
    }
}

impl AsRawPid for std::process::Child {
    fn as_raw_pid(&self) -> Pid {
        self.id() as Pid
    }
}

/// A safe wrapper around waitpid.
///
/// On success if a process was reaped, it will be returned as the first value.
/// The second returned value is the ExitStatus from the libc::waitpid() call.
///
/// Note: this can block if libc::WNOHANG is not set and EINTR is not handled internally.
pub fn wait_for_pid<A: AsRawPid>(pid: A, options: c_int) -> Result<(Option<Pid>, ExitStatus)> {
    let pid = pid.as_raw_pid();
    let mut status: c_int = 1;
    // Safe because status is owned and the error is checked.
    let ret = unsafe { libc::waitpid(pid, &mut status, options) };
    if ret < 0 {
        return errno_result();
    }
    Ok((
        if ret == 0 { None } else { Some(ret) },
        ExitStatus::from_raw(status),
    ))
}

/// Reaps a child process that has terminated.
///
/// Returns `Ok(pid)` where `pid` is the process that was reaped or `Ok(0)` if none of the children
/// have terminated. An `Error` is with `errno == ECHILD` if there are no children left to reap.
///
/// # Examples
///
/// Reaps all child processes until there are no terminated children to reap.
///
/// ```
/// fn reap_children() {
///     loop {
///         match base::platform::reap_child() {
///             Ok(0) => println!("no children ready to reap"),
///             Ok(pid) => {
///                 println!("reaped {}", pid);
///                 continue
///             },
///             Err(e) if e.errno() == libc::ECHILD => println!("no children left"),
///             Err(e) => println!("error reaping children: {}", e),
///         }
///         break
///     }
/// }
/// ```
pub fn reap_child() -> Result<Pid> {
    // Safe because we pass in no memory, prevent blocking with WNOHANG, and check for error.
    let ret = unsafe { waitpid(-1, ptr::null_mut(), WNOHANG) };
    if ret == -1 {
        errno_result()
    } else {
        Ok(ret)
    }
}

/// Kill all processes in the current process group.
///
/// On success, this kills all processes in the current process group, including the current
/// process, meaning this will not return. This is equivalent to a call to `kill(0, SIGKILL)`.
pub fn kill_process_group() -> Result<()> {
    unsafe { kill(0, SIGKILL) }?;
    // Kill succeeded, so this process never reaches here.
    unreachable!();
}

/// Spawns a pipe pair where the first pipe is the read end and the second pipe is the write end.
///
/// If `close_on_exec` is true, the `O_CLOEXEC` flag will be set during pipe creation.
pub fn pipe(close_on_exec: bool) -> Result<(File, File)> {
    let flags = if close_on_exec { O_CLOEXEC } else { 0 };
    let mut pipe_fds = [-1; 2];
    // Safe because pipe2 will only write 2 element array of i32 to the given pointer, and we check
    // for error.
    let ret = unsafe { pipe2(&mut pipe_fds[0], flags) };
    if ret == -1 {
        errno_result()
    } else {
        // Safe because both fds must be valid for pipe2 to have returned sucessfully and we have
        // exclusive ownership of them.
        Ok(unsafe {
            (
                File::from_raw_fd(pipe_fds[0]),
                File::from_raw_fd(pipe_fds[1]),
            )
        })
    }
}

/// Sets the pipe signified with fd to `size`.
///
/// Returns the new size of the pipe or an error if the OS fails to set the pipe size.
pub fn set_pipe_size(fd: RawFd, size: usize) -> Result<usize> {
    // Safe because fcntl with the `F_SETPIPE_SZ` arg doesn't touch memory.
    syscall!(unsafe { fcntl(fd, libc::F_SETPIPE_SZ, size as c_int) }).map(|ret| ret as usize)
}

/// Test-only function used to create a pipe that is full. The pipe is created, has its size set to
/// the minimum and then has that much data written to it. Use `new_pipe_full` to test handling of
/// blocking `write` calls in unit tests.
pub fn new_pipe_full() -> Result<(File, File)> {
    use std::io::Write;

    let (rx, mut tx) = pipe(true)?;
    // The smallest allowed size of a pipe is the system page size on linux.
    let page_size = set_pipe_size(tx.as_raw_fd(), round_up_to_page_size(1))?;

    // Fill the pipe with page_size zeros so the next write call will block.
    let buf = vec![0u8; page_size];
    tx.write_all(&buf)?;

    Ok((rx, tx))
}

/// Used to attempt to clean up a named pipe after it is no longer used.
pub struct UnlinkUnixDatagram(pub UnixDatagram);
impl AsRef<UnixDatagram> for UnlinkUnixDatagram {
    fn as_ref(&self) -> &UnixDatagram {
        &self.0
    }
}
impl Drop for UnlinkUnixDatagram {
    fn drop(&mut self) {
        if let Ok(addr) = self.0.local_addr() {
            if let Some(path) = addr.as_pathname() {
                if let Err(e) = remove_file(path) {
                    warn!("failed to remove control socket file: {}", e);
                }
            }
        }
    }
}

/// Used to attempt to clean up a named pipe after it is no longer used.
pub struct UnlinkUnixListener(pub UnixListener);

impl AsRef<UnixListener> for UnlinkUnixListener {
    fn as_ref(&self) -> &UnixListener {
        &self.0
    }
}

impl Deref for UnlinkUnixListener {
    type Target = UnixListener;

    fn deref(&self) -> &UnixListener {
        &self.0
    }
}

impl Drop for UnlinkUnixListener {
    fn drop(&mut self) {
        if let Ok(addr) = self.0.local_addr() {
            if let Some(path) = addr.as_pathname() {
                if let Err(e) = remove_file(path) {
                    warn!("failed to remove control socket file: {}", e);
                }
            }
        }
    }
}

/// Verifies that |raw_descriptor| is actually owned by this process and duplicates it
/// to ensure that we have a unique handle to it.
pub fn validate_raw_descriptor(raw_descriptor: RawDescriptor) -> Result<RawDescriptor> {
    validate_raw_fd(raw_descriptor)
}

/// Verifies that |raw_fd| is actually owned by this process and duplicates it to ensure that
/// we have a unique handle to it.
pub fn validate_raw_fd(raw_fd: RawFd) -> Result<RawFd> {
    // Checking that close-on-exec isn't set helps filter out FDs that were opened by
    // crosvm as all crosvm FDs are close on exec.
    // Safe because this doesn't modify any memory and we check the return value.
    let flags = unsafe { libc::fcntl(raw_fd, libc::F_GETFD) };
    if flags < 0 || (flags & libc::FD_CLOEXEC) != 0 {
        return Err(Error::new(libc::EBADF));
    }

    // Duplicate the fd to ensure that we don't accidentally close an fd previously
    // opened by another subsystem.  Safe because this doesn't modify any memory and
    // we check the return value.
    let dup_fd = unsafe { libc::fcntl(raw_fd, libc::F_DUPFD_CLOEXEC, 0) };
    if dup_fd < 0 {
        return Err(Error::last());
    }
    Ok(dup_fd as RawFd)
}

/// Utility function that returns true if the given FD is readable without blocking.
///
/// On an error, such as an invalid or incompatible FD, this will return false, which can not be
/// distinguished from a non-ready to read FD.
pub fn poll_in(fd: &dyn AsRawFd) -> bool {
    let mut fds = libc::pollfd {
        fd: fd.as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    };
    // Safe because we give a valid pointer to a list (of 1) FD and check the return value.
    let ret = unsafe { libc::poll(&mut fds, 1, 0) };
    // An error probably indicates an invalid FD, or an FD that can't be polled. Returning false in
    // that case is probably correct as such an FD is unlikely to be readable, although there are
    // probably corner cases in which that is wrong.
    if ret == -1 {
        return false;
    }
    fds.revents & libc::POLLIN != 0
}

/// Returns the file flags set for the given `RawFD`
///
/// Returns an error if the OS indicates the flags can't be retrieved.
fn get_fd_flags(fd: RawFd) -> Result<c_int> {
    // Safe because no third parameter is expected and we check the return result.
    syscall!(unsafe { fcntl(fd, F_GETFL) })
}

/// Sets the file flags set for the given `RawFD`.
///
/// Returns an error if the OS indicates the flags can't be retrieved.
fn set_fd_flags(fd: RawFd, flags: c_int) -> Result<()> {
    // Safe because we supply the third parameter and we check the return result.
    // fcntlt is trusted not to modify the memory of the calling process.
    syscall!(unsafe { fcntl(fd, F_SETFL, flags) }).map(|_| ())
}

/// Performs a logical OR of the given flags with the FD's flags, setting the given bits for the
/// FD.
///
/// Returns an error if the OS indicates the flags can't be retrieved or set.
pub fn add_fd_flags(fd: RawFd, set_flags: c_int) -> Result<()> {
    let start_flags = get_fd_flags(fd)?;
    set_fd_flags(fd, start_flags | set_flags)
}

/// Clears the given flags in the FD's flags.
///
/// Returns an error if the OS indicates the flags can't be retrieved or set.
pub fn clear_fd_flags(fd: RawFd, clear_flags: c_int) -> Result<()> {
    let start_flags = get_fd_flags(fd)?;
    set_fd_flags(fd, start_flags & !clear_flags)
}

/// Return a timespec filed with the specified Duration `duration`.
pub fn duration_to_timespec(duration: Duration) -> libc::timespec {
    // nsec always fits in i32 because subsec_nanos is defined to be less than one billion.
    let nsec = duration.subsec_nanos() as i32;
    libc::timespec {
        tv_sec: duration.as_secs() as libc::time_t,
        tv_nsec: nsec.into(),
    }
}

/// Return the maximum Duration that can be used with libc::timespec.
pub fn max_timeout() -> Duration {
    Duration::new(libc::time_t::max_value() as u64, 999999999)
}

/// If the given path is of the form /proc/self/fd/N for some N, returns `Ok(Some(N))`. Otherwise
/// returns `Ok(None)`.
pub fn safe_descriptor_from_path<P: AsRef<Path>>(path: P) -> Result<Option<SafeDescriptor>> {
    let path = path.as_ref();
    if path.parent() == Some(Path::new("/proc/self/fd")) {
        let raw_descriptor = path
            .file_name()
            .and_then(|fd_osstr| fd_osstr.to_str())
            .and_then(|fd_str| fd_str.parse::<RawFd>().ok())
            .ok_or_else(|| Error::new(EINVAL))?;
        let validated_fd = validate_raw_fd(raw_descriptor)?;
        Ok(Some(
            // Safe because nothing else has access to validated_fd after this call.
            unsafe { SafeDescriptor::from_raw_descriptor(validated_fd) },
        ))
    } else {
        Ok(None)
    }
}

/// Open the file with the given path, or if it is of the form `/proc/self/fd/N` then just use the
/// file descriptor.
///
/// Note that this will not work properly if the same `/proc/self/fd/N` path is used twice in
/// different places, as the metadata (including the offset) will be shared between both file
/// descriptors.
pub fn open_file<P: AsRef<Path>>(path: P, options: &OpenOptions) -> Result<File> {
    let path = path.as_ref();
    // Special case '/proc/self/fd/*' paths. The FD is already open, just use it.
    Ok(if let Some(fd) = safe_descriptor_from_path(path)? {
        fd.into()
    } else {
        options.open(path)?
    })
}

/// Get the max number of open files allowed by the environment.
pub fn get_max_open_files() -> Result<u64> {
    let mut buf = mem::MaybeUninit::<libc::rlimit64>::zeroed();

    // Safe because this will only modify `buf` and we check the return value.
    let res = unsafe { libc::prlimit64(0, libc::RLIMIT_NOFILE, ptr::null(), buf.as_mut_ptr()) };
    if res == 0 {
        // Safe because the kernel guarantees that the struct is fully initialized.
        let limit = unsafe { buf.assume_init() };
        Ok(limit.rlim_max)
    } else {
        errno_result()
    }
}

/// Returns the number of online logical cores on the system.
pub fn number_of_logical_cores() -> Result<usize> {
    // Safe because we pass a flag for this call and the host supports this system call
    Ok(unsafe { libc::sysconf(libc::_SC_NPROCESSORS_CONF) } as usize)
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;

    #[test]
    fn pipe_size_and_fill() {
        let (_rx, mut tx) = new_pipe_full().expect("Failed to pipe");

        // To  check that setting the size worked, set the descriptor to non blocking and check that
        // write returns an error.
        add_fd_flags(tx.as_raw_fd(), libc::O_NONBLOCK).expect("Failed to set tx non blocking");
        tx.write(&[0u8; 8])
            .expect_err("Write after fill didn't fail");
    }
}
