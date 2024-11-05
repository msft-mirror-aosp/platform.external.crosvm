// Copyright 2024 The ChromiumOS Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use std::collections::BTreeMap;
use std::fs::File;
use std::path::PathBuf;

use base::sched_attr;
use base::sched_setattr;
use base::warn;
use base::Error;

use crate::pci::CrosvmDeviceId;
use crate::BusAccessInfo;
use crate::BusDevice;
use crate::DeviceId;
use crate::Suspendable;

const CPUFREQ_GOV_SCALE_FACTOR_DEFAULT: u32 = 100;
const CPUFREQ_GOV_SCALE_FACTOR_SCHEDUTIL: u32 = 80;

const SCHED_FLAG_RESET_ON_FORK: u64 = 0x1;
const SCHED_FLAG_KEEP_POLICY: u64 = 0x08;
const SCHED_FLAG_KEEP_PARAMS: u64 = 0x10;
const SCHED_FLAG_UTIL_CLAMP_MIN: u64 = 0x20;

const VCPUFREQ_CUR_PERF: u32 = 0x0;
const VCPUFREQ_SET_PERF: u32 = 0x4;
const VCPUFREQ_FREQTBL_LEN: u32 = 0x8;
const VCPUFREQ_FREQTBL_SEL: u32 = 0xc;
const VCPUFREQ_FREQTBL_RD: u32 = 0x10;
const VCPUFREQ_PERF_DOMAIN: u32 = 0x14;

const SCHED_FLAG_KEEP_ALL: u64 = SCHED_FLAG_KEEP_POLICY | SCHED_FLAG_KEEP_PARAMS;
const SCHED_SCALE_CAPACITY: u32 = 1024;

/// Upstream linux compatible version of the virtual cpufreq interface
pub struct VirtCpufreqV2 {
    vcpu_freq_table: Vec<u32>,
    pcpu_fmax: u32,
    pcpu_capacity: u32,
    pcpu: u32,
    util_factor: u32,
    freqtbl_sel: u32,
    vcpu_domain: u32,
    domain_uclamp_min: Option<File>,
    domain_uclamp_max: Option<File>,
    vcpu_fmax: u32,
    vcpu_capacity: u32,
    vcpu_relative_capacity: u32,
}

fn get_cpu_info(cpu_id: u32, property: &str) -> Result<u32, Error> {
    let path = format!("/sys/devices/system/cpu/cpu{cpu_id}/{property}");
    std::fs::read_to_string(path)?
        .trim()
        .parse()
        .map_err(|_| Error::new(libc::EINVAL))
}

fn get_cpu_info_str(cpu_id: u32, property: &str) -> Result<String, Error> {
    let path = format!("/sys/devices/system/cpu/cpu{cpu_id}/{property}");
    std::fs::read_to_string(path).map_err(|_| Error::new(libc::EINVAL))
}

fn get_cpu_capacity(cpu_id: u32) -> Result<u32, Error> {
    get_cpu_info(cpu_id, "cpu_capacity")
}

fn get_cpu_maxfreq_khz(cpu_id: u32) -> Result<u32, Error> {
    get_cpu_info(cpu_id, "cpufreq/cpuinfo_max_freq")
}

fn get_cpu_curfreq_khz(cpu_id: u32) -> Result<u32, Error> {
    get_cpu_info(cpu_id, "cpufreq/scaling_cur_freq")
}

fn get_cpu_util_factor(cpu_id: u32) -> Result<u32, Error> {
    let gov = get_cpu_info_str(cpu_id, "cpufreq/scaling_governor")?;
    match gov.trim() {
        "schedutil" => Ok(CPUFREQ_GOV_SCALE_FACTOR_SCHEDUTIL),
        _ => Ok(CPUFREQ_GOV_SCALE_FACTOR_DEFAULT),
    }
}

impl VirtCpufreqV2 {
    pub fn new(
        pcpu: u32,
        cpu_frequencies: BTreeMap<usize, Vec<u32>>,
        vcpu_domain_path: Option<PathBuf>,
        vcpu_domain: u32,
        vcpu_capacity: u32,
    ) -> Self {
        let pcpu_capacity = get_cpu_capacity(pcpu).expect("Error reading capacity");
        let pcpu_fmax = get_cpu_maxfreq_khz(pcpu).expect("Error reading max freq");
        let util_factor = get_cpu_util_factor(pcpu).expect("Error getting util factor");
        let vcpu_freq_table = cpu_frequencies.get(&(pcpu as usize)).unwrap().clone();
        let freqtbl_sel = 0;
        let mut domain_uclamp_min = None;
        let mut domain_uclamp_max = None;
        // The vcpu_capacity passed in is normalized for frequency, reverse the normalization to
        // get the performance per clock ratio between the vCPU and the pCPU its running on. This
        // "relative capacity" is an approximation of the delta in IPC (Instructions per Cycle)
        // between the pCPU vs vCPU running a usecase containing a mix of instruction types.
        let vcpu_fmax = vcpu_freq_table.clone().into_iter().max().unwrap();
        let vcpu_relative_capacity =
            u32::try_from(u64::from(vcpu_capacity) * u64::from(pcpu_fmax) / u64::from(vcpu_fmax))
                .unwrap();

        if let Some(cgroup_path) = &vcpu_domain_path {
            domain_uclamp_min = Some(
                File::create(cgroup_path.join("cpu.uclamp.min")).unwrap_or_else(|err| {
                    panic!(
                        "Err: {}, Unable to open: {}",
                        err,
                        cgroup_path.join("cpu.uclamp.min").display()
                    )
                }),
            );
            domain_uclamp_max = Some(
                File::create(cgroup_path.join("cpu.uclamp.max")).unwrap_or_else(|err| {
                    panic!(
                        "Err: {}, Unable to open: {}",
                        err,
                        cgroup_path.join("cpu.uclamp.max").display()
                    )
                }),
            );
        }

        VirtCpufreqV2 {
            vcpu_freq_table,
            pcpu_fmax,
            pcpu_capacity,
            pcpu,
            util_factor,
            freqtbl_sel,
            vcpu_domain,
            domain_uclamp_min,
            domain_uclamp_max,
            vcpu_fmax,
            vcpu_capacity,
            vcpu_relative_capacity,
        }
    }
}

