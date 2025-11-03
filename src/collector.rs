use crate::Metrics;

use super::metrics::GcState;
use super::config::Config;
use super::debug::gc_debug;
use super::header::GcMark;
use super::mutator::Mutator;
use super::trace::{Trace, TracerController};

use higher_kinded_types::ForLt;

use core::sync::atomic::{AtomicUsize, AtomicU64, Ordering};
use std::time::Instant;
use std::sync::{Arc, Mutex};


// collect trait is nice so that monitor does not need
// to be generic on Root type, and just needs a type that impls collect
pub trait Collect {
    fn major_collect(&self);
    fn minor_collect(&self);

    fn get_old_objects_count(&self) -> u64;
    fn get_arena_size(&self) -> u64;
    fn get_major_collections(&self) -> usize;
    fn get_minor_collections(&self) -> usize;
    fn get_major_collect_avg_time(&self) -> usize;
    fn get_minor_collect_avg_time(&self) -> usize;
    fn get_state(&self) -> GcState;
}

pub struct Collector<R: ForLt>
where
    for<'a> <R as ForLt>::Of<'a>: Trace,
{
    tracer_controller: TracerController,

    // this lock is held while a collection is happening.
    // It is used to ensure that we don't start a collection while a collection
    // is happening. TODO: could we possibly start a collection while one is happening?
    collection_lock: Mutex<()>,

    root: R::Of<'static>,
    major_collections: AtomicUsize,
    minor_collections: AtomicUsize,
    old_objects: Arc<AtomicU64>,

    // time stored in milisceonds
    minor_collect_avg_time: AtomicUsize,
    major_collect_avg_time: AtomicUsize,
}

impl<R: ForLt> Collect for Collector<R>
where
    for<'a> <R as ForLt>::Of<'a>: Trace,
{
    fn major_collect(&self) {
        gc_debug("MAJOR COLLECTION TRIGGERED!");

        let _collection_lock = self.collection_lock.lock().unwrap();

        gc_debug("Rotating Trace Mark");

        let start_time = Instant::now();
        self.old_objects.store(0, Ordering::Relaxed);
        self.rotate_mark(); // major collection rotates the mark!
                            
        self.trace_and_sweep();

        self.major_collections.fetch_add(1, Ordering::Relaxed);

        // update collection time
        let elapsed_time = start_time.elapsed().as_millis();
        gc_debug(&format!("Collection completed in {}ms", elapsed_time));
        self.update_collection_time(
            &self.major_collect_avg_time,
            elapsed_time as usize,
            self.get_major_collections(),
        );
    }

    fn minor_collect(&self) {
        gc_debug("MINOR COLLECTION TRIGGERED!");

        let _collection_lock = self.collection_lock.lock().unwrap();
        let start_time = Instant::now();

        self.trace_and_sweep();

        self.minor_collections.fetch_add(1, Ordering::Relaxed);

        // update collection time
        let elapsed_time = start_time.elapsed().as_millis();
        gc_debug(&format!("Collection completed in {}ms", elapsed_time));
        self.update_collection_time(
            &self.minor_collect_avg_time,
            elapsed_time as usize,
            self.get_minor_collections(),
        );
    }

    fn get_major_collections(&self) -> usize {
        self.major_collections.load(Ordering::Relaxed)
    }

    fn get_minor_collections(&self) -> usize {
        self.minor_collections.load(Ordering::Relaxed)
    }

    fn get_arena_size(&self) -> u64 {
        self.tracer_controller.get_arena_size()
    }

    fn get_old_objects_count(&self) -> u64 {
        self.old_objects.load(Ordering::Relaxed)
    }

    fn get_major_collect_avg_time(&self) -> usize {
        self.major_collect_avg_time.load(Ordering::Relaxed)
    }

    fn get_minor_collect_avg_time(&self) -> usize {
        self.minor_collect_avg_time.load(Ordering::Relaxed)
    }

    fn get_state(&self) -> GcState {
        if self.collection_lock.try_lock().is_ok() {
            GcState::Waiting
        } else if self.tracer_controller.yield_flag() {
            GcState::Finishing
        } else {
            GcState::Marking
        }
    }
}

impl<R: ForLt> Collector<R>
where
    for<'a> <R as ForLt>::Of<'a>: Trace,
{
    pub fn new<F>(f: F, config: Config) -> Self
    where
        F: for<'gc> FnOnce(&'gc Mutator<'gc>) -> R::Of<'gc>,
    {

        // This is function is extremely sketchy.
        //
        // I have a very fragile understanding of whats going on here, and
        // I truly don't know whether this is safe.
        let tracer_controller = TracerController::new(config);
        let tracer_ref: &'static TracerController =
            unsafe { &*(&tracer_controller as *const TracerController) };
        let lock = tracer_ref.yield_lock();
        let mutator = Mutator::new(tracer_ref, lock);
        let mutator_ref: &'static Mutator<'static> =
            unsafe { &*(&mutator as *const Mutator<'static>) };

        let root: R::Of<'static> = f(mutator_ref);

        drop(mutator);

        Self {
            tracer_controller,
            root,
            collection_lock: Mutex::new(()),
            major_collections: AtomicUsize::new(0),
            minor_collections: AtomicUsize::new(0),
            major_collect_avg_time: AtomicUsize::new(0),
            minor_collect_avg_time: AtomicUsize::new(0),
            old_objects: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn mutate<F>(&self, f: F)
    where
        F: for<'gc> FnOnce(&'gc Mutator<'gc>, &'gc R::Of<'gc>),
    {
        let mutator = self.new_mutator();
        let root = unsafe { self.scoped_root() };

        f(&mutator, root);
    }

    pub fn view<F>(&self, f: F)
    where
        F: for<'gc> FnOnce(&'gc R::Of<'gc>),
    {
        let root = unsafe { self.scoped_root() };

        f(root);
    }

    unsafe fn scoped_root<'gc>(&self) -> &'gc R::Of<'gc> {
        core::mem::transmute::<&R::Of<'static>, &R::Of<'gc>>(&self.root)
    }

    fn new_mutator(&self) -> Mutator {
        let _collection_lock = self.collection_lock.lock();
        let yield_lock = self.tracer_controller.yield_lock();

        Mutator::new(&self.tracer_controller, yield_lock)
    }

    fn update_collection_time(
        &self,
        average: &AtomicUsize,
        elapsed_time: usize,
        num_collections: usize,
    ) {
        let avg = average.load(Ordering::Relaxed);
        let update = elapsed_time.abs_diff(avg) / num_collections;

        if avg > elapsed_time {
            average.fetch_sub(update, Ordering::Relaxed);
        } else {
            average.fetch_add(update, Ordering::Relaxed);
        }
    }

    fn rotate_mark(&self) -> GcMark {
        self.tracer_controller.rotate_mark()
    }

    pub fn get_metrics(&self) -> Metrics {
        self.tracer_controller.get_metrics()
    }

    fn trace_and_sweep(&self) {
        self.tracer_controller.trace_and_sweep(&self.root, self.old_objects.clone());
    }
}
