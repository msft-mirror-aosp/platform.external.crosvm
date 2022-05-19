// Copyright 2017 The Chromium OS Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use std::io::{self, Write};
use std::mem::size_of;
use std::path::PathBuf;
use std::result;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use std::u32;

use base::Error as SysError;
use base::Result as SysResult;
use base::{
    error, info, warn, AsRawDescriptor, Event, PollToken, RawDescriptor, Timer, Tube, WaitContext,
};
use data_model::DataInit;
use disk::DiskFile;

use remain::sorted;
use serde::{Deserialize, Deserializer};
use sync::Mutex;
use thiserror::Error;
use vm_control::{DiskControlCommand, DiskControlResult};
use vm_memory::GuestMemory;

use super::common::*;
use crate::virtio::{
    block::sys::*, copy_config, DescriptorChain, DescriptorError, DeviceType, Interrupt, Queue,
    Reader, SignalableInterrupt, VirtioDevice, Writer,
};

const QUEUE_SIZE: u16 = 256;
const QUEUE_SIZES: &[u16] = &[QUEUE_SIZE];
const NUM_QUEUES: u16 = 1;

#[sorted]
#[derive(Error, Debug)]
enum ExecuteError {
    #[error("failed to copy ID string: {0}")]
    CopyId(io::Error),
    #[error("virtio descriptor error: {0}")]
    Descriptor(DescriptorError),
    #[error("failed to perform discard or write zeroes; sector={sector} num_sectors={num_sectors} flags={flags}; {ioerr:?}")]
    DiscardWriteZeroes {
        ioerr: Option<io::Error>,
        sector: u64,
        num_sectors: u32,
        flags: u32,
    },
    /// Error arming the flush timer.
    #[error("failed to flush: {0}")]
    Flush(io::Error),
    #[error("not enough space in descriptor chain to write status")]
    MissingStatus,
    #[error("out of range")]
    OutOfRange,
    #[error("failed to read message: {0}")]
    Read(io::Error),
    #[error("io error reading {length} bytes from sector {sector}: {desc_error}")]
    ReadIo {
        length: usize,
        sector: u64,
        desc_error: io::Error,
    },
    #[error("read only; request_type={request_type}")]
    ReadOnly { request_type: u32 },
    #[error("timer error: {0}")]
    Timer(SysError),
    #[error("unsupported ({0})")]
    Unsupported(u32),
    #[error("io error writing {length} bytes to sector {sector}: {desc_error}")]
    WriteIo {
        length: usize,
        sector: u64,
        desc_error: io::Error,
    },
    #[error("failed to write request status: {0}")]
    WriteStatus(io::Error),
}

impl ExecuteError {
    fn status(&self) -> u8 {
        match self {
            ExecuteError::CopyId(_) => VIRTIO_BLK_S_IOERR,
            ExecuteError::Descriptor(_) => VIRTIO_BLK_S_IOERR,
            ExecuteError::Read(_) => VIRTIO_BLK_S_IOERR,
            ExecuteError::WriteStatus(_) => VIRTIO_BLK_S_IOERR,
            ExecuteError::Flush(_) => VIRTIO_BLK_S_IOERR,
            ExecuteError::ReadIo { .. } => VIRTIO_BLK_S_IOERR,
            ExecuteError::Timer(_) => VIRTIO_BLK_S_IOERR,
            ExecuteError::WriteIo { .. } => VIRTIO_BLK_S_IOERR,
            ExecuteError::DiscardWriteZeroes { .. } => VIRTIO_BLK_S_IOERR,
            ExecuteError::ReadOnly { .. } => VIRTIO_BLK_S_IOERR,
            ExecuteError::OutOfRange { .. } => VIRTIO_BLK_S_IOERR,
            ExecuteError::MissingStatus => VIRTIO_BLK_S_IOERR,
            ExecuteError::Unsupported(_) => VIRTIO_BLK_S_UNSUPP,
        }
    }
}

fn block_option_sparse_default() -> bool {
    true
}
fn block_option_block_size_default() -> u32 {
    512
}

/// Maximum length of a `DiskOption` identifier.
///
/// This is based on the virtio-block ID length limit.
pub const DISK_ID_LEN: usize = 20;

