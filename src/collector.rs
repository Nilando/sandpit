use super::allocator::Allocator;
use super::config::GcConfig;
use super::header::GcMark;
use super::mutator::Mutator;
use super::time_slicer::TimeSlicer;
use super::trace::{Trace, TracerController};

use higher_kinded_types::ForLt;

use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};
use std::time::Instant;

#[repr(u8)]
#[derive(Clone, Debug)]
pub enum GcState {
    Waiting,
    Marking,
    Finishing,
}

// collect trait is nice so that monitor does not need
// to be generic on Root type, and just needs a type that impls collect
pub trait Collect {
    fn major_collect(&self);
    fn minor_collect(&self);

    fn get_old_objects_count(&self) -> usize;
    fn get_arena_size(&self) -> usize;
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
    arena: Allocator,
    tracer: Arc<TracerController>,

    // this lock is held while a collection is happening.
    // It is used to ensure that we don't start a collection while a collection
    // is happening. TODO: could we possibly start a collection while one is happening?
    collection_lock: Mutex<()>,
    mutation_lock: Mutex<()>,

    root: R::Of<'static>,
    major_collections: AtomicUsize,
    minor_collections: AtomicUsize,
    old_objects: Arc<AtomicUsize>,

    // time stored in milisceonds
    minor_collect_avg_time: AtomicUsize,
    major_collect_avg_time: AtomicUsize,

    time_slicer: TimeSlicer,
}

impl<R: ForLt> Collect for Collector<R>
where
    for<'a> <R as ForLt>::Of<'a>: Trace,
{
    fn major_collect(&self) {
        let _collection_lock = self.collection_lock.lock().unwrap();
        let mutation_lock = self.mutation_lock.lock().unwrap();
        let start_time = Instant::now();
        self.old_objects.store(0, Ordering::Relaxed);
        self.rotate_mark(); // major collection rotates the mark!
        self.collect();

        // SAFETY: at this point there are no mutators and all garbage collected
        // values have been marked with the current_mark
        unsafe {
            self.arena.sweep(self.get_current_mark(), || {
                drop(mutation_lock);
            });
        }

        self.major_collections.fetch_add(1, Ordering::Relaxed);

        // update collection time
        let elapsed_time = start_time.elapsed().as_millis() as usize;
        self.update_collection_time(
            &self.major_collect_avg_time,
            elapsed_time,
            self.get_major_collections(),
        );
    }

    fn minor_collect(&self) {
        let _collection_lock = self.collection_lock.lock().unwrap();
        let mutation_lock = self.mutation_lock.lock().unwrap();
        let start_time = Instant::now();
        self.collect();

        // SAFETY: at this point there are no mutators and all garbage collected
        // values have been marked with the current_mark
        unsafe {
            self.arena.sweep(self.get_current_mark(), || {
                drop(mutation_lock);
            });
        }

        self.minor_collections.fetch_add(1, Ordering::Relaxed);

        // update collection time
        let elapsed_time = start_time.elapsed().as_millis() as usize;
        self.update_collection_time(
            &self.minor_collect_avg_time,
            elapsed_time,
            self.get_minor_collections(),
        );
    }

    fn get_major_collections(&self) -> usize {
        self.major_collections.load(Ordering::Relaxed)
    }

    fn get_minor_collections(&self) -> usize {
        self.minor_collections.load(Ordering::Relaxed)
    }

    fn get_arena_size(&self) -> usize {
        self.arena.get_size()
    }

    fn get_old_objects_count(&self) -> usize {
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
        } else if self.tracer.yield_flag() {
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
    pub fn new<F>(f: F, config: &GcConfig) -> Self
    where
        F: for<'gc> FnOnce(&'gc Mutator<'gc>) -> R::Of<'gc>,
    {
        let arena = Allocator::new();
        let tracer = Arc::new(TracerController::new(config));
        let tracer_ref: &'static TracerController =
            unsafe { &*(&*tracer as *const TracerController) };
        let lock = tracer_ref.yield_lock();
        let mutator = Mutator::new(arena.clone(), tracer_ref, lock);
        let mutator_ref: &'static Mutator<'static> =
            unsafe { &*(&mutator as *const Mutator<'static>) };

        let root: R::Of<'static> = f(mutator_ref);
        let time_slicer = TimeSlicer::new(
            tracer.clone(),
            arena.clone(),
            config.monitor_arena_size_ratio_trigger,
            config.collector_max_headroom_ratio,
            config.collector_timeslice_size,
            config.collector_slice_min,
        );

        Self {
            arena,
            tracer,
            root,
            collection_lock: Mutex::new(()),
            mutation_lock: Mutex::new(()),
            major_collections: AtomicUsize::new(0),
            minor_collections: AtomicUsize::new(0),
            major_collect_avg_time: AtomicUsize::new(0),
            minor_collect_avg_time: AtomicUsize::new(0),
            old_objects: Arc::new(AtomicUsize::new(0)),
            time_slicer,
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

    unsafe fn scoped_root<'gc>(&self) -> &'gc R::Of<'gc> {
        std::mem::transmute::<&R::Of<'static>, &R::Of<'gc>>(&self.root)
    }

    fn new_mutator(&self) -> Mutator {
        let _mutation_lock = self.mutation_lock.lock().unwrap();
        let yield_lock = self.tracer.yield_lock();

        Mutator::new(self.arena.clone(), self.tracer.as_ref(), yield_lock)
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
        self.tracer.rotate_mark()
    }

    fn get_current_mark(&self) -> GcMark {
        self.tracer.get_current_mark()
    }

    fn collect(&self) {
        self.tracer
            .clone()
            .trace(&self.root, self.old_objects.clone(), || {
                self.time_slicer.run();
            });
    }
}
