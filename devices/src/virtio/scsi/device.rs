// Copyright 2023 The ChromiumOS Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#![deny(missing_docs)]

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::io;
use std::io::Write;
use std::rc::Rc;

use anyhow::Context;
use base::error;
use base::warn;
use base::Event;
use base::WorkerThread;
use cros_async::EventAsync;
use cros_async::Executor;
use cros_async::ExecutorKind;
use disk::AsyncDisk;
use disk::DiskFile;
use futures::pin_mut;
use futures::stream::FuturesUnordered;
use futures::FutureExt;
use futures::StreamExt;
use remain::sorted;
use thiserror::Error as ThisError;
use virtio_sys::virtio_scsi::virtio_scsi_cmd_req;
use virtio_sys::virtio_scsi::virtio_scsi_cmd_resp;
use virtio_sys::virtio_scsi::virtio_scsi_config;
use virtio_sys::virtio_scsi::virtio_scsi_ctrl_an_resp;
use virtio_sys::virtio_scsi::virtio_scsi_ctrl_tmf_req;
use virtio_sys::virtio_scsi::virtio_scsi_ctrl_tmf_resp;
use virtio_sys::virtio_scsi::virtio_scsi_event;
use virtio_sys::virtio_scsi::VIRTIO_SCSI_CDB_DEFAULT_SIZE;
use virtio_sys::virtio_scsi::VIRTIO_SCSI_SENSE_DEFAULT_SIZE;
use virtio_sys::virtio_scsi::VIRTIO_SCSI_S_BAD_TARGET;
use virtio_sys::virtio_scsi::VIRTIO_SCSI_S_FUNCTION_REJECTED;
use virtio_sys::virtio_scsi::VIRTIO_SCSI_S_FUNCTION_SUCCEEDED;
use virtio_sys::virtio_scsi::VIRTIO_SCSI_S_INCORRECT_LUN;
use virtio_sys::virtio_scsi::VIRTIO_SCSI_S_OK;
use virtio_sys::virtio_scsi::VIRTIO_SCSI_T_AN_QUERY;
use virtio_sys::virtio_scsi::VIRTIO_SCSI_T_AN_SUBSCRIBE;
use virtio_sys::virtio_scsi::VIRTIO_SCSI_T_TMF;
use virtio_sys::virtio_scsi::VIRTIO_SCSI_T_TMF_I_T_NEXUS_RESET;
use virtio_sys::virtio_scsi::VIRTIO_SCSI_T_TMF_LOGICAL_UNIT_RESET;
use vm_memory::GuestMemory;
use zerocopy::AsBytes;

use crate::virtio::async_utils;
use crate::virtio::block::sys::get_seg_max;
use crate::virtio::copy_config;
use crate::virtio::scsi::commands::Command;
use crate::virtio::scsi::constants::CHECK_CONDITION;
use crate::virtio::scsi::constants::GOOD;
use crate::virtio::scsi::constants::ILLEGAL_REQUEST;
use crate::virtio::scsi::constants::MEDIUM_ERROR;
use crate::virtio::DescriptorChain;
use crate::virtio::DeviceType as VirtioDeviceType;
use crate::virtio::Interrupt;
use crate::virtio::Queue;
use crate::virtio::Reader;
use crate::virtio::VirtioDevice;
use crate::virtio::Writer;

// The following values reflects the virtio v1.2 spec:
// <https://docs.oasis-open.org/virtio/virtio/v1.2/csd01/virtio-v1.2-csd01.html#x1-3470004>

// Should have one controlq, one eventq, and at least one request queue.
const MIN_NUM_QUEUES: usize = 3;
// The number of queues exposed by the device.
// First crosvm pass this value through `VirtioDevice::read_config`, and then the driver determines
// the number of queues which does not exceed the passed value. The determined value eventually
// shows as the length of `queues` in `VirtioDevice::activate`.
const MAX_NUM_QUEUES: usize = 16;
// Max channel should be 0.
const DEFAULT_MAX_CHANNEL: u16 = 0;
// Max target should be less than or equal to 255.
const DEFAULT_MAX_TARGET: u16 = 255;
// Max lun should be less than or equal to 16383
const DEFAULT_MAX_LUN: u32 = 16383;

