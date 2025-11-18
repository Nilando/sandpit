use super::trace::Trace;
use super::trace_job::TraceJob;
use super::tracer::Tracer;
use crate::config::Config;
use crate::debug::gc_debug;
use crate::header::GcMark;
use crate::heap::{Allocator, Heap};
use crate::metrics::{GC_STATE_SLEEPING, GC_STATE_SWEEPING, GC_STATE_TRACING};
use crate::pointee::Thin;
use crate::Metrics;
use alloc::format;
use alloc::vec::Vec;
use core::cell::RefCell;
use core::ptr::NonNull;
use core::sync::atomic::{AtomicU8, Ordering};

pub struct SingleThreadedCollector {
    work_queue: RefCell<Vec<TraceJob>>,
    heap: Heap,
    current_mark: AtomicU8,
    pub config: Config,
    pub metrics: Metrics,
}

impl SingleThreadedCollector {
    pub fn new(config: Config) -> Self {
        let heap = Heap::new();
        let metrics = Metrics::new();

        Self {
            heap,
            work_queue: RefCell::new(Vec::new()),
            current_mark: AtomicU8::new(GcMark::Red.into()),
            metrics,
            config,
        }
    }

    fn trace_and_sweep<T: Trace + ?Sized>(&self, root: &T) {
        self.trace(root);

        self.metrics
            .state
            .store(GC_STATE_SWEEPING, Ordering::Relaxed);
        let current_arena_size = self.heap.get_size();
        let max_arena_size = self.metrics.get_max_arena_size();
        if max_arena_size < current_arena_size {
            self.metrics
                .max_arena_size
                .store(current_arena_size, Ordering::Relaxed);
        }
        // SAFETY: We just completed a trace in single-threaded mode
        unsafe {
            self.sweep();
        }

        self.print_debug_info();
    }

    fn trace<T: Trace + ?Sized>(&self, root: &T) {
        gc_debug("Beginning trace...");
        self.metrics
            .state
            .store(GC_STATE_TRACING, Ordering::Relaxed);
        self.trace_root(root);
        self.spawn_tracers();
        gc_debug("Trace Complete!");
    }

    fn trace_root<T: Trace + ?Sized>(&self, root: &T) {
        let ptr: NonNull<Thin<T>> = NonNull::from(root).cast();
        let trace_job = TraceJob::new(ptr);
        self.work_queue.borrow_mut().push(trace_job);
    }

    fn spawn_tracers(&self) {
        self.run_tracer();
    }

    fn run_tracer(&self) {
        let mut tracer = self.new_tracer();
        let marked_objects = tracer.trace_loop() as u64;
        self.metrics
            .old_objects_count
            .fetch_add(marked_objects, Ordering::SeqCst);
    }

    fn new_tracer(&self) -> Tracer<'_> {
        let mark = self.get_current_mark();
        Tracer::new(self, mark)
    }

    unsafe fn sweep(&self) {
        self.heap.sweep(self.get_current_mark());
    }

    fn print_debug_info(&self) {
        let arena_size = self.get_arena_size();
        let current_old_objects_count = self.metrics.old_objects_count.load(Ordering::Relaxed);
        let max_old_objects_count = self.metrics.max_old_objects.load(Ordering::Relaxed);
        let prev_arena_size = self.metrics.prev_arena_size.load(Ordering::Relaxed);

        gc_debug(&format!(
            "max_old: {}, current_old: {}, prev_size: {} kb, size: {} kb",
            max_old_objects_count,
            current_old_objects_count,
            (prev_arena_size / 1024),
            (arena_size / 1024)
        ));
    }

    fn get_arena_size(&self) -> u64 {
        let arena_size = self.heap.get_size();
        self.metrics.arena_size.store(arena_size, Ordering::Relaxed);
        arena_size
    }
}

impl SingleThreadedCollector {
    pub fn major_collect<T: Trace + ?Sized>(&self, root: &T) {
        gc_debug("Starting Major Collection");

        self.metrics.old_objects_count.store(0, Ordering::Relaxed);
        self.rotate_mark();
        self.trace_and_sweep(root);

        self.metrics
            .major_collections
            .fetch_add(1, Ordering::Relaxed);
        self.metrics
            .prev_arena_size
            .store(self.get_arena_size(), Ordering::Relaxed);
        let old_objects = self.metrics.get_old_objects_count();
        self.metrics.max_old_objects.store(
            (old_objects as f32 * self.config.monitor_max_old_growth_rate) as u64,
            Ordering::Relaxed,
        );

        self.metrics
            .state
            .store(GC_STATE_SLEEPING, Ordering::Relaxed);
    }

    pub fn minor_collect<T: Trace + ?Sized>(&self, root: &T) {
        gc_debug("Starting Minor Collection");

        self.trace_and_sweep(root);

        self.metrics
            .minor_collections
            .fetch_add(1, Ordering::Relaxed);
        self.metrics
            .prev_arena_size
            .store(self.get_arena_size(), Ordering::Relaxed);

        self.metrics
            .state
            .store(GC_STATE_SLEEPING, Ordering::Relaxed);
    }

    pub fn get_current_mark(&self) -> GcMark {
        self.current_mark.load(Ordering::SeqCst).into()
    }

    pub fn prev_mark(&self) -> GcMark {
        self.get_current_mark().prev()
    }

    pub fn rotate_mark(&self) -> GcMark {
        let new_mark = self.get_current_mark().rotate();
        self.current_mark.store(new_mark.into(), Ordering::SeqCst);
        new_mark
    }

    pub fn new_allocator(&self) -> Allocator {
        Allocator::from(&self.heap)
    }

    pub fn send_work(&self, mut work: Vec<TraceJob>) {
        self.work_queue.borrow_mut().append(&mut work);
    }

    pub fn recv_work(&self) -> Option<Vec<TraceJob>> {
        let mut work_queue = self.work_queue.borrow_mut();
        if work_queue.is_empty() {
            None
        } else {
            Some(work_queue.drain(..).collect())
        }
    }

    pub fn has_work(&self) -> bool {
        !self.work_queue.borrow().is_empty()
    }

    pub fn yield_flag(&self) -> bool {
        false
    }

    pub fn increment_mutators(&self) {
        // No-op in single-threaded mode
    }

    pub fn decrement_mutators(&self) {
        // No-op in single-threaded mode
    }

    pub fn major_trigger(&self) -> bool {
        let current_old_objects_count = self.metrics.old_objects_count.load(Ordering::Relaxed);
        let max_old_objects_count = self.metrics.max_old_objects.load(Ordering::Relaxed);

        let result = current_old_objects_count > max_old_objects_count;
        if result {
            self.print_debug_info();
        };
        result
    }

    pub fn minor_trigger(&self) -> bool {
        let arena_size = self.get_arena_size();
        let prev_arena_size = self.metrics.prev_arena_size.load(Ordering::Relaxed);
        let arena_size_ratio_trigger = self.config.monitor_arena_size_ratio_trigger;

        arena_size as f32 > (prev_arena_size as f32 * arena_size_ratio_trigger)
    }

    pub fn get_trace_share_ratio(&self) -> f32 {
        self.config.trace_share_ratio
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn metrics(&self) -> &Metrics {
        &self.metrics
    }
}
