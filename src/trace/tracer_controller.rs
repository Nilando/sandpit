use super::trace::Trace;
use super::trace_job::TraceJob;
use super::tracer::Tracer;
use crate::config::Config;
use crate::debug::gc_debug;
use crate::header::GcMark;
use crate::heap::{Allocator, Heap};
use crate::Metrics;
use crate::pointee::Thin;
use alloc::vec;
use crossbeam_channel::{Receiver, Sender};
use core::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use core::ptr::NonNull;
use alloc::vec::Vec;
use alloc::format;

#[cfg(feature = "multi_threaded")]
use std::sync::{Mutex, RwLock, RwLockReadGuard};
#[cfg(feature = "multi_threaded")]
use std::time::{Instant, SystemTime};

// Lock abstraction for single-threaded vs multi-threaded mode
#[cfg(feature = "multi_threaded")]
struct ControllerLocks {
    collection_lock: Mutex<()>,
    yield_lock: RwLock<()>,
}

#[cfg(not(feature = "multi_threaded"))]
struct ControllerLocks;

#[cfg(feature = "multi_threaded")]
pub type YieldLockGuard<'a> = RwLockReadGuard<'a, ()>;

#[cfg(not(feature = "multi_threaded"))]
pub type YieldLockGuard<'a> = ();

impl ControllerLocks {
    fn new() -> Self {
        #[cfg(feature = "multi_threaded")]
        return ControllerLocks {
            yield_lock: RwLock::new(()),
            collection_lock: Mutex::new(()),
        };

        #[cfg(not(feature = "multi_threaded"))]
        return ControllerLocks;
    }

    #[cfg(feature = "multi_threaded")]
    fn lock_collection(&self) -> std::sync::MutexGuard<()> {
        self.collection_lock.lock().unwrap()
    }

    #[cfg(not(feature = "multi_threaded"))]
    fn lock_collection(&self) -> () {
        ()
    }

    #[cfg(feature = "multi_threaded")]
    fn read_yield(&self) -> RwLockReadGuard<()> {
        self.yield_lock.read().unwrap()
    }

    #[cfg(not(feature = "multi_threaded"))]
    fn read_yield(&self) -> () {
        ()
    }

    #[cfg(feature = "multi_threaded")]
    fn try_write_yield(&self) -> Result<std::sync::RwLockWriteGuard<()>, std::sync::TryLockError<std::sync::RwLockWriteGuard<()>>> {
        self.yield_lock.try_write()
    }

    #[cfg(not(feature = "multi_threaded"))]
    fn try_write_yield(&self) -> Result<(), ()> {
        Ok(())
    }
}

pub struct TracerController {
    sender: Sender<Vec<TraceJob>>,
    receiver: Receiver<Vec<TraceJob>>,
    heap: Heap,
    current_mark: AtomicU8,
    yield_flag: AtomicBool,
    locks: ControllerLocks,
    root_jobs: Option<TraceJob>,
    pub config: Config,
    pub metrics: Metrics,
}

impl TracerController {
    pub fn new(config: Config) -> Self {
        let (sender, receiver) = crossbeam_channel::unbounded();
        let heap = Heap::new();
        let metrics = Metrics::new();

        Self {
            heap,
            sender,
            receiver,

            yield_flag: AtomicBool::new(false),
            locks: ControllerLocks::new(),
            current_mark: AtomicU8::new(GcMark::Red.into()),
            root_jobs: None,

            metrics,
            config
        }
    }

    pub fn get_metrics(&self) -> &Metrics {
        &self.metrics
    }

    pub fn set_root_job<T: Trace + ?Sized>(&mut self, root: &T) {
        let ptr: NonNull<Thin<T>> = NonNull::from(root).cast();
        let trace_job = TraceJob::new(ptr);

        self.root_jobs = Some(trace_job);
    }

    pub fn new_allocator(&self) -> Allocator {
        Allocator::from(&self.heap)
    }

    pub fn major_collect(&self) {
        let _guard = self.locks.lock_collection();

        gc_debug("Starting Major Collection");

        self.metrics.old_objects_count.store(0, Ordering::Relaxed);

        self.rotate_mark();

        self.timed_collection(true, || self.trace_and_sweep());

        self.metrics.major_collections.fetch_add(1, Ordering::Relaxed);
        self.metrics.prev_arena_size.store(self.get_arena_size(), Ordering::Relaxed);
    }

    pub fn minor_collect(&self) {
        let _guard = self.locks.lock_collection();

        gc_debug("Starting Minor Collection");

        self.timed_collection(false, || self.trace_and_sweep());

        self.metrics.minor_collections.fetch_add(1, Ordering::Relaxed);
        self.metrics.prev_arena_size.store(self.get_arena_size(), Ordering::Relaxed);
    }

    #[cfg(feature = "multi_threaded")]
    pub fn timed_collection(&self, is_major: bool, f: impl FnOnce() -> ()) {
        let start_time = SystemTime::now();

        f();

        let collection_duration: u64 = start_time.elapsed().unwrap().as_millis() as u64;

        if is_major {
            self.metrics.update_minor_collection_avg_time(collection_duration);
        } else {
            self.metrics.update_major_collection_avg_time(collection_duration);
        }
    }

    #[cfg(not(feature = "multi_threaded"))]
    pub fn timed_collection(&self, _is_major: bool, f: impl FnOnce() -> ()) {
        f();
    }