const DEFAULT_QUEUE_SIZE: u16 = 256;

// The maximum number of linked commands.
const MAX_CMD_PER_LUN: u32 = 128;
// We set the maximum transfer size hint to 0xffff: 2^16 * 512 ~ 34mb.
const MAX_SECTORS: u32 = 0xffff;

const fn virtio_scsi_cmd_resp_ok() -> virtio_scsi_cmd_resp {
    virtio_scsi_cmd_resp {
        sense_len: 0,
        resid: 0,
        status_qualifier: 0,
        status: GOOD,
        response: VIRTIO_SCSI_S_OK as u8,
        sense: [0; VIRTIO_SCSI_SENSE_DEFAULT_SIZE as usize],
    }
}

/// Errors that happen while handling scsi commands.
#[sorted]
#[derive(ThisError, Debug)]
pub enum ExecuteError {
    #[error("invalid cdb field")]
    InvalidField,
    #[error("{length} bytes from sector {sector} exceeds end of this device {max_lba}")]
    LbaOutOfRange {
        length: usize,
        sector: u64,
        max_lba: u64,
    },
    #[error("failed to read message: {0}")]
    Read(io::Error),
    #[error("failed to read command from cdb")]
    ReadCommand,
    #[error("io error {resid} bytes remained to be read: {desc_error}")]
    ReadIo {
        resid: usize,
        desc_error: disk::Error,
    },
    #[error("writing to a read only device")]
    ReadOnly,
    #[error("saving parameters not supported")]
    SavingParamNotSupported,
    #[error("synchronization error")]
    SynchronizationError,
    #[error("unsupported scsi command: {0}")]
    Unsupported(u8),
    #[error("failed to write message: {0}")]
    Write(io::Error),
    #[error("io error {resid} bytes remained to be written: {desc_error}")]
    WriteIo {
        resid: usize,
        desc_error: disk::Error,
    },
}

impl ExecuteError {
    // TODO(b/301011017): We would need to define something like
    // virtio_scsi_cmd_resp_header to cope with the configurable sense size.
    fn as_resp(&self) -> virtio_scsi_cmd_resp {
        let resp = virtio_scsi_cmd_resp_ok();
        // The asc and ascq assignments are taken from the t10 SPC spec.
        // cf) Table 28 of <https://www.t10.org/cgi-bin/ac.pl?t=f&f=spc3r23.pdf>
        let sense = match self {
            Self::Read(_) | Self::ReadCommand => {
                // UNRECOVERED READ ERROR
                Sense {
                    key: MEDIUM_ERROR,
                    asc: 0x11,
                    ascq: 0x00,
                }
            }
            Self::Write(_) => {
                // WRITE ERROR
                Sense {
                    key: MEDIUM_ERROR,
                    asc: 0x0c,
                    ascq: 0x00,
                }
            }
            Self::InvalidField => {
                // INVALID FIELD IN CDB
                Sense {
                    key: ILLEGAL_REQUEST,
                    asc: 0x24,
                    ascq: 0x00,
                }
            }
            Self::Unsupported(_) => {
                // INVALID COMMAND OPERATION CODE
                Sense {
                    key: ILLEGAL_REQUEST,
                    asc: 0x20,
                    ascq: 0x00,
                }
            }
            Self::ReadOnly | Self::LbaOutOfRange { .. } => {
                // LOGICAL BLOCK ADDRESS OUT OF RANGE
                Sense {
                    key: ILLEGAL_REQUEST,
                    asc: 0x21,
                    ascq: 0x00,
                }
            }
            Self::SavingParamNotSupported => Sense {
                // SAVING PARAMETERS NOT SUPPORTED
                key: ILLEGAL_REQUEST,
                asc: 0x39,
                ascq: 0x00,
            },
            Self::SynchronizationError => Sense {
                // SYNCHRONIZATION ERROR
                key: MEDIUM_ERROR,
                asc: 0x16,
                ascq: 0x00,
            },
            // Ignore these errors.
            Self::ReadIo { resid, desc_error } | Self::WriteIo { resid, desc_error } => {
                warn!("error while performing I/O {}", desc_error);
                return virtio_scsi_cmd_resp {
                    resid: (*resid).try_into().unwrap_or(u32::MAX).to_be(),
                    ..resp
                };
            }
        };
        let (sense, sense_len) = sense.as_bytes(true);
        virtio_scsi_cmd_resp {
            sense_len,
            sense,
            status: CHECK_CONDITION,
            ..resp
        }
    }
}

