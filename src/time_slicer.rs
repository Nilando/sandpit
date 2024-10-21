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
    max_headroom_ratio: f64,
    timeslice_size: f64,
    slice_min: f64,
}

impl TimeSlicer {
    pub fn new(
        tracer_controller: Arc<TracerController>,
        heap: Heap,
        max_headroom_ratio: f64,
        timeslice_size: f64,
        slice_min: f64,
    ) -> Self {
        Self {
            tracer_controller,
            heap,
            max_headroom_ratio,
            timeslice_size,
            slice_min,
        }
    }

    fn split_timeslice(&self, max_headroom: f64, starting_heap_size: f64) -> Option<(Duration, Duration)> {
        // Algorithm inspired from webkit riptide collector:
        let one_mili_in_nanos = 1_000_000.0;
        let current_heap_size = self.heap.get_size() as f64;
        let capped_heap_size = max_headroom + starting_heap_size;
        if current_heap_size >= capped_heap_size {
            return None;
        }
        let available_headroom = capped_heap_size - current_heap_size;
        let headroom_ratio = available_headroom / max_headroom;
        let m = (self.timeslice_size - self.slice_min) * headroom_ratio;
        let mutator_nanos = one_mili_in_nanos * m;
        let collector_nanos = (self.timeslice_size * one_mili_in_nanos) - mutator_nanos;
        let mutator_duration = Duration::from_nanos(mutator_nanos as u64);
        let collector_duration = Duration::from_nanos(collector_nanos as u64);

        debug_assert_eq!(
            collector_nanos + mutator_nanos,
            one_mili_in_nanos * self.timeslice_size
        );

        Some((mutator_duration, collector_duration))
    }

    pub fn run(&self) {
        let starting_heap_size = self.heap.get_size() as f64;
        let headroom = starting_heap_size * self.max_headroom_ratio;

        loop {
            match self.split_timeslice(headroom, starting_heap_size) {
                Some((mutator_duration, collector_duration)) => {
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
                None => {
                    self.tracer_controller.raise_yield_flag();
                    break;
                }
            }
        }
    }
}