fn deserialize_disk_id<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<[u8; DISK_ID_LEN]>, D::Error> {
    let id = String::deserialize(deserializer)?;

    if id.len() > DISK_ID_LEN {
        return Err(serde::de::Error::custom(format!(
            "disk id must be {} or fewer characters",
            DISK_ID_LEN
        )));
    }

    let mut ret = [0u8; DISK_ID_LEN];
    // Slicing id to value's length will never panic
    // because we checked that value will fit into id above.
    ret[..id.len()].copy_from_slice(id.as_bytes());
    Ok(Some(ret))
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DiskOption {
    pub path: PathBuf,
    #[serde(default, rename = "ro")]
    pub read_only: bool,
    #[serde(default = "block_option_sparse_default")]
    pub sparse: bool,
    #[serde(default)]
    pub o_direct: bool,
    #[serde(default = "block_option_block_size_default")]
    pub block_size: u32,
    #[serde(default, deserialize_with = "deserialize_disk_id")]
    pub id: Option<[u8; DISK_ID_LEN]>,
}

struct Worker {
    interrupt: Interrupt,
    queues: Vec<Queue>,
    mem: GuestMemory,
    disk_image: Box<dyn DiskFile>,
    disk_size: Arc<Mutex<u64>>,
    read_only: bool,
    sparse: bool,
    id: Option<BlockId>,
    control_tube: Option<Tube>,
}

impl Worker {
    fn process_one_request(
        avail_desc: DescriptorChain,
        read_only: bool,
        sparse: bool,
        disk: &mut dyn DiskFile,
        disk_size: u64,
        id: Option<BlockId>,
        flush_timer: &mut Timer,
        flush_timer_armed: &mut bool,
        mem: &GuestMemory,
    ) -> result::Result<usize, ExecuteError> {
        let mut reader =
            Reader::new(mem.clone(), avail_desc.clone()).map_err(ExecuteError::Descriptor)?;
        let mut writer = Writer::new(mem.clone(), avail_desc).map_err(ExecuteError::Descriptor)?;

        // The last byte of the buffer is virtio_blk_req::status.
        // Split it into a separate Writer so that status_writer is the final byte and
        // the original writer is left with just the actual block I/O data.
        let available_bytes = writer.available_bytes();
        let status_offset = available_bytes
            .checked_sub(1)
            .ok_or(ExecuteError::MissingStatus)?;
        let mut status_writer = writer.split_at(status_offset);

        let status = match Block::execute_request(
            &mut reader,
            &mut writer,
            read_only,
            sparse,
            disk,
            disk_size,
            id,
            flush_timer,
            flush_timer_armed,
        ) {
            Ok(()) => VIRTIO_BLK_S_OK,
            Err(e) => {
                if !matches!(e, ExecuteError::Unsupported(VIRTIO_BLK_T_GET_ID)) {
                    error!("failed executing disk request: {}", e);
                }
                e.status()
            }
        };

        status_writer
            .write_all(&[status])
            .map_err(ExecuteError::WriteStatus)?;
        Ok(available_bytes)
    }

    fn process_queue(
        &mut self,
        queue_index: usize,
        flush_timer: &mut Timer,
        flush_timer_armed: &mut bool,
    ) {
        let queue = &mut self.queues[queue_index];

        let disk_size = self.disk_size.lock();

        while let Some(avail_desc) = queue.pop(&self.mem) {
            queue.set_notify(&self.mem, false);
            let desc_index = avail_desc.index;

            let len = match Worker::process_one_request(
                avail_desc,
                self.read_only,
                self.sparse,
                &mut *self.disk_image,
                *disk_size,
                self.id,
                flush_timer,
                flush_timer_armed,
                &self.mem,
            ) {
                Ok(len) => len,
                Err(e) => {
                    error!("block: failed to handle request: {}", e);
                    0
                }
            };

            queue.add_used(&self.mem, desc_index, len as u32);
            queue.trigger_interrupt(&self.mem, &self.interrupt);
            queue.set_notify(&self.mem, true);
        }
    }

    fn resize(&mut self, new_size: u64) -> DiskControlResult {
        if self.read_only {
            error!("Attempted to resize read-only block device");
            return DiskControlResult::Err(SysError::new(libc::EROFS));
        }

        info!("Resizing block device to {} bytes", new_size);

        if let Err(e) = self.disk_image.set_len(new_size) {
            error!("Resizing disk failed! {}", e);
            return DiskControlResult::Err(SysError::new(libc::EIO));
        }

        // Allocate new space if the disk image is not sparse.
        if let Err(e) = self.disk_image.allocate(0, new_size) {
            error!("Allocating disk space after resize failed! {}", e);
            return DiskControlResult::Err(SysError::new(libc::EIO));
        }

        self.sparse = false;

        if let Ok(new_disk_size) = self.disk_image.get_len() {
            let mut disk_size = self.disk_size.lock();
            *disk_size = new_disk_size;
        }
        DiskControlResult::Ok
    }

    fn run(&mut self, queue_evt: Event, kill_evt: Event) {
        #[derive(PollToken)]
        enum Token {
            FlushTimer,
            QueueAvailable,
            ControlRequest,
            InterruptResample,
            Kill,
        }

        let mut flush_timer = match Timer::new() {
            Ok(t) => t,
            Err(e) => {
                error!("Failed to create the flush timer: {}", e);
                return;
            }
        };
        let mut flush_timer_armed = false;

        let wait_ctx: WaitContext<Token> = match WaitContext::build_with(&[
            (&flush_timer, Token::FlushTimer),
            (&queue_evt, Token::QueueAvailable),
            (&kill_evt, Token::Kill),
        ])
        .and_then(|wc| {
            if let Some(resample_evt) = self.interrupt.get_resample_evt() {
                wc.add(resample_evt, Token::InterruptResample)?;
            }
            Ok(wc)
        })
        .and_then(|pc| {
            if let Some(control_tube) = self.control_tube.as_ref() {
                pc.add(control_tube, Token::ControlRequest)?
            }
            Ok(pc)
        }) {
            Ok(pc) => pc,
            Err(e) => {
                error!("failed creating WaitContext: {}", e);
                return;
            }
        };

        'wait: loop {
            let events = match wait_ctx.wait() {
                Ok(v) => v,
                Err(e) => {
                    error!("failed polling for events: {}", e);
                    break;
                }
            };

            let mut needs_config_interrupt = false;
            for event in events.iter().filter(|e| e.is_readable) {
                match event.token {
                    Token::FlushTimer => {
                        if let Err(e) = self.disk_image.fsync() {
                            error!("Failed to flush the disk: {}", e);
                            break 'wait;
                        }
                        if let Err(e) = flush_timer.wait() {
                            error!("Failed to clear flush timer: {}", e);
                            break 'wait;
                        }
                    }
                    Token::QueueAvailable => {
                        if let Err(e) = queue_evt.read() {
                            error!("failed reading queue Event: {}", e);
                            break 'wait;
                        }
                        self.process_queue(0, &mut flush_timer, &mut flush_timer_armed);
                    }
                    Token::ControlRequest => {
                        let control_tube = match self.control_tube.as_ref() {
                            Some(cs) => cs,
                            None => {
                                error!("received control socket request with no control socket");
                                break 'wait;
                            }
                        };
                        let req = match control_tube.recv() {
                            Ok(req) => req,
                            Err(e) => {
                                error!("control socket failed recv: {}", e);
                                break 'wait;
                            }
                        };

                        let resp = match req {
                            DiskControlCommand::Resize { new_size } => {
                                let resize_resp = self.resize(new_size);
                                if let DiskControlResult::Ok = resize_resp {
                                    needs_config_interrupt = true;
                                }
                                resize_resp
                            }
                        };

                        // We already know there is Some control_tube used to recv a request.
                        if let Err(e) = self.control_tube.as_ref().unwrap().send(&resp) {
                            error!("control socket failed send: {}", e);
                            break 'wait;
                        }
                    }
                    Token::InterruptResample => {
                        self.interrupt.interrupt_resample();
                    }
                    Token::Kill => break 'wait,
                }
            }
            if needs_config_interrupt {
                self.interrupt.signal_config_changed();
            }
        }
    }
}