/// Sense code representation
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct Sense {
    /// Provides generic information describing an error or exception condition.
    pub key: u8,
    /// Additional Sense Code.
    /// Indicates further information related to the error or exception reported in the key field.
    pub asc: u8,
    /// Additional Sense Code Qualifier.
    /// Indicates further detailed information related to the additional sense code.
    pub ascq: u8,
}

impl Sense {
    // Converts to (sense bytes, actual size of the sense data)
    // There are two formats to convert sense data to bytes; fixed format and descriptor format.
    // Details are in SPC-3 t10 revision 23: <https://www.t10.org/cgi-bin/ac.pl?t=f&f=spc3r23.pdf>
    fn as_bytes(&self, fixed: bool) -> ([u8; VIRTIO_SCSI_SENSE_DEFAULT_SIZE as usize], u32) {
        let mut sense_data = [0u8; VIRTIO_SCSI_SENSE_DEFAULT_SIZE as usize];
        if fixed {
            // Fixed format sense data has response code:
            // 1) 0x70 for current errors
            // 2) 0x71 for deferred errors
            sense_data[0] = 0x70;
            // sense_data[1]: Obsolete
            // Sense key
            sense_data[2] = self.key;
            // sense_data[3..7]: Information field, which we do not support.
            // Additional length. The data is 18 bytes, and this byte is 8th.
            sense_data[7] = 10;
            // sense_data[8..12]: Command specific information, which we do not support.
            // Additional sense code
            sense_data[12] = self.asc;
            // Additional sense code qualifier
            sense_data[13] = self.ascq;
            // sense_data[14]: Field replaceable unit code, which we do not support.
            // sense_data[15..18]: Field replaceable unit code, which we do not support.
            (sense_data, 18)
        } else {
            // Descriptor format sense data has response code:
            // 1) 0x72 for current errors
            // 2) 0x73 for deferred errors
            sense_data[0] = 0x72;
            // Sense key
            sense_data[1] = self.key;
            // Additional sense code
            sense_data[2] = self.asc;
            // Additional sense code qualifier
            sense_data[3] = self.ascq;
            // sense_data[4..7]: Reserved
            // sense_data[7]: Additional sense length, which is 0 in this case.
            (sense_data, 8)
        }
    }
}

/// Describes each SCSI device.
#[derive(Copy, Clone)]
pub struct LogicalUnit {
    /// The maximum logical block address of the target device.
    pub max_lba: u64,
    /// Block size of the target device.
    pub block_size: u32,
    pub read_only: bool,
}

/// Vitio device for exposing SCSI command operations on a host file.
pub struct Controller {
    // Bitmap of virtio-scsi feature bits.
    avail_features: u64,
    // Represents the image on disk.
    disk_image: Option<Box<dyn DiskFile>>,
    // Sizes for the virtqueue.
    queue_sizes: Vec<u16>,
    // The maximum number of segments that can be in a command.
    seg_max: u32,
    // The size of the sense data.
    sense_size: u32,
    // The byte size of the CDB that the driver will write.
    cdb_size: u32,
    executor_kind: ExecutorKind,
    worker_threads: Vec<WorkerThread<()>>,
    // TODO(b/300586438): Make this a BTreeMap<_> to enable this Device struct to manage multiple
    // LogicalUnit. That is, when user passes multiple --scsi-block options, we will have a single
    // instance of Device which has multiple LogicalUnit.
    #[allow(dead_code)]
    target: LogicalUnit,
    // Whether the devices handles requests in multiple request queues.
    // If true, each virtqueue will be handled in a separate worker thread.
    multi_queue: bool,
}

