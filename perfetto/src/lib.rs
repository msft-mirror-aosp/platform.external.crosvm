// Copyright 2023 The ChromiumOS Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

mod bindings;
mod hashes;

pub mod protos {
    include!(concat!(env!("OUT_DIR"), "/perfetto_protos/generated.rs"));
}

use std::ffi::c_void;
use std::ffi::CString;
use std::mem::size_of;
use std::path::Path;
use std::slice;
use std::time::Duration;

pub use bindings::*;
pub use cros_tracing_types::static_strings::StaticString;
use cros_tracing_types::TraceDuration;
use protobuf::Message;
use protos::perfetto_config::trace_config::BufferConfig;
use protos::perfetto_config::trace_config::DataSource;
use protos::perfetto_config::trace_config::IncrementalStateConfig;
use protos::perfetto_config::DataSourceConfig;
use protos::perfetto_config::TraceConfig;
use protos::perfetto_config::TrackEventConfig;
use zerocopy::AsBytes;
use zerocopy::FromBytes;
use zerocopy::FromZeroes;

/// Randomly generated GUID to help locate the AOT header.
const HEADER_MAGIC: &[u8; 16] = b"\x8d\x10\xa3\xee\x79\x1f\x47\x25\xb2\xb8\xb8\x9f\x85\xe7\xd6\x7c";

/// The optional header written ahead of the trace data.
#[repr(C)]
#[derive(Copy, Clone, AsBytes, FromZeroes, FromBytes)]
struct TraceHeader {
    magic: [u8; 16],
    data_size: u64,
    data_checksum_sha256: [u8; 32],
}

#[macro_export]
macro_rules! zero {
    ($x:ident) => {
        0
    };
}

/// Helper macro for perfetto_tags
#[macro_export]
macro_rules! tag_or_empty_string {
    () => {
        "\0".as_ptr() as *const std::ffi::c_char
    };
    ($tag:expr) => {
        concat!($tag, "\0").as_ptr() as *const std::ffi::c_char
    };
}

/// Macro for creating an array of const char * for perfetto tags.
#[macro_export]
macro_rules! perfetto_tags {
    () => {
        [
            tag_or_empty_string!(),
            tag_or_empty_string!(),
            tag_or_empty_string!(),
            tag_or_empty_string!(),
        ]
    };
    ($tag0:expr) => {
        [
            tag_or_empty_string!($tag0),
            tag_or_empty_string!(),
            tag_or_empty_string!(),
            tag_or_empty_string!(),
        ]
    };
    ($tag0:expr, $tag1:expr) => {
        [
            tag_or_empty_string!($tag0),
            tag_or_empty_string!($tag1),
            tag_or_empty_string!(),
            tag_or_empty_string!(),
        ]
    };
    ($tag0:expr, $tag1:expr, $tag2:expr) => {
        [
            tag_or_empty_string!($tag0),
            tag_or_empty_string!($tag1),
            tag_or_empty_string!($tag2),
            tag_or_empty_string!(),
        ]
    };
    ($tag0:expr, $tag1:expr, $tag2:expr, $tag3:expr) => {
        [
            tag_or_empty_string!($tag0),
            tag_or_empty_string!($tag1),
            tag_or_empty_string!($tag2),
            tag_or_empty_string!($tag3),
        ]
    };
}

