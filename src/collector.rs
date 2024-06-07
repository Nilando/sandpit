use super::allocator::{Allocate, GenerationalArena};
use super::config::GcConfig;
use super::mutator::MutatorScope;
use super::trace::TraceLeaf;
use super::trace::{Marker, Trace, TraceMarker, TracerController};
use std::time::{Duration, Instant};

use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};

pub trait Collect {
    fn major_collect(&self);
    fn minor_collect(&self);

    fn get_old_objects_count(&self) -> usize;
    fn get_arena_size(&self) -> usize;
    fn get_major_collections(&self) -> usize;
    fn get_minor_collections(&self) -> usize;
    fn get_major_collect_avg_time(&self) -> usize;
    fn get_minor_collect_avg_time(&self) -> usize;
}

pub struct Collector<A: Allocate, T: Trace> {
    arena: A::Arena,
    tracer: Arc<TracerController<TraceMarker<A>>>,
    lock: Mutex<()>,
    root: T,
    major_collections: AtomicUsize,
    minor_collections: AtomicUsize,
    old_objects: AtomicUsize,
    // time stored in milisceonds
    minor_collect_avg_time: AtomicUsize,
    major_collect_avg_time: AtomicUsize,

    //config vars
    arena_size_ratio_trigger: f32,
    max_headroom_ratio: f32,
    timeslice_size: f32,
    slice_min: f32,

}

impl<A: Allocate, T: Trace> Collect for Collector<A, T> {
    fn major_collect(&self) {
        let _lock = self.lock.lock().unwrap();
        let start_time = Instant::now();
        self.old_objects.store(0, Ordering::SeqCst);
        self.major_collections.fetch_add(1, Ordering::SeqCst);
        self.collect(TraceMarker::new(self.arena.rotate_mark()).into());

        // update collection time
        let elapsed_time = start_time.elapsed().as_millis() as usize;
        self.update_collection_time(&self.major_collect_avg_time, elapsed_time, self.get_major_collections());
    }

    fn minor_collect(&self) {
        let _lock = self.lock.lock().unwrap();
        let start_time = Instant::now();
        self.minor_collections.fetch_add(1, Ordering::SeqCst);
        self.collect(TraceMarker::new(self.arena.current_mark()).into());

        // update collection time
        let elapsed_time = start_time.elapsed().as_millis() as usize;
        self.update_collection_time(&self.minor_collect_avg_time, elapsed_time, self.get_minor_collections());
    }

    fn get_major_collections(&self) -> usize {
        self.major_collections.load(Ordering::SeqCst)
    }

    fn get_minor_collections(&self) -> usize {
        self.minor_collections.load(Ordering::SeqCst)
    }

    fn get_arena_size(&self) -> usize {
        self.arena.get_size()
    }

    fn get_old_objects_count(&self) -> usize {
        self.old_objects.load(Ordering::SeqCst)
    }

    fn get_major_collect_avg_time(&self) -> usize {
        self.major_collect_avg_time.load(Ordering::SeqCst)
    }

    fn get_minor_collect_avg_time(&self) -> usize {
        self.minor_collect_avg_time.load(Ordering::SeqCst)
    }
}

impl<A: Allocate, T: Trace> Collector<A, T> {
    pub fn build(callback: fn(&mut MutatorScope<A>) -> T, config: &GcConfig) -> Self {
        let arena = A::Arena::new();
        let tracer = Arc::new(TracerController::new(config));
        let lock = tracer.yield_lock();
        let mut mutator = MutatorScope::new(&arena, &tracer, lock);
        let root = callback(&mut mutator);

        drop(mutator);

        Self {
            arena,
            tracer,
            root,
            lock: Mutex::new(()),
            major_collections: AtomicUsize::new(0),
            minor_collections: AtomicUsize::new(0),
            major_collect_avg_time: AtomicUsize::new(0),
            minor_collect_avg_time: AtomicUsize::new(0),
            old_objects: AtomicUsize::new(0),
            arena_size_ratio_trigger: config.monitor_arena_size_ratio_trigger,
            max_headroom_ratio: config.collector_max_headroom_ratio,
            timeslice_size: config.collector_timeslize,
            slice_min: config.collector_slice_min,
        }
    }

    pub fn insert<L: TraceLeaf>(&self, callback: fn(&T, L), value: L) {
        drop(self.lock.lock().unwrap());
        let _lock = self.tracer.yield_lock();

        callback(&self.root, value);
    }

    pub fn extract<L: TraceLeaf>(&self, callback: fn(&T) -> L) -> L {
        drop(self.lock.lock().unwrap());
        let _lock = self.tracer.yield_lock();

        callback(&self.root)
    }

    pub fn mutate(&self, callback: fn(&T, &mut MutatorScope<A>)) {
        let mut mutator = self.new_mutator();

        callback(&self.root, &mut mutator);
    }

    pub fn mutate_io<I: TraceLeaf, O: TraceLeaf>(
        &self,
        callback: fn(&T, &mut MutatorScope<A>, input: I) -> O,
        input: I,
    ) -> O {
        let mut mutator = self.new_mutator();

        callback(&self.root, &mut mutator, input)
    }

    fn new_mutator(&self) -> MutatorScope<A> {
        let _collection_lock = self.lock.lock().unwrap();
        let lock = self.tracer.yield_lock();

        MutatorScope::new(&self.arena, self.tracer.as_ref(), lock)
    }

    fn update_collection_time(&self, average: &AtomicUsize, elapsed_time: usize, num_collections: usize) { 
        let avg = average.load(Ordering::SeqCst);
        let update = elapsed_time.abs_diff(avg)/ num_collections;

        if avg > elapsed_time {
            average.fetch_sub(update, Ordering::SeqCst);
        } else {
            average.fetch_add(update, Ordering::SeqCst);
        }
    }

    fn split_timeslice(&self, max_headroom: usize, prev_size: usize) -> (Duration, Duration) {
        // Algorithm taken from webkit riptide collector:
        let one_mili_in_nanos = 1_000_000.0;
        let available_headroom = (max_headroom + prev_size) - self.arena.get_size();
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

    fn run_space_time_manager(&self) {
        let prev_size = self.arena.get_size();
        let max_headroom =
            ((prev_size as f32 / self.arena_size_ratio_trigger) * self.max_headroom_ratio) as usize;

        loop {
            // we've ran out of headroom, stop the mutators
            if self.arena.get_size() >= (max_headroom + prev_size) {
                self.tracer.raise_yield_flag();
                break;
            }

            let (mutator_duration, collector_duration) =
                self.split_timeslice(max_headroom, prev_size);

            std::thread::sleep(mutator_duration);

            if !self.tracer.is_tracing() {
                break;
            }

            let _lock = self.tracer.get_write_barrier_lock();
            std::thread::sleep(collector_duration);

            if !self.tracer.is_tracing() {
                break;
            }
        }
    }

    // TODO: differentiate sync and concurrent collections
    // sync collections need not track headroom
    fn collect(&self, marker: Arc<TraceMarker<A>>) {
        self.tracer.clone().trace(&self.root, marker.clone());
        // TODO: should space & time managing be done in a separate thread? otherwise a collection is guaranteed
        // to take 1.4ms
        self.run_space_time_manager();
        self.tracer.wait_for_trace_completion();
        self.old_objects
            .fetch_add(marker.get_mark_count(), Ordering::SeqCst);
        self.arena.refresh();
    }
}