impl BusDevice for VirtCpufreqV2 {
    fn device_id(&self) -> DeviceId {
        CrosvmDeviceId::VirtCpufreq.into()
    }

    fn debug_label(&self) -> String {
        "VirtCpufreq Device".to_owned()
    }

    fn read(&mut self, info: BusAccessInfo, data: &mut [u8]) {
        if data.len() != std::mem::size_of::<u32>() {
            warn!(
                "{}: unsupported read length {}, only support 4bytes read",
                self.debug_label(),
                data.len()
            );
            return;
        }

        let val = match info.offset as u32 {
            VCPUFREQ_CUR_PERF => match get_cpu_curfreq_khz(self.pcpu) {
                Ok(freq) => u32::try_from(
                    u64::from(freq) * u64::from(self.pcpu_capacity)
                        / u64::from(self.vcpu_relative_capacity),
                )
                .unwrap(),
                Err(_) => 0,
            },
            VCPUFREQ_FREQTBL_LEN => self.vcpu_freq_table.len() as u32,
            VCPUFREQ_PERF_DOMAIN => self.vcpu_domain,
            VCPUFREQ_FREQTBL_RD => *self
                .vcpu_freq_table
                .get(self.freqtbl_sel as usize)
                .unwrap_or(&0),
            _ => {
                warn!("{}: unsupported read address {}", self.debug_label(), info);
                return;
            }
        };

        let val_arr = val.to_ne_bytes();
        data.copy_from_slice(&val_arr);
    }

    fn write(&mut self, info: BusAccessInfo, data: &[u8]) {
        let val: u32 = match data.try_into().map(u32::from_ne_bytes) {
            Ok(v) => v,
            Err(e) => {
                warn!(
                    "{}: unsupported write length {:#}, only support 4bytes write",
                    self.debug_label(),
                    e
                );
                return;
            }
        };

        match info.offset as u32 {
            VCPUFREQ_SET_PERF => {
                // Util margin depends on the cpufreq governor on the host
                let util_raw = match u32::try_from(
                    u64::from(self.vcpu_capacity) * u64::from(val) / u64::from(self.vcpu_fmax),
                ) {
                    Ok(util) => util,
                    Err(e) => {
                        warn!("Potential overflow {:#}", e);
                        SCHED_SCALE_CAPACITY
                    }
                };

                let util = util_raw * self.util_factor / CPUFREQ_GOV_SCALE_FACTOR_DEFAULT;

                if let (Some(domain_uclamp_min), Some(domain_uclamp_max)) =
                    (&mut self.domain_uclamp_min, &mut self.domain_uclamp_max)
                {
                    use std::io::Write;
                    let val = util as f32 * 100.0 / SCHED_SCALE_CAPACITY as f32;
                    let val_formatted = format!("{:4}", val).into_bytes();

                    if self.vcpu_fmax != self.pcpu_fmax {
                        if let Err(e) = domain_uclamp_max.write(&val_formatted) {
                            warn!("Error setting uclamp_max: {:#}", e);
                        }
                    }
                    if let Err(e) = domain_uclamp_min.write(&val_formatted) {
                        warn!("Error setting uclamp_min: {:#}", e);
                    }
                } else {
                    let mut sched_attr = sched_attr::default();
                    sched_attr.sched_flags =
                        SCHED_FLAG_KEEP_ALL | SCHED_FLAG_UTIL_CLAMP_MIN | SCHED_FLAG_RESET_ON_FORK;
                    sched_attr.sched_util_min = util;

                    if self.vcpu_fmax != self.pcpu_fmax {
                        sched_attr.sched_util_max = util;
                    } else {
                        sched_attr.sched_util_max = SCHED_SCALE_CAPACITY;
                    }

                    if let Err(e) = sched_setattr(0, &mut sched_attr, 0) {
                        panic!("{}: Error setting util value: {:#}", self.debug_label(), e);
                    }
                }
            }
            VCPUFREQ_FREQTBL_SEL => self.freqtbl_sel = val,
            _ => {
                warn!("{}: unsupported read address {}", self.debug_label(), info);
            }
        }
    }
}

impl Suspendable for VirtCpufreqV2 {}