/// Main macro to be called by any crate wanting to use perfetto tracing. It
/// should be called once in your crate outside of any function.
///
/// # Arguments
///  * `module_path` - is the module path where this
///  * The remaining arguments are an arbitrary list of triples that describe the tracing
///    categories. They are supplied flattened (e.g. ((a, b, c), (d, e, f)) => (a, b, c, d, e, f).
///    Each triple contains:
///         - the category name (this is the same name/ident that will be passed to trace point
///           macros).
///         - a free form text description of the category.
///         - the tag set for this category (generated by calling perfetto_tags).
///
/// # Examples
/// ```no_run
/// setup_perfetto!(
///     tracing,
///     mycrate,
///     "General trace points for my crate",
///     perfetto_tags!(),
///     debug,
///     "Debug trace points",
///     perfetto_tags!("debug"))
/// ```
#[macro_export]
macro_rules! setup_perfetto {
    ($mod:ident, $($cat:ident, $description:expr, $tags:expr),+) => {
        #[allow(non_camel_case_types)]
        #[derive(Copy, Clone)]
        pub enum PerfettoCategory {
            $($cat,)+
            // Hacky way to get the count of the perfetto categories.
            CATEGORY_COUNT,
        }

        /// Const array of perfetto categories that will be passed to perfetto api.
        pub const CATEGORIES: [&ctrace_category; PerfettoCategory::CATEGORY_COUNT as usize] = [
            $(
                &ctrace_category {
                    client_index: PerfettoCategory::$cat as u64,
                    instances_callback: Some(instances_callback),
                    name: concat!(stringify!($cat), "\0").as_ptr() as *const std::ffi::c_char,
                    description: concat!($description, "\0").as_ptr() as *const std::ffi::c_char,
                    tags: $tags
                },
            )+
        ];

        /// Base offset into the global list of categories where our categories live.
        pub static PERFETTO_CATEGORY_BASE: std::sync::atomic::AtomicU64 =
            std::sync::atomic::AtomicU64::new(0);

        /// Active trace instance bitmaps for each of our categories. We use a u32 because the
        /// cperfetto API uses a u32, but really only 8 traces can be active at a time.
        pub static PERFETTO_CATEGORY_INSTANCES:
            [std::sync::atomic::AtomicU32; PerfettoCategory::CATEGORY_COUNT as usize] = [
                $(
                    // Note, we pass $cat to the zero! macro here, which always just returns
                    // 0, because it's impossible to iterate over $cat unless $cat is used.
                    std::sync::atomic::AtomicU32::new($crate::zero!($cat)),
                )+
        ];

        /// Register the perfetto categories defined by this macro with the perfetto shared
        /// library. This should be called once at process startup.
        pub fn register_categories() {
            PERFETTO_CATEGORY_BASE.store(
                unsafe {
                    ctrace_register_categories(
                        CATEGORIES.as_ptr() as *const *const ctrace_category,
                        CATEGORIES.len() as u64,
                    )
                },
                std::sync::atomic::Ordering::SeqCst,
            );
        }

        /// Callback from the perfetto shared library when the set of active trace instances for
        /// a given category has changed. Index is the client index of one of our registered
        /// categories.
        extern "C" fn instances_callback(instances: u32, index: u64) {
            PERFETTO_CATEGORY_INSTANCES[index as usize].store(
                instances, std::sync::atomic::Ordering::SeqCst);

            for cb in PERFETTO_PER_TRACE_CALLBACKS.lock().iter() {
                cb();
            }
        }


        static PERFETTO_PER_TRACE_CALLBACKS: sync::Mutex<Vec<fn()>> = sync::Mutex::new(Vec::new());


        pub fn add_per_trace_callback(callback: fn()) {
            PERFETTO_PER_TRACE_CALLBACKS.lock().push(callback);
        }

        /// Create and return a scoped named trace event, which will start at construction and end when
        /// the event goes out of scope and is dropped. Will return None if tracing is disabled for
        /// this category.
        ///
        /// # Examples
        /// ```no_run
        /// {
        ///     let _trace = trace_event!(my_category, "trace_point_name");
        ///     do_some_work();
        /// } // _trace dropped here & records the span.
        /// ```
        #[macro_export]
        macro_rules! trace_event {
            ($category:ident, $name:literal) => {
                {
                    let instances = $mod::PERFETTO_CATEGORY_INSTANCES
                        [$mod::PerfettoCategory::$category as usize]
                        .load(std::sync::atomic::Ordering::SeqCst);

                    if instances != 0 {
                        let category_index = $mod::PERFETTO_CATEGORY_BASE
                            .load(std::sync::atomic::Ordering::SeqCst)
                            + $mod::PerfettoCategory::$category as u64;
                        Some($crate::TraceEvent::new(
                            category_index,
                            instances,
                            concat!($name, "\0").as_ptr() as *const std::ffi::c_char,
                        ))
                    } else {
                        None
                    }
                }
            };
            ($category:ident, $name:expr $(,$t:expr)+) => {
                // Perfetto doesn't support extra detail arguments, so drop
                // them.
                trace_event!($category, $name)
            };
        }

        /// Internal macro used to begin a trace event. Not intended for direct
        /// use by library consumers.
        #[macro_export]
        macro_rules! trace_event_begin {
            ($category:ident, $name:expr) => {
                let instances = $mod::PERFETTO_CATEGORY_INSTANCES
                    [$mod::PerfettoCategory::$category as usize]
                    .load(std::sync::atomic::Ordering::SeqCst);

                if instances != 0 {
                    let category_index = $mod::PERFETTO_CATEGORY_BASE
                        .load(std::sync::atomic::Ordering::SeqCst)
                        + $mod::PerfettoCategory::$category as u64;

                    unsafe {
                        $crate::trace_event_begin(
                            category_index,
                            instances,
                            concat!($name, "\0").as_ptr() as *const std::ffi::c_char,
                        )
                    };
                }
            };
        }

        /// Ends the currently active trace event. Not intended for direct use
        /// by library consumers.
        #[macro_export]
        macro_rules! trace_event_end {
            ($category:ident) => {
                let instances = $mod::PERFETTO_CATEGORY_INSTANCES
                    [$mod::PerfettoCategory::$category as usize]
                        .load(std::sync::atomic::Ordering::SeqCst);

                if instances != 0 {
                    let category_index = $mod::PERFETTO_CATEGORY_BASE
                        .load(std::sync::atomic::Ordering::SeqCst)
                        + $mod::PerfettoCategory::$category as u64;
                    unsafe { $crate::trace_event_end(category_index, instances) }
                }
            };
        }

        /// Creates an async flow but does not start it.
        ///
        /// Internal wrapper for use by cros_async_trace.
        #[macro_export]
        macro_rules! trace_create_async {
            ($category:expr, $name:expr) => {
                {
                    let instances = $mod::PERFETTO_CATEGORY_INSTANCES
                        [$category as usize]
                            .load(std::sync::atomic::Ordering::SeqCst);

                    if instances != 0 {
                        let category_index = $mod::PERFETTO_CATEGORY_BASE
                            .load(std::sync::atomic::Ordering::SeqCst)
                            + $category as u64;

                        let trace_point_name: $crate::StaticString = $name;
                        unsafe {
                            Some($crate::trace_create_async(
                                category_index,
                                instances,
                                trace_point_name.as_ptr(),
                            ))
                        }
                    } else {
                        None
                    }
                }
            }
        }

        /// Starts an existing async flow.
        ///
        /// Internal wrapper for use by cros_async_trace.
        #[macro_export]
        macro_rules! trace_begin_async {
            ($category:expr, $name:expr, $optional_terminating_flow_id:expr) => {
                let instances = $mod::PERFETTO_CATEGORY_INSTANCES
                    [$category as usize]
                        .load(std::sync::atomic::Ordering::SeqCst);

                if instances != 0 {
                    let category_index = $mod::PERFETTO_CATEGORY_BASE
                        .load(std::sync::atomic::Ordering::SeqCst)
                        + $category as u64;

                    if let Some(terminating_flow_id) = $optional_terminating_flow_id {
                        let trace_point_name: $crate::StaticString = $name;
                        // Safe because we guarantee $name is a StaticString (which enforces static
                        // a lifetime for the underlying CString).
                        unsafe {
                            $crate::trace_begin_async(
                                category_index,
                                instances,
                                trace_point_name.as_ptr(),
                                terminating_flow_id,
                            )
                        };
                    }
                }
            }
        }

        /// Pauses a running async flow.
        ///
        /// Internal wrapper for use by cros_async_trace.
        #[macro_export]
        macro_rules! trace_pause_async {
            ($category:expr) => {
                {
                    let instances = $mod::PERFETTO_CATEGORY_INSTANCES
                        [$category as usize]
                            .load(std::sync::atomic::Ordering::SeqCst);

                    if instances != 0 {
                        let category_index = $mod::PERFETTO_CATEGORY_BASE
                            .load(std::sync::atomic::Ordering::SeqCst)
                            + $category as u64;

                        unsafe {
                            // Safe because we are only passing primitives in.
                            Some($crate::trace_pause_async(
                                category_index,
                                instances,
                            ))
                        }
                    } else {
                        None
                    }
                }
            }
        }

        /// Ends a running async flow.
        ///
        /// Internal wrapper for use by cros_async_trace.
        #[macro_export]
        macro_rules! trace_end_async {
            ($category:expr) => {
                let instances = $mod::PERFETTO_CATEGORY_INSTANCES
                    [$category as usize]
                        .load(std::sync::atomic::Ordering::SeqCst);

                if instances != 0 {
                    let category_index = $mod::PERFETTO_CATEGORY_BASE
                        .load(std::sync::atomic::Ordering::SeqCst)
                        + $category as u64;

                    // Safe because we are only passing primitives in.
                    unsafe {
                        $crate::trace_end_async(
                            category_index,
                            instances,
                        )
                    };
                }
            }
        }

        /// Emits a counter with the specified name and value. Note that
        /// Perfetto does NOT average or sample this data, so a high volume of
        /// calls will very quickly fill the trace buffer.
        ///
        /// # Examples
        /// ```no_run
        /// trace_counter!(my_category, "counter_name", 500);
        /// ```
        #[macro_export]
        macro_rules! trace_counter {
            ($category:ident, $name:literal, $value:expr) => {
                let instances = $mod::PERFETTO_CATEGORY_INSTANCES
                    [$mod::PerfettoCategory::$category as usize]
                        .load(std::sync::atomic::Ordering::SeqCst);

                if instances != 0 {
                    let category_index = $mod::PERFETTO_CATEGORY_BASE
                        .load(std::sync::atomic::Ordering::SeqCst)
                        + $mod::PerfettoCategory::$category as u64;

                    // Safe because the counter name is a 'static string.
                    unsafe {
                        $crate::trace_counter(
                            category_index,
                            instances,
                            concat!($name, "\0").as_ptr() as *const std::ffi::c_char,
                            $value,
                        )
                    };
                }
            };

            ($category:ident, $name:expr, $value:expr) => {
                // Required for safety when calling trace_counter.
                let trace_point_name: $crate::StaticString = $name;

                let instances = $mod::PERFETTO_CATEGORY_INSTANCES
                    [$mod::PerfettoCategory::$category as usize]
                        .load(std::sync::atomic::Ordering::SeqCst);

                if instances != 0 {
                    let category_index = $mod::PERFETTO_CATEGORY_BASE
                        .load(std::sync::atomic::Ordering::SeqCst)
                        + $mod::PerfettoCategory::$category as u64;

                    // Safe because we guarantee $name is a StaticString (which enforces static a
                    // lifetime for the underlying CString).
                    unsafe {
                        $crate::trace_counter(
                            category_index,
                            instances,
                            trace_point_name.as_ptr(),
                            $value,
                        )
                    };
                }
            };
        }
    };
}

