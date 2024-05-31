/// This structure contains the configuration settings for a garbage collector.
#[derive(Copy, Clone, Debug)]
pub struct GcConfig {
    // The number of tracer threads, not including the thread that is used for 
    // monitoring.
    pub tracer_threads: usize,
    // The amount of work a tracer does before attempting to share its work.
    pub trace_chunk_size: usize,
    // The minimum a mount of work a tracer must obtain in order to share its work.
    pub trace_share_min: usize,
    // The percent of work a tracer shares when sharing its work.
    pub trace_share_ratio: f32,
    // The amount of miliseconds a tracer will wait for work before checking again or finishing its trace.
    pub trace_wait_time: u64,

    // Once the amount of marked objects surpasses the max old object count
    // a major collection will be triggered. The max old object count is calculated
    // by multiplying this value by the amount of old objects marked in the
    // previous major collection.
    pub monitor_max_old_growth_rate: f32,
    // Once the size of the arena divided by the previous arena size after collection
    // surpassed this value, a minor collection will be triggered.
    pub monitor_arena_size_ratio_trigger: f32,
    // This setting this flag on or off will enable the monitor respectively.
    pub monitor_wait_time: u64,
    pub monitor_on: bool,

    // The minimum amount of work a mutator must accumulate before sending to
    // be traced.
    pub mutator_share_min: usize,

    pub collector_max_headroom_ratio: f32,
    pub collector_timeslize: f32,
    pub collector_slice_min: f32,
}

// The GcConfig can be updated after the Gc is created, but the update will only take place
// until tracing has completed.
impl GcConfig {
    pub fn default() -> Self {
        GcConfig {
            tracer_threads: 2,
            trace_chunk_size: 10_000,
            trace_share_min: 20_000,
            trace_share_ratio: 0.5,
            trace_wait_time: 1,
            monitor_max_old_growth_rate: 10.0,
            monitor_arena_size_ratio_trigger: 2.0,
            monitor_wait_time: 10,
            monitor_on: true,
            mutator_share_min: 10_000,
            collector_max_headroom_ratio: 2.0,
            collector_timeslize: 2.0,
            collector_slice_min: 0.6,
        }
    }
}
