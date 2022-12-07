// Copyright 2022 The ChromiumOS Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

//! crate for the vmm-swap feature.

#![deny(missing_docs)]

mod file;
mod logger;
// this is public only for integration tests.
pub mod page_handler;
mod processes;
// this is public only for integration tests.
pub mod userfaultfd;

use std::io::stderr;
use std::io::stdout;
use std::ops::Range;
use std::path::Path;
use std::path::PathBuf;

use anyhow::bail;
use anyhow::Context;
use base::debug;
use base::error;
use base::info;
use base::pagesize;
use base::syslog;
use base::unix::process::fork_process;
use base::unix::process::Child;
use base::warn;
use base::AsRawDescriptor;
use base::AsRawDescriptors;
use base::EventToken;
use base::MemoryMapping;
use base::MemoryMappingBuilder;
use base::Tube;
use base::WaitContext;
use data_model::VolatileMemory;
use minijail::Minijail;
use serde::Deserialize;
use serde::Serialize;
use vm_memory::GuestMemory;

use crate::logger::PageFaultEventLogger;
use crate::page_handler::PageHandler;
use crate::processes::freeze_all_processes;
use crate::userfaultfd::UffdError;
use crate::userfaultfd::UffdEvent;
use crate::userfaultfd::Userfaultfd;

/// Current status of vmm-swap.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Status {
    /// vmm-swap is ready. userfaultfd is disabled until swap out.
    Ready,
    /// swap-out is in progress. this is not used for now because the monitor process runs in a
    /// single thread.
    InProgress,
    /// swap out succeeded.
    Active {
        /// time taken for swap-out.
        swap_time_ms: u128,
        /// count of pages on RAM.
        resident_pages: usize,
        /// count of pages in swap files.
        swap_pages: usize,
    },
    /// swap out failed.
    Failed,
}

/// commands used in vmm-swap feature internally between [SwapController] and [monitor_process].
#[derive(Serialize, Deserialize, Debug)]
enum Command {
    Enable,
    Disable,
    Exit,
    Status,
    StartPageFaultLogging,
}

/// [SwapController] provides APIs to control vmm-swap.
pub struct SwapController {
    child_process: Child,
    tube: Tube,
    _dummy_page: MemoryMapping,
}

impl SwapController {
    /// Launch a monitor process for vmm-swap and return a controller.
    ///
    /// Pages on the [GuestMemory] are registered to userfaultfd to track pagefault events.
    ///
    /// # Arguments
    ///
    /// * `guest_memory` - fresh new [GuestMemory]. Any pages on the [GuestMemory] must not be
    ///   touched.
    /// * `swap_dir` - directory to store swap files.
    pub fn launch(guest_memory: GuestMemory, swap_dir: PathBuf) -> anyhow::Result<Self> {
        info!("vmm-swap is enabled. launch monitor process.");

        let dummy_page = MemoryMappingBuilder::new(pagesize())
            .build()
            .context("allocate dummy page")?;
        let dummy_page_addr = dummy_page
            .get_ref::<u8>(0)
            .context("get base address of dummy page")?
            .as_mut_ptr() as usize;

        let mut keep_rds = vec![stdout().as_raw_descriptor(), stderr().as_raw_descriptor()];

        let (tube_main_process, tube_monitor_process) = Tube::pair().context("create swap tube")?;
        keep_rds.push(tube_monitor_process.as_raw_descriptor());

        syslog::push_descriptors(&mut keep_rds);
        keep_rds.extend(guest_memory.as_raw_descriptors());

        let userfaultfd = Userfaultfd::new().context("create userfaultfd")?;
        keep_rds.push(userfaultfd.as_raw_descriptor());

        // TODO(b/258351526): setup minijail details
        let jail = Minijail::new().context("create minijail")?;

        // Start a page fault monitoring process (this will be the first child process of the
        // current process)
        let child_process =
            fork_process(jail, keep_rds, Some(String::from("swap monitor")), || {
                // userfaultfd triggeres UFFD_EVENT_FORK event only while at least 1 page is
                // registered to it. This is a workaround for it to register dummy page which is
                // never touched. We have to register the dummy page after the monitor process forks
                // not to be blocked and before device processes fork to catch the fork event with
                // userfaultfd.
                // safe because no one access dummy_page.
                if let Err(e) = unsafe { userfaultfd.register(dummy_page_addr, pagesize()) } {
                    panic!("failed to register dummy page to userfaultfd: {:?}", e);
                }
                if let Err(e) =
                    monitor_process(tube_monitor_process, guest_memory, userfaultfd, swap_dir)
                {
                    panic!("page_fault_handler_thread exited with error: {:?}", e)
                }
            })
            .context("fork monitor process")?;

        // send first status request to the monitor process and wait for the response until setup on
        // the monitor process completes.
        tube_main_process.send(&Command::Status)?;
        match tube_main_process.recv().context("recv initial status")? {
            Status::Ready => {
                // The initial state of swap status is Ready and this is a signal that the
                // monitoring process completes setup and is running.
            }
            status => {
                bail!("initial state is not Ready, but {:?}", status);
            }
        };

        Ok(Self {
            child_process,
            tube: tube_main_process,
            _dummy_page: dummy_page,
        })
    }