/// Perfetto supports two backends, a system backend that runs in a dedicated
/// process, and an in process backend. These are selected using this enum.
pub enum BackendType {
    InProcess = BackendType_CTRACE_IN_PROCESS_BACKEND as isize,
    System = BackendType_CTRACE_SYSTEM_BACKEND as isize,
}

/// Initializes the tracing system. Should not be called directly (use
/// `setup_perfetto` instead).
pub fn init_tracing(backend: BackendType) {
    let args = ctrace_init_args {
        api_version: 1,
        backend: backend as u32,
        shmem_size_hint_kb: 0,
        shmem_page_size_hint_kb: 0,
        shmem_batch_commits_duration_ms: 0,
    };
    unsafe { ctrace_init(&args) }
}

/// Rust wrapper for running traces.
pub struct Trace {
    session: ctrace_trace_session_handle,
    trace_stopped: bool,
}

// Safe because the trace session handle can be sent between threads without ill effect.
unsafe impl Sync for Trace {}
unsafe impl Send for Trace {}

impl Trace {
    /// Starts a trace.
    pub fn start(
        duration: TraceDuration,
        buffer_size_kb: u32,
        clear_period: Duration,
        categories: Option<Vec<String>>,
    ) -> anyhow::Result<Self> {
        let mut config = TraceConfig::new();
        let mut incremental_state_config = IncrementalStateConfig::new();
        incremental_state_config.set_clear_period_ms(clear_period.as_millis().try_into()?);
        config.incremental_state_config = Some(incremental_state_config).into();

        let mut buffer = BufferConfig::new();
        buffer.set_size_kb(buffer_size_kb);
        config.buffers.push(buffer);

        let mut data_source = DataSource::new();
        let mut data_source_config = DataSourceConfig::new();
        data_source_config.name = Some("track_event".to_owned());

        if let Some(categories) = categories {
            let mut track_event_config = TrackEventConfig::new();
            track_event_config.enabled_categories = categories;
            track_event_config.disabled_categories.push("*".to_string());
            data_source_config.track_event_config = Some(track_event_config).into();
        }

        data_source.config = Some(data_source_config).into();

        if let TraceDuration::StopIn(trace_duration) = duration {
            config.set_duration_ms(trace_duration.as_millis().try_into()?);
        }

        config.data_sources.push(data_source);

        Ok(Self {
            session: start_trace_from_proto(config)?,
            trace_stopped: false,
        })
    }

