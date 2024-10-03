/// This structure contains the configuration settings for a garbage collector.
#[derive(Copy, Clone, Debug)]
pub struct GcConfig {
    /// The number of tracer threads, not including the thread that is used for
    /// monitoring.
    pub tracer_threads: usize,
    /// The amount of work a tracer does before attempting to share its work.
    pub trace_chunk_size: usize,
    /// The minimum a mount of work a tracer must obtain in order to share its work.
    pub trace_share_min: usize,
    /// The percent of work a tracer shares when sharing its work.
    pub trace_share_ratio: f32,
    /// The amount of miliseconds a tracer will wait for work before checking again or finishing its trace.
    pub trace_wait_time: u64,

    /// Once the amount of marked objects surpasses the max old object count
    /// a major collection will be triggered. The max old object count is calculated
    /// by multiplying this value by the amount of old objects marked in the
    /// previous major collection.
    pub monitor_max_old_growth_rate: f32,
    /// Once the size of the arena divided by the previous arena size after collection
    /// surpassed this value, a minor collection will be triggered.
    pub monitor_arena_size_ratio_trigger: f32,
    /// This setting this flag on or off will enable the monitor respectively.
    pub monitor_wait_time: u64,
    pub monitor_on: bool,

    /// The minimum amount of work a mutator must accumulate before sending to
    /// be traced.
    pub mutator_share_min: usize,

    /// The headroom is the maximum amount of memory that can be allocated,
    /// during a collection, before a yield is requested.
    /// The headroom is calculated by multiplying the previous arena size by
    /// the headroom ratio.
    pub collector_max_headroom_ratio: f32,

    /// The timeslicer essentially rate limits aggressive mutators, to ensure
    /// that the tracers will outpace them. The timeslice size will effect the
    /// length of time the timeslicer will request a mutator to yield in a single
    /// time frame. This represent number represents milliseconds.
    pub collector_timeslice_size: f32,

    /// The minimum amount of time the timeslicer will request yields for.
    /// Lowering this may help less aggressive mutators, but might make more
    /// aggressive mutators more likely to trigger yield earlier in the tracing
    /// process (meaning less concurrency).
    /// This represent number represents milliseconds.
    pub collector_slice_min: f32,
}

pub const GC_CONFIG_DEFAULT_TRACE_THREADS: usize = 2;
pub const GC_CONFIG_DEFAULT_TRACE_CHUNK_SIZE: usize = 100;
pub const GC_CONFIG_DEFAULT_TRACE_SHARE_MIN: usize = 50;
pub const GC_CONFIG_DEFAULT_TRACE_SHARE_RATIO: f32 = 0.5;
pub const GC_CONFIG_DEFAULT_TRACE_WAIT_TIME: u64 = 1;

// basically all the heuristic constant values used in this crate are collected here
// not necessarily a good idea to change any of these, but collecting them all here
// at least makes it clear what values are "arbitrary"
//
// The GcConfig can be updated after the Gc is created, but the update will only take place
// until tracing has completed.
impl GcConfig {
    /// Creates a default GcConfig. Good for most use cases.
    pub fn default() -> Self {
        GcConfig {
            tracer_threads: GC_CONFIG_DEFAULT_TRACE_THREADS,
            trace_chunk_size: GC_CONFIG_DEFAULT_TRACE_CHUNK_SIZE,
            trace_share_min: GC_CONFIG_DEFAULT_TRACE_SHARE_MIN,
            trace_share_ratio: GC_CONFIG_DEFAULT_TRACE_SHARE_RATIO,
            trace_wait_time: GC_CONFIG_DEFAULT_TRACE_WAIT_TIME,

            monitor_max_old_growth_rate: 10.0,
            monitor_arena_size_ratio_trigger: 2.0,
            monitor_wait_time: 10,
            monitor_on: true,

            mutator_share_min: 1000,

            collector_max_headroom_ratio: 2.0,
            collector_timeslice_size: 2.0,
            collector_slice_min: 0.6,
        }
    }
}
