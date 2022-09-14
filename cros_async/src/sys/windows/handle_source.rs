// Copyright 2020 The ChromiumOS Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use std::fs::File;
use std::io;
use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;
use std::io::Write;
use std::mem::ManuallyDrop;
use std::ptr::null_mut;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use base::error;
use base::warn;
use base::AsRawDescriptor;
use base::Descriptor;
use base::Error as SysUtilError;
use base::FileReadWriteAtVolatile;
use base::FileReadWriteVolatile;
use base::FromRawDescriptor;
use base::PunchHole;
use base::WriteZeroesAt;
use data_model::VolatileSlice;
use smallvec::SmallVec;
use sync::Mutex;
use thiserror::Error as ThisError;
use winapi::um::ioapiset::CancelIoEx;
use winapi::um::processthreadsapi::GetCurrentThreadId;

use crate::io_ext::AllocateMode;
use crate::mem::BackingMemory;
use crate::mem::MemRegion;
use crate::AsyncError;
use crate::AsyncResult;
use crate::CancellableBlockingPool;
use crate::IoSourceExt;
use crate::ReadAsync;
use crate::WriteAsync;

#[derive(ThisError, Debug)]
pub enum Error {
    #[error("An error occurred trying to seek: {0}.")]
    IoSeekError(io::Error),
    #[error("An error occurred trying to read: {0}.")]
    IoReadError(io::Error),
    #[error("An error occurred trying to write: {0}.")]
    IoWriteError(io::Error),
    #[error("An error occurred trying to flush: {0}.")]
    IoFlushError(io::Error),
    #[error("An error occurred trying to punch hole: {0}.")]
    IoPunchHoleError(io::Error),
    #[error("An error occurred trying to write zeroes: {0}.")]
    IoWriteZeroesError(io::Error),
    #[error("An error occurred trying to duplicate source handles: {0}.")]
    HandleDuplicationFailed(io::Error),
    #[error("An error occurred trying to wait on source handles: {0}.")]
    HandleWaitFailed(base::Error),
    #[error("An error occurred trying to get a VolatileSlice into BackingMemory: {0}.")]
    BackingMemoryVolatileSliceFetchFailed(crate::mem::Error),
    #[error("HandleSource is gone, so no handles are available to fulfill the IO request.")]
    NoHandleSource,
    #[error("Operation on HandleSource is cancelled.")]
    OperationCancelled,
    #[error("Operation on HandleSource was aborted (unexpected).")]
    OperationAborted,
}