    /// Ends a trace and writes the results to the provided file path.
    pub fn end(mut self, output: &Path) {
        // Safe because the session is guaranteed to be valid by self.
        unsafe { end_trace(self.session, output) }
        self.trace_stopped = true;
    }

    /// Ends a trace and returns the trace data. Prepends a magic value & data length to the trace
    /// data.
    pub fn end_to_buffer(mut self) -> Vec<u8> {
        // Safe because the session is guaranteed to be valid by self, and trace_data is disposed
        // by later calling ctrace_free_trace_buffer.
        let mut trace_data = unsafe { end_trace_to_buffer(self.session) };

        // Safe because:
        // 1. trace_data is valid from 0..size.
        // 2. trace_data lives as long as this slice.
        let trace_data_slice =
            unsafe { slice::from_raw_parts(trace_data.data as *mut u8, trace_data.size as usize) };

        let header = TraceHeader {
            magic: *HEADER_MAGIC,
            data_size: trace_data.size,
            data_checksum_sha256: hashes::sha256(trace_data_slice),
        };
        let mut trace_vec: Vec<u8> =
            Vec::with_capacity(size_of::<TraceHeader>() + trace_data.size as usize);
        trace_vec.extend_from_slice(header.as_bytes());
        trace_vec.extend_from_slice(trace_data_slice);

        // Safe because trace data is a valid buffer created by ctrace_stop_trace_to_buffer and
        // there are no other references to it.
        unsafe { ctrace_free_trace_buffer(&mut trace_data) };

        self.trace_stopped = true;
        trace_vec
    }
}

