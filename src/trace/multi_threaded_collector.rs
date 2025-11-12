use super::trace::Trace;
use super::trace_job::TraceJob;
use super::tracer::Tracer;
use crate::config::Config;
use crate::debug::gc_debug;
use crate::header::GcMark;
use crate::heap::{Allocator, Heap};
use crate::metrics::{GC_STATE_SLEEPING, GC_STATE_SWEEPING, GC_STATE_TRACING, GC_STATE_WAITING_ON_MUTATORS};
use crate::Metrics;
use alloc::format;
use alloc::vec;
use alloc::vec::Vec;
use core::ptr::NonNull;
use core::sync::atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering};
use crossbeam_channel::{Receiver, Sender};
use std::sync::Mutex;
use std::time::{Instant, SystemTime};
use crate::pointee::Thin;

pub struct MultiThreadedCollector {
    sender: Sender<Vec<TraceJob>>,
    receiver: Receiver<Vec<TraceJob>>,
    heap: Heap,
    current_mark: AtomicU8,
    yield_flag: AtomicBool,
    collection_lock: Mutex<()>,
    active_mutators: AtomicUsize,
    shutdown_flag: AtomicBool,
    pub config: Config,
    pub metrics: Metrics,
}

impl MultiThreadedCollector {
    pub fn new(config: Config) -> Self {
        let (sender, receiver) = crossbeam_channel::unbounded();
        let heap = Heap::new();
        let metrics = Metrics::new();

        Self {
            heap,
            sender,
            receiver,
            yield_flag: AtomicBool::new(false),
            collection_lock: Mutex::new(()),
            active_mutators: AtomicUsize::new(0),
            current_mark: AtomicU8::new(GcMark::Red.into()),
            shutdown_flag: AtomicBool::new(false),
            metrics,
            config,
        }
    }

    pub fn shutdown(&self) {
        self.shutdown_flag.store(true, Ordering::SeqCst);
    }

    pub fn should_shutdown(&self) -> bool {
        self.shutdown_flag.load(Ordering::SeqCst)
    }