impl Controller {
    /// Creates a virtio-scsi device.
    pub fn new(
        disk_image: Box<dyn DiskFile>,
        base_features: u64,
        block_size: u32,
        read_only: bool,
    ) -> anyhow::Result<Self> {
        let target = LogicalUnit {
            max_lba: disk_image
                .get_len()
                .context("Failed to get the length of the disk image")?,
            block_size,
            read_only,
        };
        let multi_queue = disk_image.try_clone().is_ok();
        let num_queues = if multi_queue {
            MAX_NUM_QUEUES
        } else {
            MIN_NUM_QUEUES
        };
        // b/300560198: Support feature bits in virtio-scsi.
        Ok(Self {
            avail_features: base_features,
            disk_image: Some(disk_image),
            queue_sizes: vec![DEFAULT_QUEUE_SIZE; num_queues],
            seg_max: get_seg_max(DEFAULT_QUEUE_SIZE),
            sense_size: VIRTIO_SCSI_SENSE_DEFAULT_SIZE,
            cdb_size: VIRTIO_SCSI_CDB_DEFAULT_SIZE,
            executor_kind: ExecutorKind::default(),
            worker_threads: vec![],
            target,
            multi_queue,
        })
    }

    fn build_config_space(&self) -> virtio_scsi_config {
        virtio_scsi_config {
            // num_queues is the number of request queues only so we subtract 2 for the control
            // queue and the event queue.
            num_queues: self.queue_sizes.len() as u32 - 2,
            seg_max: self.seg_max,
            max_sectors: MAX_SECTORS,
            cmd_per_lun: MAX_CMD_PER_LUN,
            event_info_size: std::mem::size_of::<virtio_scsi_event>() as u32,
            sense_size: self.sense_size,
            cdb_size: self.cdb_size,
            max_channel: DEFAULT_MAX_CHANNEL,
            max_target: DEFAULT_MAX_TARGET,
            max_lun: DEFAULT_MAX_LUN,
        }
    }

    // Executes a request in the controlq.
    fn execute_control(reader: &mut Reader, writer: &mut Writer) -> Result<(), ExecuteError> {
        let typ = reader.peek_obj::<u32>().map_err(ExecuteError::Read)?;
        match typ {
            VIRTIO_SCSI_T_TMF => {
                let tmf = reader
                    .read_obj::<virtio_scsi_ctrl_tmf_req>()
                    .map_err(ExecuteError::Read)?;
                let resp = Self::execute_tmf(tmf);
                writer.write_obj(resp).map_err(ExecuteError::Write)?;
                Ok(())
            }
            VIRTIO_SCSI_T_AN_QUERY | VIRTIO_SCSI_T_AN_SUBSCRIBE => {
                // We do not support any asynchronous notification queries hence `event_actual`
                // will be 0.
                let resp = virtio_scsi_ctrl_an_resp {
                    event_actual: 0,
                    response: VIRTIO_SCSI_S_OK as u8,
                };
                writer.write_obj(resp).map_err(ExecuteError::Write)?;
                Ok(())
            }
            _ => {
                error!("invalid type of a control request: {typ}");
                Err(ExecuteError::InvalidField)
            }
        }
    }

    // Executes a TMF (task management function) request.
    fn execute_tmf(tmf: virtio_scsi_ctrl_tmf_req) -> virtio_scsi_ctrl_tmf_resp {
        match tmf.subtype {
            VIRTIO_SCSI_T_TMF_LOGICAL_UNIT_RESET | VIRTIO_SCSI_T_TMF_I_T_NEXUS_RESET => {
                // We only have LUN0.
                let response = if Self::is_lun0(tmf.lun) {
                    VIRTIO_SCSI_S_FUNCTION_SUCCEEDED as u8
                } else {
                    VIRTIO_SCSI_S_INCORRECT_LUN as u8
                };
                virtio_scsi_ctrl_tmf_resp { response }
            }
            subtype => {
                error!("TMF request {subtype} is not supported");
                virtio_scsi_ctrl_tmf_resp {
                    response: VIRTIO_SCSI_S_FUNCTION_REJECTED as u8,
                }
            }
        }
    }