impl From<Error> for io::Error {
    fn from(e: Error) -> Self {
        use Error::*;
        match e {
            IoSeekError(e) => e,
            IoReadError(e) => e,
            IoWriteError(e) => e,
            IoFlushError(e) => e,
            IoPunchHoleError(e) => e,
            IoWriteZeroesError(e) => e,
            HandleDuplicationFailed(e) => e,
            HandleWaitFailed(e) => e.into(),
            BackingMemoryVolatileSliceFetchFailed(e) => io::Error::new(io::ErrorKind::Other, e),
            NoHandleSource => io::Error::new(io::ErrorKind::Other, NoHandleSource),
            OperationCancelled => io::Error::new(io::ErrorKind::Interrupted, OperationCancelled),
            OperationAborted => io::Error::new(io::ErrorKind::Interrupted, OperationAborted),
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;

/// Used to shutdown IO running on a CancellableBlockingPool.
pub struct HandleWrapper {
    handles: Vec<Descriptor>,
}

impl HandleWrapper {
    pub fn new(handles: Vec<Descriptor>) -> Arc<Mutex<HandleWrapper>> {
        Arc::new(Mutex::new(Self { handles }))
    }

    pub fn cancel_sync_io<T>(&mut self, ret: T) -> T {
        for handle in &self.handles {
            // There isn't much we can do if cancel fails.
            if unsafe { CancelIoEx(handle.as_raw_descriptor(), null_mut()) } == 0 {
                warn!(
                    "Cancel IO for handle:{:?} failed with {}",
                    handle.as_raw_descriptor(),
                    SysUtilError::last()
                );
            }
        }
        ret
    }
}

/// Async IO source for Windows that uses a multi-threaded, multi-handle approach to provide fast IO
/// operations. It demuxes IO requests across a set of handles that refer to the same underlying IO
/// source, such as a file, and executes those requests across multiple threads. Benchmarks show
/// that this is the fastest method to perform IO on Windows, especially for file reads.
pub struct HandleSource<F: AsRawDescriptor> {
    sources: Box<[F]>,
    source_descriptors: Vec<Descriptor>,
    blocking_pool: CancellableBlockingPool,
}

impl<F: AsRawDescriptor> HandleSource<F> {
    /// Create a new `HandleSource` from the given IO source.
    ///
    /// Each HandleSource uses its own thread pool, with one thread per source supplied. Since these
    /// threads are generally idle because they're waiting on blocking IO, so the cost is minimal.
    /// Long term, we may migrate away from this approach toward IOCP or overlapped IO.
    ///
    /// WARNING: every `source` in `sources` MUST be a unique file object (e.g. separate handles
    /// each created by CreateFile), and point at the same file on disk. This is because IO
    /// operations on the HandleSource are randomly distributed to each source.
    ///
    /// # Safety
    /// The caller must guarantee that `F`'s handle is compatible with the underlying functions
    /// exposed on `HandleSource`. The behavior when calling unsupported functions is not defined
    /// by this struct. Note that most winapis will fail with reasonable errors.
    pub fn new(sources: Box<[F]>) -> Result<Self> {
        let source_count = sources.len();
        let mut source_descriptors = Vec::with_capacity(source_count);

        // Safe because consumers of the descriptors are tied to the lifetime of HandleSource.
        for source in sources.iter() {
            source_descriptors.push(Descriptor(source.as_raw_descriptor()));
        }

        Ok(Self {
            sources,
            source_descriptors,
            blocking_pool: CancellableBlockingPool::new(
                // WARNING: this is a safety requirement! Threads are 1:1 with sources.
                source_count,
                Duration::from_secs(10),
            ),
        })
    }

    #[inline]
    fn get_slices(
        mem: &Arc<dyn BackingMemory + Send + Sync>,
        mem_offsets: Vec<MemRegion>,
    ) -> Result<SmallVec<[VolatileSlice; 16]>> {
        mem_offsets
            .into_iter()
            .map(|region| {
                mem.get_volatile_slice(region)
                    .map_err(Error::BackingMemoryVolatileSliceFetchFailed)
            })
            .collect::<Result<SmallVec<[VolatileSlice; 16]>>>()
    }

    // Returns a copy of all the source handles as a vector of descriptors.
    fn as_descriptors(&self) -> Vec<Descriptor> {
        self.sources
            .iter()
            .map(|i| Descriptor(i.as_raw_descriptor()))
            .collect()
    }
}

impl<F: AsRawDescriptor> Drop for HandleSource<F> {
    fn drop(&mut self) {
        if let Err(e) = self.blocking_pool.shutdown() {
            error!("failed to clean up HandleSource: {}", e);
        }
    }
}

fn get_thread_file(descriptors: Vec<Descriptor>) -> ManuallyDrop<File> {
    // Safe because all callers must exit *before* these handles will be closed (guaranteed by
    // HandleSource's Drop impl.).
    unsafe {
        ManuallyDrop::new(File::from_raw_descriptor(
            descriptors[GetCurrentThreadId() as usize % descriptors.len()].0,
        ))
    }
}

#[async_trait(?Send)]
impl<F: AsRawDescriptor> ReadAsync for HandleSource<F> {
    /// Reads from the iosource at `file_offset` and fill the given `vec`.
    async fn read_to_vec<'a>(
        &'a self,
        file_offset: Option<u64>,
        mut vec: Vec<u8>,
    ) -> AsyncResult<(usize, Vec<u8>)> {
        let handles = HandleWrapper::new(self.as_descriptors());
        let descriptors = self.source_descriptors.clone();

        self.blocking_pool
            .spawn(
                move || {
                    let mut file = get_thread_file(descriptors);
                    if let Some(file_offset) = file_offset {
                        file.seek(SeekFrom::Start(file_offset))
                            .map_err(Error::IoSeekError)?;
                    }
                    Ok((
                        file.read(vec.as_mut_slice()).map_err(Error::IoReadError)?,
                        vec,
                    ))
                },
                move || Err(handles.lock().cancel_sync_io(Error::OperationCancelled)),
            )
            .await
            .map_err(AsyncError::HandleSource)
    }

    /// Reads to the given `mem` at the given offsets from the file starting at `file_offset`.
    async fn read_to_mem<'a>(
        &'a self,
        file_offset: Option<u64>,
        mem: Arc<dyn BackingMemory + Send + Sync>,
        mem_offsets: &'a [MemRegion],
    ) -> AsyncResult<usize> {
        let mem_offsets = mem_offsets.to_owned();
        let handles = HandleWrapper::new(self.as_descriptors());
        let descriptors = self.source_descriptors.clone();

        self.blocking_pool
            .spawn(
                move || {
                    let mut file = get_thread_file(descriptors);
                    let memory_slices = Self::get_slices(&mem, mem_offsets)?;

                    match file_offset {
                        Some(file_offset) => file
                            .read_vectored_at_volatile(memory_slices.as_slice(), file_offset)
                            .map_err(Error::IoReadError),
                        None => file
                            .read_vectored_volatile(memory_slices.as_slice())
                            .map_err(Error::IoReadError),
                    }
                },
                move || Err(handles.lock().cancel_sync_io(Error::OperationCancelled)),
            )
            .await
            .map_err(AsyncError::HandleSource)
    }

    /// Wait for the handle of `self` to be readable.
    async fn wait_readable(&self) -> AsyncResult<()> {
        unimplemented!()
    }

    /// Reads a single u64 from the current offset.
    async fn read_u64(&self) -> AsyncResult<u64> {
        unimplemented!()
    }
}

#[async_trait(?Send)]
impl<F: AsRawDescriptor> WriteAsync for HandleSource<F> {
    /// Writes from the given `vec` to the file starting at `file_offset`.
    async fn write_from_vec<'a>(
        &'a self,
        file_offset: Option<u64>,
        vec: Vec<u8>,
    ) -> AsyncResult<(usize, Vec<u8>)> {
        let handles = HandleWrapper::new(self.as_descriptors());
        let descriptors = self.source_descriptors.clone();

        self.blocking_pool
            .spawn(
                move || {
                    let mut file = get_thread_file(descriptors);
                    if let Some(file_offset) = file_offset {
                        file.seek(SeekFrom::Start(file_offset))
                            .map_err(Error::IoSeekError)?;
                    }
                    Ok((
                        file.write(vec.as_slice()).map_err(Error::IoWriteError)?,
                        vec,
                    ))
                },
                move || Err(handles.lock().cancel_sync_io(Error::OperationCancelled)),
            )
            .await
            .map_err(AsyncError::HandleSource)
    }