    /// Swap out all the guest memory and enable monitoring page faults.
    ///
    /// This returns as soon as it succeeds to send request to the monitor process.
    pub fn enable(&self) -> anyhow::Result<()> {
        self.tube
            .send(&Command::Enable)
            .context("send swap enable request")?;
        Ok(())
    }

    /// Swap in all the guest memory and disable monitoring page faults.
    ///
    /// This returns as soon as it succeeds to send request to the monitor process.
    pub fn disable(&self) -> anyhow::Result<()> {
        self.tube
            .send(&Command::Disable)
            .context("send swap disable request")?;
        Ok(())
    }

    /// Return current swap status.
    ///
    /// This blocks until response from the monitor process arrives to the main process.
    pub fn status(&self) -> anyhow::Result<Status> {
        self.tube
            .send(&Command::Status)
            .context("send swap status request")?;
        let status = self.tube.recv().context("receive swap status")?;
        Ok(status)
    }

    /// Start page fault logging.
    ///
    /// This returns as soon as it succeeds to send request to the monitor process.
    /// Requests will be ignored if it is already start logging.
    pub fn start_page_fault_logging(&self) -> anyhow::Result<()> {
        self.tube
            .send(&Command::StartPageFaultLogging)
            .context("send page fault logging request")?;
        Ok(())
    }

    /// Shutdown the monitor process.
    ///
    /// This blocks until the monitor process exits.
    ///
    /// This should be called once.
    pub fn exit(self) -> anyhow::Result<()> {
        self.tube
            .send(&Command::Exit)
            .context("send exit command")?;
        self.child_process
            .wait()
            .context("wait monitor process shutdown")?;
        Ok(())
    }
}

#[derive(EventToken)]
enum Token {
    UffdEvents(u32),
    Command,
}

struct UffdList<'a> {
    list: Vec<Userfaultfd>,
    wait_ctx: &'a WaitContext<Token>,
}

impl<'a> UffdList<'a> {
    const ID_MAIN_UFFD: u32 = 0;

    fn new(main_uffd: Userfaultfd, wait_ctx: &'a WaitContext<Token>) -> Self {
        Self {
            list: vec![main_uffd],
            wait_ctx,
        }
    }

    fn register(&mut self, uffd: Userfaultfd) -> anyhow::Result<()> {
        let id_uffd = self
            .list
            .len()
            .try_into()
            .context("too many userfaultfd forked")?;

        self.wait_ctx
            .add(&uffd, Token::UffdEvents(id_uffd))
            .context("add to wait context")?;
        self.list.push(uffd);

        Ok(())
    }

    fn get(&self, id: u32) -> Option<&Userfaultfd> {
        self.list.get(id as usize)
    }

    fn main_uffd(&self) -> &Userfaultfd {
        &self.list[Self::ID_MAIN_UFFD as usize]
    }

    fn get_list(&self) -> &[Userfaultfd] {
        &self.list
    }
}

/// Register all the regions to all the userfaultfd
///
/// This is public only for integration tests.
///
/// # Arguments
///
/// * `regions` - the list of address range of regions.
/// * `uffds` - the reference to the list of [Userfaultfd] for all the processes which may touch
///   the `address_range` to be registered.
///
/// # Safety
///
/// Each address range in `regions` must be from guest memory.
///
/// The `uffds` must cover all the processes which may touch the `address_range`. otherwise some
/// pages are zeroed by kernel on the unregistered process instead of swapping in from the swap
/// file.
#[deny(unsafe_op_in_unsafe_fn)]
pub unsafe fn register_regions(
    regions: &[Range<usize>],
    uffds: &[Userfaultfd],
) -> anyhow::Result<()> {
    for address_range in regions {
        for uffd in uffds {
            // safe because the range is from the guest memory region. Even after the memory
            // is removed by `MADV_REMOVE` at [PageHandler::swap_out()], the content will be
            // swapped in from the swap file safely on a page fault.
            let result = unsafe {
                uffd.register(address_range.start, address_range.end - address_range.start)
            };
            match result {
                Ok(_) => Ok(()),
                // Userfaultfd returns `ENOMEM` if the corresponding process dies or run as
                // another program by `exec` system call. crosvm just skip the userfaultfd.
                Err(UffdError::SystemError(errno)) if errno as i32 == libc::ENOMEM => Ok(()),
                Err(e) => Err(e),
            }?;
        }
    }
    Ok(())
}