    async fn execute_request(
        reader: &mut Reader,
        resp_writer: &mut Writer,
        data_writer: &mut Writer,
        disk_image: &dyn AsyncDisk,
        dev: LogicalUnit,
    ) -> Result<(), ExecuteError> {
        // TODO(b/301011017): Cope with the configurable cdb size. We would need to define
        // something like virtio_scsi_cmd_req_header.
        let req_header = reader
            .read_obj::<virtio_scsi_cmd_req>()
            .map_err(ExecuteError::Read)?;
        let resp = if Self::has_lun(req_header.lun) {
            let command = Command::new(&req_header.cdb)?;
            match command.execute(reader, data_writer, dev, disk_image).await {
                Ok(()) => virtio_scsi_cmd_resp {
                    sense_len: 0,
                    resid: 0,
                    status_qualifier: 0,
                    status: GOOD,
                    response: VIRTIO_SCSI_S_OK as u8,
                    sense: [0; VIRTIO_SCSI_SENSE_DEFAULT_SIZE as usize],
                },
                Err(err) => {
                    error!("error while executing a scsi request: {err}");
                    err.as_resp()
                }
            }
        } else {
            virtio_scsi_cmd_resp {
                response: VIRTIO_SCSI_S_BAD_TARGET as u8,
                ..Default::default()
            }
        };
        resp_writer
            .write_all(resp.as_bytes())
            .map_err(ExecuteError::Write)?;
        Ok(())
    }

    // TODO(b/300586438): Once we alter Controller to handle multiple LogicalUnit, we should update
    // the search strategy as well.
    fn has_lun(lun: [u8; 8]) -> bool {
        // First byte should be 1.
        if lun[0] != 1 {
            return false;
        }
        let bus_id = lun[1];
        // General search strategy for scsi devices is as follows:
        // 1) Look for a device which has the same bus id and lun indicated by the given lun. If
        //    there is one, that is the target device.
        // 2) If we cannot find such device, then we return the first device that has the same bus
        //    id.
        // Since we only support LUN0 for now, we only need to compare the bus id.
        bus_id == 0
    }

    fn is_lun0(lun: [u8; 8]) -> bool {
        u16::from_be_bytes([lun[2], lun[3]]) & 0x3fff == 0
    }
}

impl VirtioDevice for Controller {
    fn keep_rds(&self) -> Vec<base::RawDescriptor> {
        self.disk_image
            .as_ref()
            .map(|i| i.as_raw_descriptors())
            .unwrap_or_default()
    }

    fn features(&self) -> u64 {
        self.avail_features
    }

    fn device_type(&self) -> VirtioDeviceType {
        VirtioDeviceType::Scsi
    }

    fn queue_max_sizes(&self) -> &[u16] {
        &self.queue_sizes
    }

    fn read_config(&self, offset: u64, data: &mut [u8]) {
        let config_space = self.build_config_space();
        copy_config(data, 0, config_space.as_bytes(), offset);
    }

    // TODO(b/301011017): implement the write_config method to make spec values writable from the
    // guest driver.