    /// Spawn a monitor thread for automatic garbage collection.
    /// Returns Some(JoinHandle) if monitor_on is true in config, None otherwise.
    ///
    /// # Safety
    /// The caller must ensure that root_ptr remains valid for the lifetime of the monitor thread.
    pub fn spawn_monitor_thread<T: Trace + 'static>(
        self: alloc::sync::Arc<Self>,
        root_ptr: *const T,
    ) -> Option<std::thread::JoinHandle<()>> {
        if !self.config.monitor_on {
            return None;
        }

        // Wrap pointer to make it Send (usize is Send)
        let root_ptr_addr = root_ptr as usize;

        let handle = std::thread::spawn(move || {
            // SAFETY: We're reconstructing the pointer that was valid when passed in
            let root_ptr = root_ptr_addr as *const T;
            monitor::spawn_monitor(self, root_ptr);
        });

        Some(handle)
    }

    fn timed_collection(&self, is_major: bool, f: impl FnOnce()) {
        let start_time = SystemTime::now();

        f();

        let collection_duration: u64 = start_time.elapsed().unwrap().as_millis() as u64;

        if is_major {
            self.metrics
                .update_minor_collection_avg_time(collection_duration);
        } else {
            self.metrics
                .update_major_collection_avg_time(collection_duration);
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
        // SAFETY: We just completed a trace, and we checked that all mutators
        // have dropped their yield locks, ensuring no mutation contexts exist
        // and we hold the collection lock, ensuring no mutation contexts can
        // be created at this point
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
        self.clean_up();
        gc_debug("Trace Complete!");
    }

    fn trace_root<T: Trace + ?Sized>(&self, root: &T) {
        let ptr: NonNull<Thin<T>> = NonNull::from(root).cast();
        let trace_job = TraceJob::new(ptr);
        self.sender.send(vec![trace_job]).unwrap();
    }

    fn spawn_tracers(&self) {
        std::thread::scope(|scope| {
            for _ in 0..self.config.tracer_threads {
                scope.spawn(|| {
                    gc_debug("Tracer Spawned");
                    self.run_tracer();
                });
            }
        });
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

    fn is_trace_completed(&self) -> bool {
        if self.receiver.is_empty() {
            if self.mutators_stopped() {
                return true;
            }

            self.metrics
                .state
                .store(GC_STATE_WAITING_ON_MUTATORS, Ordering::Relaxed);
            self.raise_yield_flag();
        }

        false
    }

    fn clean_up(&self) {
        self.yield_flag.store(false, Ordering::SeqCst);
    }

    fn raise_yield_flag(&self) {
        self.yield_flag.store(true, Ordering::SeqCst);
    }

    fn mutators_stopped(&self) -> bool {
        self.active_mutators.load(Ordering::SeqCst) == 0
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
        self.metrics
            .arena_size
            .store(arena_size, Ordering::Relaxed);
        arena_size
    }
}

impl MultiThreadedCollector {
    pub fn major_collect<T: Trace + ?Sized>(&self, root: &T) {
        let _guard = self.collection_lock.lock().unwrap();

        gc_debug("Starting Major Collection");

        self.metrics.old_objects_count.store(0, Ordering::Relaxed);
        self.rotate_mark();
        self.timed_collection(true, || self.trace_and_sweep(root));

        self.metrics
            .major_collections
            .fetch_add(1, Ordering::Relaxed);
        self.metrics
            .prev_arena_size
            .store(self.get_arena_size(), Ordering::Relaxed);
        let old_objects = self.metrics.get_old_objects_count();
        self.metrics.max_old_objects.store(
            (old_objects as f32 * self.config.monitor_max_old_growth_rate).floor() as u64,
            Ordering::Relaxed,
        );

        self.metrics
            .state
            .store(GC_STATE_SLEEPING, Ordering::Relaxed);
    }

    pub fn minor_collect<T: Trace + ?Sized>(&self, root: &T) {
        let _guard = self.collection_lock.lock().unwrap();

        gc_debug("Starting Minor Collection");

        self.timed_collection(false, || self.trace_and_sweep(root));

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
        let _lock = self.collection_lock.lock();
        Allocator::from(&self.heap)
    }

    pub fn send_work(&self, work: Vec<TraceJob>) {
        self.sender.send(work).unwrap();
    }

    pub fn recv_work(&self) -> Option<Vec<TraceJob>> {
        let duration = core::time::Duration::from_millis(self.config.trace_wait_time);
        let deadline = Instant::now().checked_add(duration).unwrap();

        loop {
            match self.receiver.recv_deadline(deadline) {
                Ok(work) => {
                    return Some(work);
                }
                Err(_) => {
                    if self.is_trace_completed() {
                        return None;
                    }
                }
            }
        }
    }

    pub fn has_work(&self) -> bool {
        !self.receiver.is_empty()
    }

    pub fn yield_flag(&self) -> bool {
        self.yield_flag.load(Ordering::SeqCst)
    }

    pub fn increment_mutators(&self) {
        self.active_mutators.fetch_add(1, Ordering::SeqCst);
    }

    pub fn decrement_mutators(&self) {
        self.active_mutators.fetch_sub(1, Ordering::SeqCst);
    }

    pub fn major_trigger(&self) -> bool {
        let current_old_objects_count = self.metrics.old_objects_count.load(Ordering::Relaxed);
        let max_old_objects_count = self.metrics.max_old_objects.load(Ordering::Relaxed);

        let result = current_old_objects_count > max_old_objects_count;
        if result {
            println!(
                "curr: {}, max {}",
                current_old_objects_count, max_old_objects_count
            )
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

// Monitor module for multi-threaded mode
pub mod monitor {
    use super::MultiThreadedCollector;
    use crate::trace::Trace;
    use alloc::sync::Arc;
    use core::ptr::NonNull;

    // SAFETY: This wrapper makes a raw pointer Send + Sync.
    // The caller must ensure that:
    // 1. The pointer remains valid for the lifetime of the monitor thread
    // 2. No mutable access occurs while the monitor thread is running
    // 3. The pointer is properly aligned and points to initialized memory
    struct SendSyncPtr<T: ?Sized>(NonNull<T>);

    unsafe impl<T: ?Sized> Send for SendSyncPtr<T> {}
    unsafe impl<T: ?Sized> Sync for SendSyncPtr<T> {}

    pub fn spawn_monitor<T: Trace + ?Sized>(
        collector: Arc<MultiThreadedCollector>,
        root_ptr: *const T,
    ) {
        // Wrap the raw pointer in a Send + Sync wrapper
        let root_ptr = SendSyncPtr(unsafe { NonNull::new_unchecked(root_ptr as *mut T) });

        loop {
            // Check shutdown flag first
            if collector.should_shutdown() {
                return;
            }

            monitor_sleep(&collector);

            // Check again after sleep
            if collector.should_shutdown() {
                return;
            }

            // SAFETY: The root pointer is guaranteed to be valid for the lifetime
            // of the Arena, which outlives the monitor thread
            let root_ref = unsafe { root_ptr.0.as_ref() };

            if collector.major_trigger() {
                collector.major_collect(root_ref);
            } else if collector.minor_trigger() {
                collector.minor_collect(root_ref);
            }
        }
    }

    fn monitor_sleep(collector: &MultiThreadedCollector) {
        let duration = std::time::Duration::from_millis(collector.config.monitor_wait_time);
        std::thread::sleep(duration);
    }
}
