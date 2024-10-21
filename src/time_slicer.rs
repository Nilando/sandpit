use super::heap::Heap;
use super::trace::TracerController;
use std::sync::Arc;
use std::time::Duration;
// The time slicer's job is to slow down aggresively allocating mutators so
// that they are not able to outpace the tracers while simulatenously trying
// not to get too much in the way of less aggressive mutators.
//
// It does this by using a concept the "headroom" which is the amount of memory
// that is deemed acceptable to be allocated during the course of a single collection.
//
// The more a mutator usees up its head room, the more the time slicer will
// make requests for the mutators to yield.
//
// The timeslicer relies on the assumption that the mutators regularly call `gc_yield`
pub struct TimeSlicer {
    tracer_controller: Arc<TracerController>,
    heap: Heap,
    arena_size_ratio_trigger: f32,
    max_headroom_ratio: f32,
    timeslice_size: f32,
    slice_min: f32,
}

impl TimeSlicer {
    pub fn new(
        tracer_controller: Arc<TracerController>,
        heap: Heap,
        arena_size_ratio_trigger: f32,
        max_headroom_ratio: f32,
        timeslice_size: f32,
        slice_min: f32,
    ) -> Self {
        Self {
            tracer_controller,
            heap,
            arena_size_ratio_trigger,
            max_headroom_ratio,
            timeslice_size,
            slice_min,
        }
    }

    fn split_timeslice(&self, max_headroom: usize, prev_size: usize) -> (Duration, Duration) {
        // Algorithm inspired from webkit riptide collector:
        let one_mili_in_nanos = 1_000_000.0;
        let available_headroom = (max_headroom + prev_size) - self.heap.get_size();
        let headroom_ratio = available_headroom as f32 / max_headroom as f32;
        let m = (self.timeslice_size - self.slice_min) * headroom_ratio;
        let mutator_nanos = (one_mili_in_nanos * m) as u64;
        let collector_nanos = (self.timeslice_size * one_mili_in_nanos) as u64 - mutator_nanos;
        let mutator_duration = Duration::from_nanos(mutator_nanos);
        let collector_duration = Duration::from_nanos(collector_nanos);

        debug_assert_eq!(
            collector_nanos + mutator_nanos,
            (one_mili_in_nanos * self.timeslice_size) as u64
        );

        (mutator_duration, collector_duration)
    }

    pub fn run(&self) {
        let prev_size = self.heap.get_size();
        let max_headroom =
            ((prev_size as f32 / self.arena_size_ratio_trigger) * self.max_headroom_ratio) as usize;

        loop {
            // we've ran out of headroom, stop the mutators
            if self.heap.get_size() >= (max_headroom + prev_size) {
                self.tracer_controller.raise_yield_flag();
                break;
            }

            let (mutator_duration, collector_duration) =
                self.split_timeslice(max_headroom, prev_size);

            std::thread::sleep(mutator_duration);

            if self.tracer_controller.yield_flag() {
                break;
            }

            let _lock = self.tracer_controller.get_time_slice_lock();
            std::thread::sleep(collector_duration);

            if self.tracer_controller.yield_flag() {
                break;
            }
        }
    }
}