    fn activate(
        &mut self,
        _mem: GuestMemory,
        interrupt: Interrupt,
        mut queues: BTreeMap<usize, Queue>,
    ) -> anyhow::Result<()> {
        let executor_kind = self.executor_kind;
        let dev = self.target;
        let disk_image = self
            .disk_image
            .take()
            .context("Failed to take a disk image")?;
        // 0th virtqueue is the controlq.
        let controlq = queues.remove(&0).context("controlq should be present")?;
        // 1st virtqueue is the eventq.
        // We do not send any events through eventq.
        let _eventq = queues.remove(&1).context("eventq should be present")?;
        // The rest of the queues are request queues.
        let request_queues = if self.multi_queue {
            queues
                .into_values()
                .map(|queue| {
                    let disk = disk_image
                        .try_clone()
                        .context("Failed to clone a disk image")?;
                    Ok((queue, disk))
                })
                .collect::<anyhow::Result<_>>()?
        } else {
            // Handle all virtio requests with one thread.
            vec![(
                queues
                    .remove(&2)
                    .context("request queue should be present")?,
                disk_image,
            )]
        };

        let intr = interrupt.clone();
        let worker_thread = WorkerThread::start("v_scsi_ctrlq", move |kill_evt| {
            let ex =
                Executor::with_executor_kind(executor_kind).expect("Failed to create an executor");
            if let Err(err) = ex
                .run_until(run_worker(
                    &ex,
                    intr,
                    controlq,
                    kill_evt,
                    QueueType::Control,
                    dev,
                ))
                .expect("run_until failed")
            {
                error!("run_worker failed: {err}");
            }
        });
        self.worker_threads.push(worker_thread);

        for (i, (queue, disk_image)) in request_queues.into_iter().enumerate() {
            let interrupt = interrupt.clone();
            let worker_thread =
                WorkerThread::start(format!("v_scsi_req_{}", i + 2), move |kill_evt| {
                    let ex = Executor::with_executor_kind(executor_kind)
                        .expect("Failed to create an executor");
                    let async_disk = match disk_image.to_async_disk(&ex) {
                        Ok(d) => d,
                        Err(e) => panic!("Failed to create async disk: {}", e),
                    };
                    if let Err(err) = ex
                        .run_until(run_worker(
                            &ex,
                            interrupt,
                            queue,
                            kill_evt,
                            QueueType::Request(async_disk),
                            dev,
                        ))
                        .expect("run_until failed")
                    {
                        error!("run_worker failed: {err}");
                    }
                });
            self.worker_threads.push(worker_thread);
        }
        Ok(())
    }
}

enum QueueType {
    Control,
    Request(Box<dyn AsyncDisk>),
}

async fn run_worker(
    ex: &Executor,
    interrupt: Interrupt,
    queue: Queue,
    kill_evt: Event,
    queue_type: QueueType,
    dev: LogicalUnit,
) -> anyhow::Result<()> {
    let kill = async_utils::await_and_exit(ex, kill_evt).fuse();
    pin_mut!(kill);

    let resample = async_utils::handle_irq_resample(ex, interrupt.clone()).fuse();
    pin_mut!(resample);

    let kick_evt = queue
        .event()
        .try_clone()
        .expect("Failed to clone queue event");
    let queue_handler = handle_queue(
        Rc::new(RefCell::new(queue)),
        EventAsync::new(kick_evt, ex).expect("Failed to create async event for queue"),
        interrupt,
        queue_type,
        dev,
    )
    .fuse();
    pin_mut!(queue_handler);

    futures::select! {
        _ = queue_handler => anyhow::bail!("queue handler exited unexpectedly"),
        r = resample => return r.context("failed to resample an irq value"),
        r = kill => return r.context("failed to wait on the kill event"),
    };
}

async fn handle_queue(
    queue: Rc<RefCell<Queue>>,
    evt: EventAsync,
    interrupt: Interrupt,
    queue_type: QueueType,
    dev: LogicalUnit,
) {
    let mut background_tasks = FuturesUnordered::new();
    let evt_future = evt.next_val().fuse();
    pin_mut!(evt_future);
    loop {
        futures::select! {
            _ = background_tasks.next() => continue,
            res = evt_future => {
                evt_future.set(evt.next_val().fuse());
                if let Err(e) = res {
                    error!("Failed to read the next queue event: {e}");
                    continue;
                }
            }
        }
        while let Some(chain) = queue.borrow_mut().pop() {
            background_tasks.push(process_one_chain(
                &queue,
                chain,
                &interrupt,
                &queue_type,
                dev,
            ));
        }
    }
}