/// Virtio device for exposing block level read/write operations on a host file.
pub struct Block {
    kill_evt: Option<Event>,
    worker_thread: Option<thread::JoinHandle<Worker>>,
    disk_image: Option<Box<dyn DiskFile>>,
    disk_size: Arc<Mutex<u64>>,
    avail_features: u64,
    read_only: bool,
    sparse: bool,
    seg_max: u32,
    block_size: u32,
    id: Option<BlockId>,
    control_tube: Option<Tube>,
}

impl Block {
    /// Create a new virtio block device that operates on the given DiskFile.
    pub fn new(
        base_features: u64,
        disk_image: Box<dyn DiskFile>,
        read_only: bool,
        sparse: bool,
        block_size: u32,
        id: Option<BlockId>,
        control_tube: Option<Tube>,
    ) -> SysResult<Block> {
        if block_size % SECTOR_SIZE as u32 != 0 {
            error!(
                "Block size {} is not a multiple of {}.",
                block_size, SECTOR_SIZE,
            );
            return Err(SysError::new(libc::EINVAL));
        }
        let disk_size = disk_image.get_len()?;
        if disk_size % block_size as u64 != 0 {
            warn!(
                "Disk size {} is not a multiple of block size {}; \
                 the remainder will not be visible to the guest.",
                disk_size, block_size,
            );
        }

        let avail_features = build_avail_features(base_features, read_only, sparse, false);

        let seg_max = get_seg_max(QUEUE_SIZE);

        Ok(Block {
            kill_evt: None,
            worker_thread: None,
            disk_image: Some(disk_image),
            disk_size: Arc::new(Mutex::new(disk_size)),
            avail_features,
            read_only,
            sparse,
            seg_max,
            block_size,
            id,
            control_tube,
        })
    }