    /// Writes from the given `mem` from the given offsets to the file starting at `file_offset`.
    async fn write_from_mem<'a>(
        &'a self,
        file_offset: Option<u64>,
        mem: Arc<dyn BackingMemory + Send + Sync>,
        mem_offsets: &'a [MemRegion],
    ) -> AsyncResult<usize> {
        let mem_offsets = mem_offsets.to_owned();
        let handles = HandleWrapper::new(self.as_descriptors());
        let descriptors = self.source_descriptors.clone();

        self.blocking_pool
            .spawn(
                move || {
                    let mut file = get_thread_file(descriptors);
                    let memory_slices = Self::get_slices(&mem, mem_offsets)?;

                    match file_offset {
                        Some(file_offset) => file
                            .write_vectored_at_volatile(memory_slices.as_slice(), file_offset)
                            .map_err(Error::IoWriteError),
                        None => file
                            .write_vectored_volatile(memory_slices.as_slice())
                            .map_err(Error::IoWriteError),
                    }
                },
                move || Err(handles.lock().cancel_sync_io(Error::OperationCancelled)),
            )
            .await
            .map_err(AsyncError::HandleSource)
    }

    /// See `fallocate(2)`. Note this op is synchronous when using the Polled backend.
    async fn fallocate(&self, file_offset: u64, len: u64, mode: AllocateMode) -> AsyncResult<()> {
        let handles = HandleWrapper::new(self.as_descriptors());
        let descriptors = self.source_descriptors.clone();
        self.blocking_pool
            .spawn(
                move || {
                    let mut file = get_thread_file(descriptors);
                    match mode {
                        AllocateMode::PunchHole => {
                            file.punch_hole(file_offset, len)
                                .map_err(Error::IoPunchHoleError)?;
                        }
                        // ZeroRange calls `punch_hole` which doesn't extend the File size if it needs to.
                        // Will fix if it becomes a problem.
                        AllocateMode::ZeroRange => {
                            file.write_zeroes_at(file_offset, len as usize)
                                .map_err(Error::IoWriteZeroesError)?;
                        }
                    }
                    Ok(())
                },
                move || Err(handles.lock().cancel_sync_io(Error::OperationCancelled)),
            )
            .await
            .map_err(AsyncError::HandleSource)
    }

    /// Sync all completed write operations to the backing storage.
    async fn fsync(&self) -> AsyncResult<()> {
        let handles = HandleWrapper::new(self.as_descriptors());
        let descriptors = self.source_descriptors.clone();

        self.blocking_pool
            .spawn(
                move || {
                    let mut file = get_thread_file(descriptors);
                    file.flush().map_err(Error::IoFlushError)
                },
                move || Err(handles.lock().cancel_sync_io(Error::OperationCancelled)),
            )
            .await
            .map_err(AsyncError::HandleSource)
    }
}