async fn process_one_chain(
    queue: &RefCell<Queue>,
    mut avail_desc: DescriptorChain,
    interrupt: &Interrupt,
    queue_type: &QueueType,
    dev: LogicalUnit,
) {
    let len = process_one_request(&mut avail_desc, queue_type, dev).await;
    let mut queue = queue.borrow_mut();
    queue.add_used(avail_desc, len as u32);
    queue.trigger_interrupt(interrupt);
}

async fn process_one_request(
    avail_desc: &mut DescriptorChain,
    queue_type: &QueueType,
    dev: LogicalUnit,
) -> usize {
    let reader = &mut avail_desc.reader;
    let resp_writer = &mut avail_desc.writer;
    match queue_type {
        QueueType::Control => {
            if let Err(err) = Controller::execute_control(reader, resp_writer) {
                error!("failed to execute control request: {err}");
            }
            resp_writer.bytes_written()
        }
        QueueType::Request(disk_image) => {
            let mut data_writer = resp_writer.split_at(std::mem::size_of::<virtio_scsi_cmd_resp>());
            if let Err(err) = Controller::execute_request(
                reader,
                resp_writer,
                &mut data_writer,
                disk_image.as_ref(),
                dev,
            )
            .await
            {
                // If the write of the virtio_scsi_cmd_resp fails, there is nothing we can do to
                // inform the error to the guest driver (we usually propagate errors with sense
                // field, which is in the struct virtio_scsi_cmd_resp). The guest driver should
                // have at least sizeof(virtio_scsi_cmd_resp) bytes of device-writable part
                // regions. For now we simply emit an error message.
                if let Err(e) = resp_writer.write_all(err.as_resp().as_bytes()) {
                    error!("failed to write response: {e}");
                }
            }
            resp_writer.bytes_written() + data_writer.bytes_written()
        }
    }
}

#[cfg(test)]
mod tests {
    use std::mem::size_of;
    use std::mem::size_of_val;
    use std::rc::Rc;

    use cros_async::Executor;
    use disk::SingleFileDisk;
    use tempfile::tempfile;
    use virtio_sys::virtio_scsi::virtio_scsi_cmd_req;
    use virtio_sys::virtio_scsi::VIRTIO_SCSI_S_OK;
    use vm_memory::GuestAddress;
    use vm_memory::GuestMemory;

    use crate::virtio::create_descriptor_chain;
    use crate::virtio::scsi::constants::READ_10;
    use crate::virtio::DescriptorType;

    use super::*;

    fn setup_disk(disk_size: u64, ex: &Executor) -> (SingleFileDisk, Vec<u8>) {
        let mut file_content = vec![0; disk_size as usize];
        for i in 0..disk_size {
            file_content[i as usize] = (i % 10) as u8;
        }
        let mut f = tempfile().unwrap();
        f.set_len(disk_size).unwrap();
        f.write_all(file_content.as_slice()).unwrap();
        let af = SingleFileDisk::new(f, ex).expect("Failed to create SFD");
        (af, file_content)
    }

    fn build_read_req_header(start_lba: u8, xfer_blocks: u8) -> virtio_scsi_cmd_req {
        let mut cdb = [0; 32];
        cdb[0] = READ_10;
        cdb[5] = start_lba;
        cdb[8] = xfer_blocks;
        virtio_scsi_cmd_req {
            lun: [1, 0, 0, 0, 0, 0, 0, 0],
            cdb,
            ..Default::default()
        }
    }

    fn setup_desciptor_chain(
        start_lba: u8,
        xfer_blocks: u8,
        block_size: u32,
        mem: &Rc<GuestMemory>,
    ) -> DescriptorChain {
        let req_hdr = build_read_req_header(start_lba, xfer_blocks);
        let xfer_bytes = xfer_blocks as u32 * block_size;
        create_descriptor_chain(
            mem,
            GuestAddress(0x100),  // Place descriptor chain at 0x100.
            GuestAddress(0x1000), // Describe buffer at 0x1000.
            vec![
                // Request header
                (DescriptorType::Readable, size_of_val(&req_hdr) as u32),
                // Response header
                (
                    DescriptorType::Writable,
                    size_of::<virtio_scsi_cmd_resp>() as u32,
                ),
                (DescriptorType::Writable, xfer_bytes),
            ],
            0,
        )
        .expect("create_descriptor_chain failed")
    }