/// Unregister all the regions from all the userfaultfd.
///
/// This is public only for integration tests.
///
/// # Arguments
///
/// * `regions` - the list of address range of regions.
/// * `uffds` - the reference to the list of registered [Userfaultfd].
pub fn unregister_regions(regions: &[Range<usize>], uffds: &[Userfaultfd]) -> anyhow::Result<()> {
    for address_range in regions {
        for uffd in uffds {
            // `UFFDIO_UNREGISTER` unblocks any threads currently waiting on the region and
            // remove page fault events on the region from the userfaultfd event queue.
            let result =
                uffd.unregister(address_range.start, address_range.end - address_range.start);
            match result {
                Ok(_) => Ok(()),
                // Userfaultfd returns `ENOMEM` if the corresponding process dies or run as
                // another program by `exec` system call. crosvm just skip the userfaultfd.
                Err(UffdError::SystemError(errno)) if errno as i32 == libc::ENOMEM => Ok(()),
                Err(e) => Err(e),
            }?;
        }
    }
    Ok(())
}

fn regions_from_guest_memory(guest_memory: &GuestMemory) -> Vec<Range<usize>> {
    let mut regions = Vec::new();
    guest_memory
        .with_regions::<_, ()>(|_, _, region_size, host_addr, _, _| {
            regions.push(host_addr..(host_addr + region_size));
            Ok(())
        })
        .unwrap(); // the callback never return error.
    regions
}

fn start_monitoring(
    uffd_list: &mut UffdList,
    guest_memory: &GuestMemory,
    swap_dir: &Path,
) -> anyhow::Result<PageHandler> {
    // Drain the event queue to ensure that the uffds for all forked processes are being monitored.
    let mut new_uffds = Vec::new();
    for uffd in uffd_list.get_list() {
        while let Some(event) = uffd.read_event().context("read event")? {
            if let UffdEvent::Fork { uffd } = event {
                new_uffds.push(uffd.into());
            } else {
                bail!("unexpected uffd event before registering: {:?}", event);
            }
        }
    }
    for uffd in new_uffds {
        uffd_list.register(uffd).context("register uffd")?;
    }

    let regions = regions_from_guest_memory(guest_memory);

    let page_hander = PageHandler::create(swap_dir, &regions).context("enable swap")?;

    // safe because the regions are from guest memory and uffd_list contains all the processes of
    // crosvm.
    unsafe { register_regions(&regions, uffd_list.get_list()) }.context("register regions")?;

    Ok(page_hander)
}

fn disable_monitoring(
    page_handler: PageHandler,
    uffd_list: &UffdList,
    guest_memory: &GuestMemory,
) -> anyhow::Result<usize> {
    let num_pages = page_handler
        .swap_in(uffd_list.main_uffd())
        .context("unregister all regions")?;
    let regions = regions_from_guest_memory(guest_memory);
    unregister_regions(&regions, uffd_list.get_list()).context("unregister regions")?;
    Ok(num_pages)
}

