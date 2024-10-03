use super::collector::GcState;
/// A 'snapshot' of the metrics relevant to the GC's internal triggers.
///
/// Obtained by calling [`crate::Arena::metrics`].
#[derive(Clone, Debug)]
pub struct Metrics {
    /// Number of major collections that have occured.
    pub major_collections: usize,

    /// Number of minor collections that have occured.
    pub minor_collections: usize,

    /// Average time that takes for a major collection to complete.
    pub major_collect_avg_time: usize,

    /// Average time that takes for a minor collection to complete.
    pub minor_collect_avg_time: usize,

    /// Once the old objects count reaches this number, a major collection will
    /// be triggered
    pub max_old_objects: usize,

    /// Running total of object that have been traced since the last major
    /// collection and all succeeding minor collections.
    pub old_objects_count: usize,

    /// Total amount of memory allocated by the arena.
    pub arena_size: usize,

    /// The arena size at the start of the last collection.
    pub prev_arena_size: usize,

    /// The current state of the GC.
    pub state: GcState,
}