    pub fn trace_and_sweep(&self) {
        self.trace();

        // SAFETY: We just completed a trace, and we checked that all mutators
        // have dropped their yield locks, ensuring no mutation contexts exist
        // and we hold the collectino lock, ensuring no mutation contexts can 
        // be created at this point
        unsafe { self.sweep(); }

        self.print_debug_info();
    }

    fn trace(&self) {
        gc_debug("Begining trace...");
        self.trace_root();
        self.spawn_tracers();
        self.clean_up();
        gc_debug("Trace Complete!");
    }

    fn run_tracer(&self) {
        let mut tracer = self.new_tracer();
        let marked_objects = tracer.trace_loop() as u64;
        self.metrics.old_objects_count.fetch_add(marked_objects, Ordering::SeqCst);
    }

    #[cfg(feature = "multi_threaded")]
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

    #[cfg(not(feature = "multi_threaded"))]
    fn spawn_tracers(&self) {
        self.run_tracer();
    }

    fn trace_root(&self) {
        // Send the stored root jobs to kick off the trace
        if let Some(jobs) = &self.root_jobs {
            self.sender.send(vec![jobs.clone()]).unwrap();
        }
    }

    fn new_tracer(&self) -> Tracer {
        let mark = self.get_current_mark();

        Tracer::new(self, mark)
    }

    pub fn send_work(&self, work: Vec<TraceJob>) {
        self.sender.send(work).unwrap();
    }

    #[cfg(feature = "multi_threaded")]
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

    #[cfg(not(feature = "multi_threaded"))]
    pub fn recv_work(&self) -> Option<Vec<TraceJob>> {
        self.receiver.try_recv().ok()
    }

    pub fn is_trace_completed(&self) -> bool {
        if self.receiver.is_empty() {
            if self.mutators_stopped() {
                return true;
            }

            self.raise_yield_flag();
        }

        false
    }

    pub fn rotate_mark(&self) -> GcMark {
        let new_mark = self.get_current_mark().rotate();

        self.current_mark.store(new_mark.into(), Ordering::SeqCst);

        new_mark
    }

    pub fn get_current_mark(&self) -> GcMark {
        self.current_mark.load(Ordering::SeqCst).into()
    }

    pub fn prev_mark(&self) -> GcMark {
        self.get_current_mark().prev()
    }

    fn clean_up(&self) {
        self.yield_flag.store(false, Ordering::SeqCst);
    }

    pub fn yield_flag(&self) -> bool {
        self.yield_flag.load(Ordering::SeqCst)
    }

    pub fn raise_yield_flag(&self) {
        self.yield_flag.store(true, Ordering::SeqCst);
    }

    pub fn yield_lock(&self) -> YieldLockGuard {
        let _guard = self.locks.lock_collection();
        self.locks.read_yield()
    }

    pub fn get_trace_share_ratio(&self) -> f32 {
        self.config.trace_share_ratio
    }

    pub fn has_work(&self) -> bool {
        !self.receiver.is_empty()
    }

    fn mutators_stopped(&self) -> bool {
        self.locks.try_write_yield().is_ok()
    }

    // SAFETY: at this point there are no mutators and all garbage collected
    // values have been marked with the current_mark
    unsafe fn sweep(&self) {
        self.heap.sweep(self.get_current_mark());
    }

    fn print_debug_info(&self) {
        let arena_size = self.get_arena_size();
        let current_old_objects_count = self.metrics.old_objects_count.load(Ordering::Relaxed);
        let max_old_objects_count = self.metrics.max_old_objects.load(Ordering::Relaxed);
        let prev_arena_size = self.metrics.prev_arena_size.load(Ordering::Relaxed);

        gc_debug(
            &format!(
                "max_old: {}, current_old: {}, prev_size: {} kb, size: {} kb", 
                max_old_objects_count, 
                current_old_objects_count, 
                (prev_arena_size/1024), 
                (arena_size/1024)
            )
        );
    }

    pub fn major_trigger(&self) -> bool {
        let current_old_objects_count = self.metrics.old_objects_count.load(Ordering::Relaxed);
        let max_old_objects_count = self.metrics.max_old_objects.load(Ordering::Relaxed);

        current_old_objects_count > max_old_objects_count
    }

    pub fn minor_trigger(&self) -> bool {
        let arena_size = self.get_arena_size();
        let prev_arena_size = self.metrics.prev_arena_size.load(Ordering::Relaxed);
        let arena_size_ratio_trigger = self.config.monitor_arena_size_ratio_trigger;

        arena_size as f32 > (prev_arena_size as f32 * arena_size_ratio_trigger)
    }

    fn get_arena_size(&self) -> u64 {
        let arena_size = self.heap.get_size();
        self.metrics.arena_size.store(arena_size, Ordering::Relaxed);
        arena_size
    }
}

#[cfg(feature = "multi_threaded")]
pub mod monitor {
    use super::TracerController;
    use alloc::sync::Arc;

    pub fn spawn_monitor(mut tc: Arc<TracerController>) {
        // TODO: if monitor is on? do nothing

        loop {
            monitor_sleep(&tc);

            if tc.major_trigger() {
                tc.major_collect();
            } else if tc.minor_trigger() {
                tc.minor_trigger();
            }

            match Arc::<TracerController>::try_unwrap(tc) {
                Ok(_) => return,
                Err(err) => tc = err,
            }
        }
    }

    fn monitor_sleep(tc: &TracerController) {
        let duration = std::time::Duration::from_millis(tc.config.monitor_wait_time);

        std::thread::sleep(duration);
    }
}