/// Subtrait for general async IO. Some not supported on Windows when multiple
/// sources are present.
///
/// Note that on Windows w/ multiple sources these functions do not make sense.
/// TODO(nkgold): decide on what these should mean.
#[async_trait(?Send)]
impl<F: AsRawDescriptor> IoSourceExt<F> for HandleSource<F> {
    /// Yields the underlying IO source.
    fn into_source(self: Box<Self>) -> F {
        unimplemented!("`into_source` is not supported on Windows.")
    }

    /// Provides a mutable ref to the underlying IO source.
    fn as_source_mut(&mut self) -> &mut F {
        if self.sources.len() == 1 {
            return &mut self.sources[0];
        }
        // Unimplemented for multiple-source use-case
        unimplemented!(
            "`as_source_mut` doesn't support source len of {}",
            self.sources.len()
        )
    }

    /// Provides a ref to the underlying IO source.
    ///
    /// In the multi-source case, the 0th source will be returned. If sources are not
    /// interchangeable, behavior is undefined.
    fn as_source(&self) -> &F {
        return &self.sources[0];
    }

    async fn wait_for_handle(&self) -> AsyncResult<u64> {
        let waiter = super::WaitForHandle::new(self);
        match waiter.await {
            Err(e) => Err(AsyncError::HandleSource(e)),
            Ok(()) => Ok(0),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempfile;
    use tempfile::NamedTempFile;

    use super::super::HandleExecutor;
    use super::*;
    use crate::mem::VecIoWrapper;

    #[test]
    fn test_read_vec() {
        let mut f = tempfile().unwrap();
        f.write_all("data".as_bytes()).unwrap();
        f.flush().unwrap();
        f.seek(SeekFrom::Start(0)).unwrap();

        async fn read_data(handle_src: &HandleSource<File>) {
            let buf: Vec<u8> = vec![0; 4];
            let (bytes_read, buf) = handle_src.read_to_vec(Some(0), buf).await.unwrap();
            assert_eq!(bytes_read, 4);
            assert_eq!(std::str::from_utf8(buf.as_slice()).unwrap(), "data");
        }

        let ex = HandleExecutor::new();
        let handle_src = HandleSource::new(vec![f].into_boxed_slice()).unwrap();
        ex.run_until(read_data(&handle_src)).unwrap();
    }

    #[test]
    fn test_read_mem() {
        let mut f = tempfile().unwrap();
        f.write_all("data".as_bytes()).unwrap();
        f.flush().unwrap();
        f.seek(SeekFrom::Start(0)).unwrap();

        async fn read_data(handle_src: &HandleSource<File>) {
            let mem = Arc::new(VecIoWrapper::from(vec![0; 4]));
            let bytes_read = handle_src
                .read_to_mem(
                    Some(0),
                    Arc::<VecIoWrapper>::clone(&mem),
                    &[
                        MemRegion { offset: 0, len: 2 },
                        MemRegion { offset: 2, len: 2 },
                    ],
                )
                .await
                .unwrap();
            assert_eq!(bytes_read, 4);
            let vec: Vec<u8> = match Arc::try_unwrap(mem) {
                Ok(v) => v.into(),
                Err(_) => panic!("Too many vec refs"),
            };
            assert_eq!(std::str::from_utf8(vec.as_slice()).unwrap(), "data");
        }

        let ex = HandleExecutor::new();
        let handle_src = HandleSource::new(vec![f].into_boxed_slice()).unwrap();
        ex.run_until(read_data(&handle_src)).unwrap();
    }

    #[test]
    fn test_write_vec() {
        let mut temp_file = NamedTempFile::new().unwrap();

        async fn write_data(handle_src: &HandleSource<File>) {
            let mut buf: Vec<u8> = Vec::new();
            buf.extend_from_slice("data".as_bytes());

            let (bytes_written, _) = handle_src.write_from_vec(Some(0), buf).await.unwrap();
            assert_eq!(bytes_written, 4);
        }

        let ex = HandleExecutor::new();
        let f = fs::OpenOptions::new()
            .write(true)
            .open(temp_file.path())
            .unwrap();
        let handle_src = HandleSource::new(vec![f].into_boxed_slice()).unwrap();
        ex.run_until(write_data(&handle_src)).unwrap();

        let mut buf = vec![0; 4];
        temp_file.read_exact(&mut buf).unwrap();
        assert_eq!(std::str::from_utf8(buf.as_slice()).unwrap(), "data");
    }

    #[test]
    fn test_write_mem() {
        let mut temp_file = NamedTempFile::new().unwrap();

        async fn write_data(handle_src: &HandleSource<File>) {
            let mut buf: Vec<u8> = Vec::new();
            buf.extend_from_slice("data".as_bytes());
            let mem = Arc::new(VecIoWrapper::from(buf));
            let bytes_written = handle_src
                .write_from_mem(
                    Some(0),
                    Arc::<VecIoWrapper>::clone(&mem),
                    &[
                        MemRegion { offset: 0, len: 2 },
                        MemRegion { offset: 2, len: 2 },
                    ],
                )
                .await
                .unwrap();
            assert_eq!(bytes_written, 4);
            match Arc::try_unwrap(mem) {
                Ok(_) => (),
                Err(_) => panic!("Too many vec refs"),
            };
        }

        let ex = HandleExecutor::new();
        let f = fs::OpenOptions::new()
            .write(true)
            .open(temp_file.path())
            .unwrap();
        let handle_src = HandleSource::new(vec![f].into_boxed_slice()).unwrap();
        ex.run_until(write_data(&handle_src)).unwrap();

        let mut buf = vec![0; 4];
        temp_file.read_exact(&mut buf).unwrap();
        assert_eq!(std::str::from_utf8(buf.as_slice()).unwrap(), "data");
    }

    #[cfg_attr(all(target_os = "windows", target_env = "gnu"), ignore)]
    #[test]
    fn test_punch_holes() {
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all("abcdefghijk".as_bytes()).unwrap();
        temp_file.flush().unwrap();
        temp_file.seek(SeekFrom::Start(0)).unwrap();

        async fn punch_hole(handle_src: &HandleSource<File>) {
            let offset = 1;
            let len = 3;
            handle_src
                .fallocate(offset, len, AllocateMode::PunchHole)
                .await
                .unwrap();
        }

        let ex = HandleExecutor::new();
        let f = fs::OpenOptions::new()
            .write(true)
            .open(temp_file.path())
            .unwrap();
        let handle_src = HandleSource::new(vec![f].into_boxed_slice()).unwrap();
        ex.run_until(punch_hole(&handle_src)).unwrap();

        let mut buf = vec![0; 11];
        temp_file.read_exact(&mut buf).unwrap();
        assert_eq!(
            std::str::from_utf8(buf.as_slice()).unwrap(),
            "a\0\0\0efghijk"
        );
    }

    /// Test should fail because punch hole should not be allowed to allocate more memory
    #[cfg_attr(all(target_os = "windows", target_env = "gnu"), ignore)]
    #[test]
    fn test_punch_holes_fail_out_of_bounds() {
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all("abcdefghijk".as_bytes()).unwrap();
        temp_file.flush().unwrap();
        temp_file.seek(SeekFrom::Start(0)).unwrap();

        async fn punch_hole(handle_src: &HandleSource<File>) {
            let offset = 9;
            let len = 4;
            handle_src
                .fallocate(offset, len, AllocateMode::PunchHole)
                .await
                .unwrap();
        }

        let ex = HandleExecutor::new();
        let f = fs::OpenOptions::new()
            .write(true)
            .open(temp_file.path())
            .unwrap();
        let handle_src = HandleSource::new(vec![f].into_boxed_slice()).unwrap();
        ex.run_until(punch_hole(&handle_src)).unwrap();

        let mut buf = vec![0; 13];
        assert!(temp_file.read_exact(&mut buf).is_err());
    }

    // TODO(b/194338842): "ZeroRange" is supposed to allocate more memory if it goes out of the
    // bounds of the file. Determine if we need to support this, since Windows doesn't do this yet.
    // #[test]
    // fn test_write_zeroes() {
    //     let mut temp_file = NamedTempFile::new().unwrap();
    //     temp_file.write("abcdefghijk".as_bytes()).unwrap();
    //     temp_file.flush().unwrap();
    //     temp_file.seek(SeekFrom::Start(0)).unwrap();

    //     async fn punch_hole(handle_src: &HandleSource<File>) {
    //         let offset = 9;
    //         let len = 4;
    //         handle_src
    //             .fallocate(offset, len, AllocateMode::ZeroRange)
    //             .await
    //             .unwrap();
    //     }

    //     let ex = HandleExecutor::new();
    //     let f = fs::OpenOptions::new()
    //         .write(true)
    //         .open(temp_file.path())
    //         .unwrap();
    //     let handle_src = HandleSource::new(vec![f].into_boxed_slice()).unwrap();
    //     ex.run_until(punch_hole(&handle_src)).unwrap();

    //     let mut buf = vec![0; 13];
    //     temp_file.read_exact(&mut buf).unwrap();
    //     assert_eq!(
    //         std::str::from_utf8(buf.as_slice()).unwrap(),
    //         "abcdefghi\0\0\0\0"
    //     );
    // }
}