impl Drop for Trace {
    fn drop(&mut self) {
        if !self.trace_stopped {
            panic!("Trace must be stopped before it is dropped.")
        }
    }
}

/// Start a perfetto trace of duration `duration` and write the output to `output`.
pub fn run_trace(duration: Duration, buffer_size: u32, output: &Path) {
    let output = output.to_owned();
    std::thread::spawn(move || {
        let session = start_trace(duration, buffer_size);
        std::thread::sleep(duration);
        unsafe { end_trace(session, output.as_path()) };
    });
}

/// Starts a Perfetto trace with the provided config.
pub fn start_trace_from_proto(config: TraceConfig) -> anyhow::Result<ctrace_trace_session_handle> {
    let mut config_bytes = config.write_to_bytes()?;

    // Safe because config_bytes points to valid memory & we pass its size as required.
    Ok(unsafe {
        ctrace_trace_start_from_config_proto(
            config_bytes.as_mut_ptr() as *mut c_void,
            config_bytes.len() as u64,
        )
    })
}

/// Starts a trace with the given "duration", where duration specifies how much history to hold in
/// the ring buffer; in other words, duration is the lookback period when the trace results are
/// dumped.
pub fn start_trace(duration: Duration, buffer_size_kb: u32) -> ctrace_trace_session_handle {
    unsafe {
        ctrace_trace_start(&ctrace_trace_config {
            duration_ms: duration.as_millis() as u32,
            buffer_size_kb,
        })
    }
}

/// End the given trace session and write the results to `output`.
/// Safety: trace_session must be a valid trace session from `start_trace`.
pub unsafe fn end_trace(trace_session: ctrace_trace_session_handle, output: &Path) {
    let path_c_str = CString::new(output.as_os_str().to_str().unwrap()).unwrap();
    ctrace_trace_stop(trace_session, path_c_str.as_ptr());
}

/// End the given trace session returns the trace data.
///
/// Safety: trace_session must be a valid trace session from `start_trace`.
pub unsafe fn end_trace_to_buffer(
    trace_session: ctrace_trace_session_handle,
) -> ctrace_trace_buffer {
    ctrace_trace_stop_to_buffer(trace_session)
}

/// Add a clock snapshot to the current trace.
///
/// This function does not not do any inline checking if a trace is active,
/// and thus should only be called in a per-trace callback registered via
/// the add_per_trace_callback! macro.
pub fn snapshot_clock(mut snapshot: ClockSnapshot) {
    unsafe { ctrace_add_clock_snapshot(&mut snapshot.snapshot) };
}

/// Represents a Perfetto trace span.
pub struct TraceEvent {
    category_index: u64,
    instances: u32,
}

impl TraceEvent {
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    pub fn new(category_index: u64, instances: u32, name: *const std::ffi::c_char) -> Self {
        unsafe {
            trace_event_begin(
                category_index,
                instances,
                #[allow(clippy::not_unsafe_ptr_arg_deref)]
                name,
            )
        };
        Self {
            category_index,
            instances,
        }
    }
}