    fn read_blocks(
        ex: &Executor,
        af: Box<SingleFileDisk>,
        start_lba: u8,
        xfer_blocks: u8,
        block_size: u32,
    ) -> (virtio_scsi_cmd_resp, Vec<u8>) {
        let xfer_bytes = xfer_blocks as u32 * block_size;
        let mem = Rc::new(
            GuestMemory::new(&[(GuestAddress(0u64), 4 * 1024 * 1024)])
                .expect("Creating guest memory failed."),
        );
        let req_hdr = build_read_req_header(start_lba, xfer_blocks);
        mem.write_obj_at_addr(req_hdr, GuestAddress(0x1000))
            .expect("writing req failed");

        let mut avail_desc = setup_desciptor_chain(0, xfer_blocks, block_size, &mem);

        let logical_unit = LogicalUnit {
            max_lba: 0x1000,
            block_size,
            read_only: false,
        };
        let queue_type = QueueType::Request(af);
        ex.run_until(process_one_request(
            &mut avail_desc,
            &queue_type,
            logical_unit,
        ))
        .expect("running executor failed");
        let resp_offset = GuestAddress((0x1000 + size_of::<virtio_scsi_cmd_resp>()) as u64);
        let resp = mem
            .read_obj_from_addr::<virtio_scsi_cmd_resp>(resp_offset)
            .unwrap();
        let dataout_offset = GuestAddress(
            (0x1000 + size_of::<virtio_scsi_cmd_req>() + size_of::<virtio_scsi_cmd_resp>()) as u64,
        );
        let dataout_slice = mem
            .get_slice_at_addr(dataout_offset, xfer_bytes as usize)
            .unwrap();
        let mut dataout = vec![0; xfer_bytes as usize];
        dataout_slice.copy_to(&mut dataout);
        (resp, dataout)
    }

    fn test_read_blocks(blocks: u8, start_lba: u8, xfer_blocks: u8, block_size: u32) {
        let ex = Executor::new().expect("creating an executor failed");
        let file_len = blocks as u64 * block_size as u64;
        let xfer_bytes = xfer_blocks as usize * block_size as usize;
        let start_off = start_lba as usize * block_size as usize;

        let (af, file_content) = setup_disk(file_len, &ex);
        let (resp, dataout) = read_blocks(&ex, Box::new(af), start_lba, xfer_blocks, block_size);

        let sense_len = resp.sense_len;
        assert_eq!(sense_len, 0);
        assert_eq!(resp.status, VIRTIO_SCSI_S_OK as u8);
        assert_eq!(resp.response, GOOD);

        assert_eq!(&dataout, &file_content[start_off..(start_off + xfer_bytes)]);
    }

    #[test]
    fn read_first_blocks() {
        // Read the first 3 blocks of a 8-block device.
        let blocks = 8u8;
        let start_lba = 0u8;
        let xfer_blocks = 3u8;

        test_read_blocks(blocks, start_lba, xfer_blocks, 64u32);
        test_read_blocks(blocks, start_lba, xfer_blocks, 128u32);
        test_read_blocks(blocks, start_lba, xfer_blocks, 512u32);
    }

    #[test]
    fn read_middle_blocks() {
        // Read 3 blocks from the 2nd block in the 8-block device.
        let blocks = 8u8;
        let start_lba = 1u8;
        let xfer_blocks = 3u8;

        test_read_blocks(blocks, start_lba, xfer_blocks, 64u32);
        test_read_blocks(blocks, start_lba, xfer_blocks, 128u32);
        test_read_blocks(blocks, start_lba, xfer_blocks, 512u32);
    }
}