    // Execute a single block device request.
    // `writer` includes the data region only; the status byte is not included.
    // It is up to the caller to convert the result of this function into a status byte
    // and write it to the expected location in guest memory.
    fn execute_request(
        reader: &mut Reader,
        writer: &mut Writer,
        read_only: bool,
        sparse: bool,
        disk: &mut dyn DiskFile,
        disk_size: u64,
        id: Option<BlockId>,
        flush_timer: &mut Timer,
        flush_timer_armed: &mut bool,
    ) -> result::Result<(), ExecuteError> {
        let req_header: virtio_blk_req_header = reader.read_obj().map_err(ExecuteError::Read)?;

        let req_type = req_header.req_type.to_native();
        let sector = req_header.sector.to_native();
        // Delay after a write when the file is auto-flushed.
        let flush_delay = Duration::from_secs(60);

        if read_only && req_type != VIRTIO_BLK_T_IN && req_type != VIRTIO_BLK_T_GET_ID {
            return Err(ExecuteError::ReadOnly {
                request_type: req_type,
            });
        }

        /// Check that a request accesses only data within the disk's current size.
        /// All parameters are in units of bytes.
        fn check_range(
            io_start: u64,
            io_length: u64,
            disk_size: u64,
        ) -> result::Result<(), ExecuteError> {
            let io_end = io_start
                .checked_add(io_length)
                .ok_or(ExecuteError::OutOfRange)?;
            if io_end > disk_size {
                Err(ExecuteError::OutOfRange)
            } else {
                Ok(())
            }
        }

        match req_type {
            VIRTIO_BLK_T_IN => {
                let data_len = writer.available_bytes();
                let offset = sector
                    .checked_shl(u32::from(SECTOR_SHIFT))
                    .ok_or(ExecuteError::OutOfRange)?;
                check_range(offset, data_len as u64, disk_size)?;
                writer
                    .write_all_from_at(disk, data_len, offset)
                    .map_err(|desc_error| ExecuteError::ReadIo {
                        length: data_len,
                        sector,
                        desc_error,
                    })?;
            }
            VIRTIO_BLK_T_OUT => {
                let data_len = reader.available_bytes();
                let offset = sector
                    .checked_shl(u32::from(SECTOR_SHIFT))
                    .ok_or(ExecuteError::OutOfRange)?;
                check_range(offset, data_len as u64, disk_size)?;
                reader
                    .read_exact_to_at(disk, data_len, offset)
                    .map_err(|desc_error| ExecuteError::WriteIo {
                        length: data_len,
                        sector,
                        desc_error,
                    })?;
                if !*flush_timer_armed {
                    flush_timer
                        .reset(flush_delay, None)
                        .map_err(ExecuteError::Timer)?;
                    *flush_timer_armed = true;
                }
            }
            VIRTIO_BLK_T_DISCARD | VIRTIO_BLK_T_WRITE_ZEROES => {
                if req_type == VIRTIO_BLK_T_DISCARD && !sparse {
                    // Discard is a hint; if this is a non-sparse disk, just ignore it.
                    return Ok(());
                }

                while reader.available_bytes() >= size_of::<virtio_blk_discard_write_zeroes>() {
                    let seg: virtio_blk_discard_write_zeroes =
                        reader.read_obj().map_err(ExecuteError::Read)?;

                    let sector = seg.sector.to_native();
                    let num_sectors = seg.num_sectors.to_native();
                    let flags = seg.flags.to_native();

                    let valid_flags = if req_type == VIRTIO_BLK_T_WRITE_ZEROES {
                        VIRTIO_BLK_DISCARD_WRITE_ZEROES_FLAG_UNMAP
                    } else {
                        0
                    };

                    if (flags & !valid_flags) != 0 {
                        return Err(ExecuteError::DiscardWriteZeroes {
                            ioerr: None,
                            sector,
                            num_sectors,
                            flags,
                        });
                    }

                    let offset = sector
                        .checked_shl(u32::from(SECTOR_SHIFT))
                        .ok_or(ExecuteError::OutOfRange)?;
                    let length = u64::from(num_sectors)
                        .checked_shl(u32::from(SECTOR_SHIFT))
                        .ok_or(ExecuteError::OutOfRange)?;
                    check_range(offset, length, disk_size)?;

                    if req_type == VIRTIO_BLK_T_DISCARD {
                        // Since Discard is just a hint and some filesystems may not implement
                        // FALLOC_FL_PUNCH_HOLE, ignore punch_hole errors.
                        let _ = disk.punch_hole(offset, length);
                    } else {
                        disk.write_zeroes_all_at(offset, length as usize)
                            .map_err(|e| ExecuteError::DiscardWriteZeroes {
                                ioerr: Some(e),
                                sector,
                                num_sectors,
                                flags,
                            })?;
                    }
                }
            }
            VIRTIO_BLK_T_FLUSH => {
                disk.fsync().map_err(ExecuteError::Flush)?;
                flush_timer.clear().map_err(ExecuteError::Timer)?;
                *flush_timer_armed = false;
            }
            VIRTIO_BLK_T_GET_ID => {
                if let Some(id) = id {
                    writer.write_all(&id).map_err(ExecuteError::CopyId)?;
                } else {
                    return Err(ExecuteError::Unsupported(req_type));
                }
            }
            t => return Err(ExecuteError::Unsupported(t)),
        };
        Ok(())
    }
}

