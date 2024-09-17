use super::config::GcConfig;
use super::header::GcMark;
use super::mutator::Mutator;
use super::trace::{Trace, TracerController};
use super::allocator::Allocator;
use log::{info, debug};
use higher_kinded_types::ForLt;
use std::time::{Duration, Instant};

use std::sync::{
    atomic::{AtomicU8, AtomicUsize, Ordering},
    Arc, Mutex,
};

#[repr(u8)]
#[derive(Clone, Debug)]
pub enum GcState {
    Waiting,
    Marking,
    Finishing,
}

pub trait Collect {
    fn major_collect(&self);
    fn minor_collect(&self);
    fn wait_for_collection(&self);

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
    lock: Mutex<()>,
    root: R::Of<'static>,
    major_collections: AtomicUsize,
    minor_collections: AtomicUsize,
    old_objects: Arc<AtomicUsize>,
    // time stored in milisceonds
    minor_collect_avg_time: AtomicUsize,
    major_collect_avg_time: AtomicUsize,

    //config vars
    arena_size_ratio_trigger: f32,
    max_headroom_ratio: f32,
    timeslice_size: f32,
    slice_min: f32,
}

impl<R: ForLt> Collect for Collector<R>
where
    for<'a> <R as ForLt>::Of<'a>: Trace,
{
    fn wait_for_collection(&self) {
        let _lock = self.lock.lock().unwrap();
    }

    fn major_collect(&self) {
        info!("MAJOR COLLECT: START");
        let _lock = self.lock.lock().unwrap();
        let start_time = Instant::now();
        self.old_objects.store(0, Ordering::SeqCst);
        self.rotate_mark(); // major collection rotates the mark!
        self.collect();
        self.major_collections.fetch_add(1, Ordering::SeqCst);

        // update collection time
        let elapsed_time = start_time.elapsed().as_millis() as usize;
        self.update_collection_time(
            &self.major_collect_avg_time,
            elapsed_time,
            self.get_major_collections(),
        );
        info!("MAJOR COLLECT: END");
    }

    fn minor_collect(&self) {
        info!("MINOR COLLECT: START");
        let _lock = self.lock.lock().unwrap();
        let start_time = Instant::now();
        self.collect();
        self.minor_collections.fetch_add(1, Ordering::SeqCst);

        // update collection time
        let elapsed_time = start_time.elapsed().as_millis() as usize;
        self.update_collection_time(
            &self.minor_collect_avg_time,
            elapsed_time,
            self.get_minor_collections(),
        );
        info!("MINOR COLLECT: END");
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

    fn get_state(&self) -> GcState {
        if self.lock.try_lock().is_ok() {
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
        unsafe {
            let arena = Allocator::new();
            let tracer = Arc::new(TracerController::new(config));
            let tracer_ref: &'static TracerController =
                &*(&*tracer as *const TracerController);
            let lock = tracer_ref.yield_lock();
            let mutator: &'static Mutator<'static> =
                &*(&Mutator::new(arena.clone(), tracer_ref, lock)
                    as *const Mutator<'static>);
            let root: R::Of<'static> = f(mutator);

            Self {
                arena,
                tracer,
                root,
                lock: Mutex::new(()),
                major_collections: AtomicUsize::new(0),
                minor_collections: AtomicUsize::new(0),
                major_collect_avg_time: AtomicUsize::new(0),
                minor_collect_avg_time: AtomicUsize::new(0),
                old_objects: Arc::new(AtomicUsize::new(0)),
                arena_size_ratio_trigger: config.monitor_arena_size_ratio_trigger,
                max_headroom_ratio: config.collector_max_headroom_ratio,
                timeslice_size: config.collector_timeslice_size,
                slice_min: config.collector_slice_min,
            }
        }
    }

    pub fn mutate<F, O>(&self, f: F) -> O
    where
        F: for<'gc> FnOnce(&'gc Mutator<'gc>, &'gc R::Of<'gc>) -> O,
    {
        unsafe {
            let mutator = self.new_mutator();
            let root = self.scoped_root();

            f(&mutator, root)
        }
    }

    unsafe fn scoped_root<'gc>(&self) -> &'gc R::Of<'gc> {
        std::mem::transmute::<&R::Of<'static>, &R::Of<'gc>>(&self.root)
    }

    fn new_mutator(&self) -> Mutator {
        let _collection_lock = self.lock.lock().unwrap();
        let lock = self.tracer.yield_lock();

        Mutator::new(self.arena.clone(), self.tracer.as_ref(), lock)
    }

    fn update_collection_time(
        &self,
        average: &AtomicUsize,
        elapsed_time: usize,
        num_collections: usize,
    ) {
        let avg = average.load(Ordering::SeqCst);
        let update = elapsed_time.abs_diff(avg) / num_collections;

        if avg > elapsed_time {
            average.fetch_sub(update, Ordering::SeqCst);
        } else {
            average.fetch_add(update, Ordering::SeqCst);
        }
    }

    fn split_timeslice(&self, max_headroom: usize, prev_size: usize) -> (Duration, Duration) {
        // Algorithm inspired from webkit riptide collector:
        let one_mili_in_nanos = 1_000_000.0;
        let available_headroom = (max_headroom + prev_size) - self.arena.get_size();
        let headroom_ratio = available_headroom as f32 / max_headroom as f32;
        let m = (self.timeslice_size - self.slice_min) * headroom_ratio;
        let mutator_nanos = (one_mili_in_nanos * m) as u64;
        let collector_nanos = (self.timeslice_size * one_mili_in_nanos) as u64 - mutator_nanos;
        let mutator_duration = Duration::from_nanos(mutator_nanos);
        let collector_duration = Duration::from_nanos(collector_nanos);

        debug!("TIMESLICE SPLIT :: MUT = {mutator_duration:?}, COL = {collector_duration:?}");

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

            // TODO: rename this from write_barrier_lock, to like space_time_lock ormaybe
            // maybe mutator lock
            let _lock = self.tracer.get_alloc_lock();
            std::thread::sleep(collector_duration);

            if !self.tracer.is_tracing() {
                break;
            }
        }
    }

    fn rotate_mark(&self) -> GcMark {
        self.tracer.rotate_mark()
    }

    fn get_current_mark(&self) -> GcMark {
        self.tracer.get_current_mark()
    }

    // TODO: differentiate sync and concurrent collections
    // sync collections need not track headroom
    fn collect(&self) {
        let join_handles = self.tracer.clone().trace(&self.root, self.old_objects.clone());
        // TODO: should space & time managing be done in a separate thread? otherwise a collection is guaranteed
        // to take 1.4ms
        self.run_space_time_manager();
        //println!("WAITING FOR TRACER JOIN HANDLES");
        for jh in join_handles.into_iter() {
            jh.join().expect("Tracer Returned OK");
        }
        self.tracer.wait_for_trace_completion();
        let current_mark = self.get_current_mark();

        self.arena.sweep(current_mark);
    }
}