impl Drop for TraceEvent {
    fn drop(&mut self) {
        unsafe { trace_event_end(self.category_index, self.instances) }
    }
}

/// Extension of the Perfetto enum (protos::perfetto_config::BuiltinClock) which
/// includes the proposed (but not yet official) TSC clock item.
pub enum BuiltinClock {
    Unknown = 0,
    Realtime = 1,
    Coarse = 2,
    Monotonic = 3,
    MonotonicCoarse = 4,
    MonotonicRaw = 5,
    Boottime = 6,
    Tsc = 9,
}

/// Wrapper struct around a ctrace_clock_snapshot.
pub struct ClockSnapshot {
    pub snapshot: ctrace_clock_snapshot,
}

impl ClockSnapshot {
    pub fn new(first: &Clock, second: &Clock) -> ClockSnapshot {
        ClockSnapshot {
            snapshot: ctrace_clock_snapshot {
                clocks: [first.clock, second.clock],
            },
        }
    }
}

/// Builder wrapper for a ctrace_clock.
pub struct Clock {
    clock: ctrace_clock,
}

impl Clock {
    pub fn new(clock_id: u32, timestamp: u64) -> Clock {
        Clock {
            clock: ctrace_clock {
                clock_id,
                timestamp,
                is_incremental: false,
                unit_multiplier_ns: 0,
            },
        }
    }

    pub fn set_multiplier(&mut self, multiplier: u64) -> &mut Clock {
        self.clock.unit_multiplier_ns = multiplier;
        self
    }

    pub fn set_is_incremental(&mut self, is_incremental: bool) -> &mut Clock {
        self.clock.is_incremental = is_incremental;
        self
    }
}

// If running tests in debug mode, ie. `cargo test -p perfetto`,
// the cperfetto.dll needs to be imported into the `target` directory.
#[cfg(test)]
mod tests {
    #![allow(dead_code)]

    use std::ffi::c_char;

    use cros_tracing_types::static_strings::StaticString;

    use super::*;

    const AOT_BUFFER_SIZE_KB: u32 = 1024;
    const AOT_BUFFER_CLEAR_PERIOD: Duration = Duration::from_secs(1);
    setup_perfetto!(tests, future, "Async ftrace points", perfetto_tags!());
    #[test]
    fn test_async_trace_builds_and_runs() {
        tests::register_categories();
        init_tracing(BackendType::InProcess);
        let trace = Trace::start(
            TraceDuration::AlwaysOn,
            AOT_BUFFER_SIZE_KB,
            AOT_BUFFER_CLEAR_PERIOD,
            Some(vec!["future".to_string()]),
        )
        .expect("Failed to start trace");

        let static_name = StaticString::register("future_1");
        let future_category = PerfettoCategory::future;

        let flow_id = tests::trace_create_async!(future_category, static_name);
        assert!(flow_id.is_some());

        tests::trace_begin_async!(future_category, static_name, flow_id);

        let flow_id = tests::trace_pause_async!(future_category);
        assert!(flow_id.is_some());

        tests::trace_begin_async!(future_category, static_name, flow_id);

        tests::trace_end_async!(future_category);

        trace.end_to_buffer();
    }

    #[test]
    fn test_tags_macro_all_empty() {
        let all_tags_empty = perfetto_tags!();

        // SAFETY: strings from perfetto_tags have static lifetime.
        unsafe {
            assert_eq!(*(all_tags_empty[0] as *const char), '\0');
            assert_eq!(*(all_tags_empty[1] as *const char), '\0');
            assert_eq!(*(all_tags_empty[2] as *const char), '\0');
            assert_eq!(*(all_tags_empty[3] as *const char), '\0');
        }
    }

    #[test]
    fn test_tags_macro_two_used() {
        let two_used_tags = perfetto_tags!("tag0", "tag1");

        // SAFETY: strings from perfetto_tags have static lifetime.
        let tag0 = unsafe { CStr::from_ptr(two_used_tags[0] as *mut c_char) };
        // SAFETY: strings from perfetto_tags have static lifetime.
        let tag1 = unsafe { CStr::from_ptr(two_used_tags[1] as *mut c_char) };
        assert_eq!(tag0.to_str().unwrap(), "tag0");
        assert_eq!(tag1.to_str().unwrap(), "tag1");

        // SAFETY: strings have static lifetime.
        unsafe {
            assert_eq!(*(two_used_tags[2] as *const char), '\0');
            assert_eq!(*(two_used_tags[3] as *const char), '\0');
        }
    }
}
