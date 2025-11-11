use std::sync::atomic::{AtomicU64, AtomicU8, Ordering};

/// A 'snapshot' of the metrics relevant to the GC's internal triggers.
///
/// Obtained by calling [`crate::Arena::metrics`].
#[derive(Debug)]
pub struct Metrics {
    /// Number of major collections that have occured.
    pub major_collections: AtomicU64,

    /// Number of minor collections that have occured.
    pub minor_collections: AtomicU64,

    /// Average time that takes for a major collection to complete.
    pub major_collect_avg_time: AtomicU64,

    /// Average time that takes for a minor collection to complete.
    pub minor_collect_avg_time: AtomicU64,

    /// Once the old objects count reaches this number, a major collection will
    /// be triggered
    pub max_old_objects: AtomicU64,

    /// Running total of object that have been traced since the last major
    /// collection and all succeeding minor collections.
    pub old_objects_count: AtomicU64,

    /// Total amount of memory allocated by the arena.
    pub arena_size: AtomicU64,

    /// The arena size at the start of the last collection.
    pub prev_arena_size: AtomicU64,

    /// The current state of the GC.
    pub state: AtomicU8,

    pub max_yield_time: AtomicU64,
    pub avg_yield_time: AtomicU64,
    pub max_arena_size: AtomicU64,
    pub monitor_is_on: bool
}

impl Metrics {
    pub fn new() -> Self {
        Self {
            major_collections: AtomicU64::new(0),
            minor_collections: AtomicU64::new(0),
            major_collect_avg_time: AtomicU64::new(0),
            minor_collect_avg_time: AtomicU64::new(0),
            max_yield_time: AtomicU64::new(0),
            avg_yield_time: AtomicU64::new(0),
            max_old_objects: AtomicU64::new(0),
            old_objects_count: AtomicU64::new(0),
            arena_size: AtomicU64::new(0),
            max_arena_size: AtomicU64::new(0),
            prev_arena_size: AtomicU64::new(0),
            state: AtomicU8::new(GC_STATE_SLEEPING),
            monitor_is_on: true
        }
    }

    pub fn get_major_collections(&self) -> u64 {
        self.major_collections.load(Ordering::Relaxed)
    }

    pub fn get_minor_collections(&self) -> u64 {
        self.minor_collections.load(Ordering::Relaxed)
    }

    pub fn update_minor_collection_avg_time(&self, new_value: u64) {
        update_avg_u64(
            &self.minor_collect_avg_time,
            new_value, 
            self.minor_collections.load(Ordering::Relaxed)
        );
    }

    pub fn update_major_collection_avg_time(&self, new_value: u64) {
        update_avg_u64(
            &self.major_collect_avg_time,
            new_value,
            self.major_collections.load(Ordering::Relaxed)
        );
    }

    pub fn get_major_collect_avg_time(&self) -> u64 {
        self.major_collect_avg_time.load(Ordering::Relaxed)
    }

    pub fn get_minor_collect_avg_time(&self) -> u64 {
        self.minor_collect_avg_time.load(Ordering::Relaxed)
    }

    pub fn get_max_old_objects(&self) -> u64 {
        self.max_old_objects.load(Ordering::Relaxed)
    }

    pub fn get_old_objects_count(&self) -> u64 {
        self.old_objects_count.load(Ordering::Relaxed)
    }

    pub fn get_arena_size(&self) -> u64 {
        self.arena_size.load(Ordering::Relaxed)
    }

    pub fn get_prev_arena_size(&self) -> u64 {
        self.prev_arena_size.load(Ordering::Relaxed)
    }

    pub fn get_state(&self) -> u8 {
        self.state.load(Ordering::Relaxed)
    }

    pub fn get_max_yield_time(&self) -> u64 {
        self.max_yield_time.load(Ordering::Relaxed)
    }

    pub fn get_avg_yield_time(&self) -> u64 {
        self.avg_yield_time.load(Ordering::Relaxed)
    }

    pub fn get_max_arena_size(&self) -> u64 {
        self.max_arena_size.load(Ordering::Relaxed)
    }
}

pub fn update_avg_u64(running_avg: &AtomicU64, new_value: u64, sample_size: u64) {
    let avg = running_avg.load(Ordering::Relaxed);
    let update = new_value.abs_diff(avg) / sample_size;
    let new_avg = avg + update;

    running_avg.store(new_avg, Ordering::Relaxed);
}

pub const GC_STATE_SLEEPING: u8 = 0;
pub const GC_STATE_TRACING: u8 = 1;
pub const GC_STATE_SWEEPING: u8 = 2;
pub const GC_STATE_WAITING_ON_MUTATORS: u8 = 3;