/// the main thread of the monitor process.
fn monitor_process(
    tube: Tube,
    guest_memory: GuestMemory,
    uffd: Userfaultfd,
    swap_dir: PathBuf,
) -> anyhow::Result<()> {
    info!("monitor_process started");

    let wait_ctx = WaitContext::build_with(&[
        (&tube, Token::Command),
        // Even though swap isn't enabled until the enable command is received, it's necessary to
        // start waiting on the main uffd here so that uffd fork events can be processed, because
        // child processes will block until their corresponding uffd fork event is read.
        (&uffd, Token::UffdEvents(UffdList::ID_MAIN_UFFD)),
    ])
    .context("create wait context")?;

    let mut uffd_list = UffdList::new(uffd, &wait_ctx);
    let mut lastest_swap_out_time_ms = None;
    let mut page_handler_opt: Option<PageHandler> = None;
    let mut page_fault_logger: Option<PageFaultEventLogger> = None;

    'wait: loop {
        let events = wait_ctx.wait().context("wait poll events")?;

        for event in events.iter() {
            match event.token {
                Token::UffdEvents(id_uffd) => {
                    // userfaultfd does not work as level triggered but as edge triggered. We need
                    // to read all the events in the userfaultfd here.
                    while let Some((uffd, event)) = {
                        // get uffd on every loop because [UffdList::register()] called in this loop
                        // is mutable.
                        let uffd = uffd_list
                            .get(id_uffd)
                            .with_context(|| format!("uffd is not found for idx: {}", id_uffd))?;
                        // TODO(kawasin): Use [userfaultfd::Uffd::read_events()] for performance.
                        uffd.read_event()
                            .context("read userfaultfd event")?
                            .map(|event| (uffd, event))
                    } {
                        match event {
                            UffdEvent::Pagefault { addr, .. } => {
                                if let Some(ref mut page_fault_logger) = page_fault_logger {
                                    page_fault_logger.log_page_fault(addr as usize);
                                }
                                if let Some(ref mut page_handler) = page_handler_opt {
                                    page_handler
                                        .handle_page_fault(uffd, addr as usize)
                                        .context("handle fault")?;
                                } else {
                                    warn!("page fault event while handler is none");
                                }
                            }
                            UffdEvent::Fork { uffd } => {
                                debug!("new fork uffd: {:?} from id_uffd: {:?}", uffd, id_uffd);
                                if page_handler_opt.is_none() {
                                    uffd_list
                                        .register(uffd.into())
                                        .context("register forked uffd")?;
                                } else {
                                    // TODO(b/259009757): Crosvm does not support forking child
                                    // processes while vmm-swap is enabled. There are
                                    // synchronization issues here around registering userfaultfd
                                    // regions with the child process as well as ensuring the child
                                    // is properly paused that haven't been worked out. However,
                                    // there is currently no use case for swap + hotplug, so this
                                    // can be solved later.
                                    bail!("child process is forked while swap is enabled");
                                }
                            }
                            UffdEvent::Remove { start, end } => {
                                if let Some(ref mut page_handler) = page_handler_opt {
                                    page_handler
                                        .handle_page_remove(start as usize, end as usize)
                                        .context("handle fault")?;
                                } else {
                                    warn!("page remove event while handler is none");
                                }
                            }
                            event => {
                                bail!("unsupported UffdEvent: {:?}", event);
                            }
                        }
                    }
                }
                Token::Command => match tube.recv::<Command>().context("recv swap command")? {
                    Command::Enable => {
                        let _processes_guard =
                            freeze_all_processes().context("freeze processes")?;

                        if page_handler_opt.is_none() {
                            info!("enable monitoring page faults");
                            page_handler_opt =
                                Some(start_monitoring(&mut uffd_list, &guest_memory, &swap_dir)?);
                        }
                        let page_handler = page_handler_opt.as_mut().unwrap();

                        info!("start swapping out");
                        let t0 = std::time::Instant::now();
                        let mut num_pages = 0;
                        let result = guest_memory.with_regions::<_, anyhow::Error>(
                            |_, _, _, host_addr, shm, shm_offset| {
                                // safe because all the regions are registered to all userfaultfd
                                // and page fault events are handled by PageHandler.
                                num_pages +=
                                    unsafe { page_handler.swap_out(host_addr, shm, shm_offset) }
                                        .context("swap out")?;
                                Ok(())
                            },
                        );
                        match result {
                            Ok(()) => {
                                let swap_time_ms = t0.elapsed().as_millis();
                                info!("swap out {} pages in {} ms", num_pages, swap_time_ms);
                                if page_handler.compute_resident_pages() > 0 {
                                    error!(
                                        "active page is not zero just after swap out but {} pages",
                                        page_handler.compute_resident_pages()
                                    );
                                }
                                lastest_swap_out_time_ms = Some(swap_time_ms);
                            }
                            Err(e) => {
                                error!("failed to swapping out the state: {}", e);
                                lastest_swap_out_time_ms = None;
                            }
                        }
                    }
                    Command::Disable => {
                        if let Some(page_handler) = page_handler_opt.take() {
                            let t0 = std::time::Instant::now();
                            let num_pages =
                                disable_monitoring(page_handler, &uffd_list, &guest_memory)?;
                            let time_took_ms = t0.elapsed().as_millis();
                            info!(
                                "swap in all {} pages in {} ms. swap disabled.",
                                num_pages, time_took_ms
                            );
                        } else {
                            warn!("swap is already disabled.");
                        }
                    }
                    Command::Exit => {
                        break 'wait;
                    }
                    Command::Status => {
                        let status = if let Some(ref page_handler) = page_handler_opt {
                            if let Some(swap_time_ms) = lastest_swap_out_time_ms {
                                Status::Active {
                                    swap_time_ms,
                                    resident_pages: page_handler.compute_resident_pages(),
                                    swap_pages: page_handler.compute_swap_pages(),
                                }
                            } else {
                                Status::Failed
                            }
                        } else {
                            Status::Ready
                        };
                        tube.send(&status).context("send status response")?;
                        info!("swap status: {:?}.", status);
                    }
                    Command::StartPageFaultLogging => {
                        if page_fault_logger.is_none() {
                            page_fault_logger = Some(
                                PageFaultEventLogger::create(&swap_dir, &guest_memory)
                                    .context("create page fault logger")?,
                            )
                        }
                    }
                },
            };
        }
    }
    Ok(())
}