impl Drop for Block {
    fn drop(&mut self) {
        if let Some(kill_evt) = self.kill_evt.take() {
            // Ignore the result because there is nothing we can do about it.
            let _ = kill_evt.write(1);
        }

        if let Some(worker_thread) = self.worker_thread.take() {
            let _ = worker_thread.join();
        }
    }
}

impl VirtioDevice for Block {
    fn keep_rds(&self) -> Vec<RawDescriptor> {
        let mut keep_rds = Vec::new();

        if let Some(disk_image) = &self.disk_image {
            keep_rds.extend(disk_image.as_raw_descriptors());
        }

        if let Some(control_tube) = &self.control_tube {
            keep_rds.push(control_tube.as_raw_descriptor());
        }

        keep_rds
    }

    fn features(&self) -> u64 {
        self.avail_features
    }

    fn device_type(&self) -> DeviceType {
        DeviceType::Block
    }

    fn queue_max_sizes(&self) -> &[u16] {
        QUEUE_SIZES
    }

    fn read_config(&self, offset: u64, data: &mut [u8]) {
        let config_space = {
            let disk_size = *self.disk_size.lock();
            build_config_space(disk_size, self.seg_max, self.block_size, NUM_QUEUES)
        };
        copy_config(data, 0, config_space.as_slice(), offset);
    }

    fn activate(
        &mut self,
        mem: GuestMemory,
        interrupt: Interrupt,
        queues: Vec<Queue>,
        mut queue_evts: Vec<Event>,
    ) {
        if queues.len() != 1 || queue_evts.len() != 1 {
            return;
        }

        let (self_kill_evt, kill_evt) = match Event::new().and_then(|e| Ok((e.try_clone()?, e))) {
            Ok(v) => v,
            Err(e) => {
                error!("failed creating kill Event pair: {}", e);
                return;
            }
        };
        self.kill_evt = Some(self_kill_evt);

        let read_only = self.read_only;
        let sparse = self.sparse;
        let disk_size = self.disk_size.clone();
        let id = self.id.take();
        if let Some(disk_image) = self.disk_image.take() {
            let control_tube = self.control_tube.take();
            let worker_result =
                thread::Builder::new()
                    .name("virtio_blk".to_string())
                    .spawn(move || {
                        let mut worker = Worker {
                            interrupt,
                            queues,
                            mem,
                            disk_image,
                            disk_size,
                            read_only,
                            sparse,
                            id,
                            control_tube,
                        };
                        worker.run(queue_evts.remove(0), kill_evt);
                        worker
                    });

            match worker_result {
                Err(e) => {
                    error!("failed to spawn virtio_blk worker: {}", e);
                    return;
                }
                Ok(join_handle) => {
                    self.worker_thread = Some(join_handle);
                }
            }
        }
    }

