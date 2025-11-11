use super::trace::Trace;
use super::trace_job::TraceJob;
use super::tracer::Tracer;
use crate::config::Config;
use crate::debug::gc_debug;
use crate::header::GcMark;
use crate::heap::{Allocator, Heap};
use crate::Metrics;
use crossbeam_channel::{Receiver, Sender};
use core::sync::atomic::{AtomicBool, AtomicU8, Ordering};

use std::sync::{Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::time::{Instant, SystemTime};

pub struct TracerController {
    sender: Sender<Vec<TraceJob>>,
    receiver: Receiver<Vec<TraceJob>>,
    heap: Heap,
    current_mark: AtomicU8,
    yield_flag: AtomicBool,
    yield_lock: RwLock<()>,
    collection_lock: Mutex<()>,
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
            yield_lock: RwLock::new(()),
            collection_lock: Mutex::new(()),
            current_mark: AtomicU8::new(GcMark::Red.into()),

            metrics,
            config
        }
    }

    pub fn get_metrics(&self) -> &Metrics {
        &self.metrics
    }

    pub fn new_allocator(&self) -> Allocator {
        let _guard = self.collection_lock.lock().unwrap();
        Allocator::from(&self.heap)
    }

    pub fn major_collect<T: Trace>(
        &self,
        root: &T,
    ) {
        let _guard = self.collection_lock.lock().unwrap();

        gc_debug("Starting Major Collection");

        self.metrics.old_objects_count.store(0, Ordering::Relaxed);

        self.rotate_mark();

        self.timed_collection(true, || self.trace_and_sweep(root));

        self.metrics.major_collections.fetch_add(1, Ordering::Relaxed);
    }

    pub fn minor_collect<T: Trace>(
        &self,
        root: &T,
    ) {
        let _guard = self.collection_lock.lock().unwrap();

        gc_debug("Starting Minor Collection");

        self.timed_collection(false, || self.trace_and_sweep(root));

        self.metrics.minor_collections.fetch_add(1, Ordering::Relaxed);
    }

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


    pub fn trace_and_sweep<T: Trace>(
        &self,
        root: &T,
    ) {
        self.trace(root);
        unsafe { self.sweep(); }
        self.print_debug_info();
    }

    fn trace<T: Trace>(
        &self,
        root: &T,
    ) {
        gc_debug("Begining trace...");

        self.trace_root(root);
        self.spawn_tracers();

        gc_debug("Trace Complete!");

        self.clean_up();
    }

    fn spawn_tracers(&self) {
        std::thread::scope(|scope| {
            for _ in 0..self.config.tracer_threads {
                scope.spawn(|| {
                    let mut tracer = self.new_tracer();

                    gc_debug("Tracer Thread Spawned");

                    let marked_objects = tracer.trace_loop() as u64;

                    self.metrics.old_objects_count.fetch_add(marked_objects, Ordering::SeqCst);
                });
            }
        });
    }

    fn trace_root<T: Trace>(&self, root: &T) {
        let mut tracer = self.new_tracer();
        root.trace(&mut tracer);
        tracer.flush_work();
        let mark_count = tracer.mark_count as u64;
        self.metrics.old_objects_count.fetch_add(mark_count, Ordering::SeqCst);
    }

    fn new_tracer(&self) -> Tracer {
        let mark = self.get_current_mark();

        Tracer::new(self, mark)
    }

    pub fn send_work(&self, work: Vec<TraceJob>) {
        self.sender.send(work).unwrap();
    }

    pub fn recv_work(&self) -> Option<Vec<TraceJob>> {
        let duration = std::time::Duration::from_millis(self.config.trace_wait_time);
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

    pub fn yield_lock(&self) -> RwLockReadGuard<()> {
        self.yield_lock.read().unwrap()
    }

    pub fn get_trace_share_ratio(&self) -> f32 {
        self.config.trace_share_ratio
    }

    pub fn has_work(&self) -> bool {
        !self.receiver.is_empty()
    }

    fn mutators_stopped(&self) -> bool {
        // if the yield lock can write
        //  write the stopped value
        //  and return true
        // else if we can read
        //  if the value is the stopped value
        //      return true
        //  else
        //      return false
        self.yield_lock.try_write().is_ok()
    }

    // SAFETY: at this point there are no mutators and all garbage collected
    // values have been marked with the current_mark
    unsafe fn sweep(&self) {
        self.heap.sweep(self.get_current_mark());
        gc_debug("Sweep Complete!");
    }

    fn print_debug_info(&self) {
        let current_old_objects_count = self.metrics.old_objects_count.load(Ordering::Relaxed);
        let max_old_objects_count = self.metrics.max_old_objects.load(Ordering::Relaxed);
        let arena_size = self.metrics.arena_size.load(Ordering::Relaxed);
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

    fn major_trigger(&self) -> bool {
        let current_old_objects_count = self.metrics.old_objects_count.load(Ordering::Relaxed);
        let max_old_objects_count = self.metrics.max_old_objects.load(Ordering::Relaxed);

        current_old_objects_count > max_old_objects_count
    }

    fn minor_trigger(&self) -> bool {
        let arena_size = self.metrics.arena_size.load(Ordering::Relaxed);
        let prev_arena_size = self.metrics.prev_arena_size.load(Ordering::Relaxed);
        let arena_size_ratio_trigger = self.config.monitor_arena_size_ratio_trigger;

        arena_size as f32 > (prev_arena_size as f32 * arena_size_ratio_trigger)
    }
}

pub mod monitor {
    use crate::Trace;

    use super::TracerController;
    use alloc::sync::Arc;
    use higher_kinded_types::ForLt;

    pub fn spawn_monitor<R: ForLt + 'static>(mut tc: Arc<TracerController>, root: R::Of<'static>)
    where
        for<'a> <R as ForLt>::Of<'a>: Trace
    {
        // TODO: if monitor is on? do nothing
       
        loop {
            monitor_sleep(&tc);

            if tc.minor_trigger() {
                tc.minor_collect(&root);

                if tc.major_trigger() {
                    tc.major_collect(&root);
                }
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