    fn reset(&mut self) -> bool {
        if let Some(kill_evt) = self.kill_evt.take() {
            if kill_evt.write(1).is_err() {
                error!("{}: failed to notify the kill event", self.debug_label());
                return false;
            }
        }

        if let Some(worker_thread) = self.worker_thread.take() {
            match worker_thread.join() {
                Err(_) => {
                    error!("{}: failed to get back resources", self.debug_label());
                    return false;
                }
                Ok(worker) => {
                    self.disk_image = Some(worker.disk_image);
                    self.control_tube = worker.control_tube;
                    return true;
                }
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use std::mem::size_of_val;

    use data_model::{Le32, Le64};
    use hypervisor::ProtectionType;
    use serde_keyvalue::*;
    use tempfile::tempfile;
    use vm_memory::GuestAddress;

    use crate::virtio::base_features;
    use crate::virtio::block::common::*;
    use crate::virtio::descriptor_utils::{create_descriptor_chain, DescriptorType};

    use super::*;

    #[test]
    fn read_size() {
        let f = tempfile().unwrap();
        f.set_len(0x1000).unwrap();

        let features = base_features(ProtectionType::Unprotected);
        let b = Block::new(features, Box::new(f), true, false, 512, None, None).unwrap();
        let mut num_sectors = [0u8; 4];
        b.read_config(0, &mut num_sectors);
        // size is 0x1000, so num_sectors is 8 (4096/512).
        assert_eq!([0x08, 0x00, 0x00, 0x00], num_sectors);
        let mut msw_sectors = [0u8; 4];
        b.read_config(4, &mut msw_sectors);
        // size is 0x1000, so msw_sectors is 0.
        assert_eq!([0x00, 0x00, 0x00, 0x00], msw_sectors);
    }

    #[test]
    fn read_block_size() {
        let f = tempfile().unwrap();
        f.set_len(0x1000).unwrap();

        let features = base_features(ProtectionType::Unprotected);
        let b = Block::new(features, Box::new(f), true, false, 4096, None, None).unwrap();
        let mut blk_size = [0u8; 4];
        b.read_config(20, &mut blk_size);
        // blk_size should be 4096 (0x1000).
        assert_eq!([0x00, 0x10, 0x00, 0x00], blk_size);
    }

    #[test]
    fn read_features() {
        // read-write block device
        {
            let f = tempfile().unwrap();
            let features = base_features(ProtectionType::Unprotected);
            let b = Block::new(features, Box::new(f), false, true, 512, None, None).unwrap();
            // writable device should set VIRTIO_BLK_F_FLUSH + VIRTIO_BLK_F_DISCARD
            // + VIRTIO_BLK_F_WRITE_ZEROES + VIRTIO_F_VERSION_1 + VIRTIO_BLK_F_BLK_SIZE
            // + VIRTIO_BLK_F_SEG_MAX + VIRTIO_RING_F_EVENT_IDX
            assert_eq!(0x120006244, b.features());
        }

        // read-write block device, non-sparse
        {
            let f = tempfile().unwrap();
            let features = base_features(ProtectionType::Unprotected);
            let b = Block::new(features, Box::new(f), false, false, 512, None, None).unwrap();
            // writable device should set VIRTIO_BLK_F_FLUSH
            // + VIRTIO_BLK_F_WRITE_ZEROES + VIRTIO_F_VERSION_1 + VIRTIO_BLK_F_BLK_SIZE
            // + VIRTIO_BLK_F_SEG_MAX + VIRTIO_RING_F_EVENT_IDX
            assert_eq!(0x120004244, b.features());
        }

        // read-only block device
        {
            let f = tempfile().unwrap();
            let features = base_features(ProtectionType::Unprotected);
            let b = Block::new(features, Box::new(f), true, true, 512, None, None).unwrap();
            // read-only device should set VIRTIO_BLK_F_FLUSH and VIRTIO_BLK_F_RO
            // + VIRTIO_F_VERSION_1 + VIRTIO_BLK_F_BLK_SIZE + VIRTIO_BLK_F_SEG_MAX
            // + VIRTIO_RING_F_EVENT_IDX
            assert_eq!(0x120000264, b.features());
        }
    }

    #[test]
    fn read_last_sector() {
        let mut f = tempfile().unwrap();
        let disk_size = 0x1000;
        f.set_len(disk_size).unwrap();

        let mem = GuestMemory::new(&[(GuestAddress(0u64), 4 * 1024 * 1024)])
            .expect("Creating guest memory failed.");

        let req_hdr = virtio_blk_req_header {
            req_type: Le32::from(VIRTIO_BLK_T_IN),
            reserved: Le32::from(0),
            sector: Le64::from(7), // Disk is 8 sectors long, so this is the last valid sector.
        };
        mem.write_obj_at_addr(req_hdr, GuestAddress(0x1000))
            .expect("writing req failed");

        let avail_desc = create_descriptor_chain(
            &mem,
            GuestAddress(0x100),  // Place descriptor chain at 0x100.
            GuestAddress(0x1000), // Describe buffer at 0x1000.
            vec![
                // Request header
                (DescriptorType::Readable, size_of_val(&req_hdr) as u32),
                // I/O buffer (1 sector of data)
                (DescriptorType::Writable, 512),
                // Request status
                (DescriptorType::Writable, 1),
            ],
            0,
        )
        .expect("create_descriptor_chain failed");

        let mut flush_timer = Timer::new().expect("failed to create flush_timer");
        let mut flush_timer_armed = false;

        Worker::process_one_request(
            avail_desc,
            false,
            true,
            &mut f,
            disk_size,
            None,
            &mut flush_timer,
            &mut flush_timer_armed,
            &mem,
        )
        .expect("execute failed");

        let status_offset = GuestAddress((0x1000 + size_of_val(&req_hdr) + 512) as u64);
        let status = mem.read_obj_from_addr::<u8>(status_offset).unwrap();
        assert_eq!(status, VIRTIO_BLK_S_OK);
    }

    #[test]
    fn read_beyond_last_sector() {
        let mut f = tempfile().unwrap();
        let disk_size = 0x1000;
        f.set_len(disk_size).unwrap();

        let mem = GuestMemory::new(&[(GuestAddress(0u64), 4 * 1024 * 1024)])
            .expect("Creating guest memory failed.");

        let req_hdr = virtio_blk_req_header {
            req_type: Le32::from(VIRTIO_BLK_T_IN),
            reserved: Le32::from(0),
            sector: Le64::from(7), // Disk is 8 sectors long, so this is the last valid sector.
        };
        mem.write_obj_at_addr(req_hdr, GuestAddress(0x1000))
            .expect("writing req failed");

        let avail_desc = create_descriptor_chain(
            &mem,
            GuestAddress(0x100),  // Place descriptor chain at 0x100.
            GuestAddress(0x1000), // Describe buffer at 0x1000.
            vec![
                // Request header
                (DescriptorType::Readable, size_of_val(&req_hdr) as u32),
                // I/O buffer (2 sectors of data - overlap the end of the disk).
                (DescriptorType::Writable, 512 * 2),
                // Request status
                (DescriptorType::Writable, 1),
            ],
            0,
        )
        .expect("create_descriptor_chain failed");

        let mut flush_timer = Timer::new().expect("failed to create flush_timer");
        let mut flush_timer_armed = false;

        Worker::process_one_request(
            avail_desc,
            false,
            true,
            &mut f,
            disk_size,
            None,
            &mut flush_timer,
            &mut flush_timer_armed,
            &mem,
        )
        .expect("execute failed");

        let status_offset = GuestAddress((0x1000 + size_of_val(&req_hdr) + 512 * 2) as u64);
        let status = mem.read_obj_from_addr::<u8>(status_offset).unwrap();
        assert_eq!(status, VIRTIO_BLK_S_IOERR);
    }

    #[test]
    fn get_id() {
        let mut f = tempfile().unwrap();
        let disk_size = 0x1000;
        f.set_len(disk_size).unwrap();

        let mem = GuestMemory::new(&[(GuestAddress(0u64), 4 * 1024 * 1024)])
            .expect("Creating guest memory failed.");

        let req_hdr = virtio_blk_req_header {
            req_type: Le32::from(VIRTIO_BLK_T_GET_ID),
            reserved: Le32::from(0),
            sector: Le64::from(0),
        };
        mem.write_obj_at_addr(req_hdr, GuestAddress(0x1000))
            .expect("writing req failed");

        let avail_desc = create_descriptor_chain(
            &mem,
            GuestAddress(0x100),  // Place descriptor chain at 0x100.
            GuestAddress(0x1000), // Describe buffer at 0x1000.
            vec![
                // Request header
                (DescriptorType::Readable, size_of_val(&req_hdr) as u32),
                // I/O buffer (20 bytes for serial)
                (DescriptorType::Writable, 20),
                // Request status
                (DescriptorType::Writable, 1),
            ],
            0,
        )
        .expect("create_descriptor_chain failed");

        let mut flush_timer = Timer::new().expect("failed to create flush_timer");
        let mut flush_timer_armed = false;

        let id = b"a20-byteserialnumber";

        Worker::process_one_request(
            avail_desc,
            false,
            true,
            &mut f,
            disk_size,
            Some(*id),
            &mut flush_timer,
            &mut flush_timer_armed,
            &mem,
        )
        .expect("execute failed");

        let status_offset = GuestAddress((0x1000 + size_of_val(&req_hdr) + 20) as u64);
        let status = mem.read_obj_from_addr::<u8>(status_offset).unwrap();
        assert_eq!(status, VIRTIO_BLK_S_OK);

        let id_offset = GuestAddress(0x1000 + size_of_val(&req_hdr) as u64);
        let returned_id = mem.read_obj_from_addr::<[u8; 20]>(id_offset).unwrap();
        assert_eq!(returned_id, *id);
    }

    fn from_block_arg(options: &str) -> Result<DiskOption, ParseError> {
        from_key_values(options)
    }

    #[test]
    fn params_from_key_values() {
        // Path argument is mandatory.
        let err = from_block_arg("").unwrap_err();
        assert_eq!(
            err,
            ParseError {
                kind: ErrorKind::SerdeError("missing field `path`".into()),
                pos: 0,
            }
        );

        // Path is the default argument.
        let params = from_block_arg("/path/to/disk.img").unwrap();
        assert_eq!(
            params,
            DiskOption {
                path: "/path/to/disk.img".into(),
                read_only: false,
                sparse: true,
                o_direct: false,
                block_size: 512,
                id: None,
            }
        );

        // Explicitly-specified path.
        let params = from_block_arg("path=/path/to/disk.img").unwrap();
        assert_eq!(
            params,
            DiskOption {
                path: "/path/to/disk.img".into(),
                read_only: false,
                sparse: true,
                o_direct: false,
                block_size: 512,
                id: None,
            }
        );

        // read_only
        let params = from_block_arg("/some/path.img,ro").unwrap();
        assert_eq!(
            params,
            DiskOption {
                path: "/some/path.img".into(),
                read_only: true,
                sparse: true,
                o_direct: false,
                block_size: 512,
                id: None,
            }
        );

        // sparse
        let params = from_block_arg("/some/path.img,sparse").unwrap();
        assert_eq!(
            params,
            DiskOption {
                path: "/some/path.img".into(),
                read_only: false,
                sparse: true,
                o_direct: false,
                block_size: 512,
                id: None,
            }
        );
        let params = from_block_arg("/some/path.img,sparse=false").unwrap();
        assert_eq!(
            params,
            DiskOption {
                path: "/some/path.img".into(),
                read_only: false,
                sparse: false,
                o_direct: false,
                block_size: 512,
                id: None,
            }
        );

        // o_direct
        let params = from_block_arg("/some/path.img,o_direct").unwrap();
        assert_eq!(
            params,
            DiskOption {
                path: "/some/path.img".into(),
                read_only: false,
                sparse: true,
                o_direct: true,
                block_size: 512,
                id: None,
            }
        );

        // block_size
        let params = from_block_arg("/some/path.img,block_size=128").unwrap();
        assert_eq!(
            params,
            DiskOption {
                path: "/some/path.img".into(),
                read_only: false,
                sparse: true,
                o_direct: false,
                block_size: 128,
                id: None,
            }
        );

        // id
        let params = from_block_arg("/some/path.img,id=DISK").unwrap();
        assert_eq!(
            params,
            DiskOption {
                path: "/some/path.img".into(),
                read_only: false,
                sparse: true,
                o_direct: false,
                block_size: 512,
                id: Some(*b"DISK\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0"),
            }
        );
        let err = from_block_arg("/some/path.img,id=DISK_ID_IS_WAY_TOO_LONG").unwrap_err();
        assert_eq!(
            err,
            ParseError {
                kind: ErrorKind::SerdeError("disk id must be 20 or fewer characters".into()),
                pos: 0,
            }
        );

        // All together
        let params =
            from_block_arg("/some/path.img,block_size=256,ro,sparse=false,id=DISK_LABEL,o_direct")
                .unwrap();
        assert_eq!(
            params,
            DiskOption {
                path: "/some/path.img".into(),
                read_only: true,
                sparse: false,
                o_direct: true,
                block_size: 256,
                id: Some(*b"DISK_LABEL\0\0\0\0\0\0\0\0\0\0"),
            }
        );
    }
}
