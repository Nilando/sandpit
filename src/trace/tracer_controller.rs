use super::trace::Trace;
use super::trace_job::TraceJob;
use super::tracer::Tracer;
use crate::config::Config;
use crate::debug::gc_debug;
use crate::header::GcMark;
use crate::heap::{Allocator, Heap};
use crossbeam_channel::{Receiver, Sender};
use core::sync::atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering};
use alloc::sync::Arc;

use std::sync::{RwLock, RwLockReadGuard};
use std::time::Instant;

pub struct TracerController {
    sender: Sender<Vec<TraceJob>>,
    receiver: Receiver<Vec<TraceJob>>,

    heap: Heap,

    yield_flag: AtomicBool,
    current_mark: AtomicU8,

    // mutators hold a ReadGuard of this lock preventing
    // the tracers from declaring the trace complete until
    // all mutators are stopped.
    yield_lock: RwLock<()>,

    // config vars
    pub num_tracers: usize,
    pub trace_share_min: usize,
    pub trace_chunk_size: usize,
    pub trace_share_ratio: f32,
    pub trace_wait_time: u64,
    pub mutator_share_min: usize,
}

impl TracerController {
    pub fn new(config: &Config) -> Self {
        let (sender, receiver) = crossbeam_channel::unbounded();
        let heap = Heap::new();

        Self {
            heap,
            sender,
            receiver,

            yield_flag: AtomicBool::new(false),
            yield_lock: RwLock::new(()),
            current_mark: AtomicU8::new(GcMark::Red.into()),

            num_tracers: config.tracer_threads,
            trace_share_min: config.trace_share_min,
            trace_chunk_size: config.trace_chunk_size,
            trace_share_ratio: config.trace_share_ratio,
            trace_wait_time: config.trace_wait_time,
            mutator_share_min: config.mutator_share_min,
        }
    }

    pub fn new_allocator(&self) -> Allocator {
        Allocator::from(&self.heap)
    }

    pub fn trace_and_sweep<T: Trace>(
        &self,
        root: &T,
        old_object_count: Arc<AtomicU64>,
    ) {
        self.trace(root, old_object_count);
        unsafe { self.sweep(); }
    }

    fn trace<T: Trace>(
        &self,
        root: &T,
        old_object_count: Arc<AtomicU64>,
    ) {
        gc_debug("Begining trace...");

        self.trace_root(root, old_object_count.clone());
        self.spawn_tracers(old_object_count);

        gc_debug("Trace Complete!");

        self.clean_up();
    }

    fn spawn_tracers(&self, old_object_count: Arc<AtomicU64>) {
        let object_count = old_object_count.clone();

        std::thread::scope(|scope| {
            for _ in 0..self.num_tracers {
                scope.spawn(|| {
                    let mut tracer = self.new_tracer();

                    gc_debug("Tracer Thread Spawned");

                    let marked_objects = tracer.trace_loop() as u64;

                    object_count.fetch_add(marked_objects, Ordering::SeqCst);
                });
            }
        });
    }

    fn trace_root<T: Trace>(&self, root: &T, old_object_count: Arc<AtomicU64>) {
        let mut tracer = self.new_tracer();
        root.trace(&mut tracer);
        tracer.flush_work();
        old_object_count.fetch_add(tracer.get_mark_count() as u64, Ordering::SeqCst);
    }

    fn new_tracer(&self) -> Tracer {
        let mark = self.get_current_mark();

        Tracer::new(self, mark)
    }

    pub fn send_work(&self, work: Vec<TraceJob>) {
        self.sender.send(work).unwrap();
    }

    pub fn recv_work(&self) -> Option<Vec<TraceJob>> {
        let duration = std::time::Duration::from_millis(self.trace_wait_time);
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
        self.trace_share_ratio
    }

    pub fn has_work(&self) -> bool {
        !self.receiver.is_empty()
    }

    fn mutators_stopped(&self) -> bool {
        self.yield_lock.try_write().is_ok()
    }

    pub fn get_arena_size(&self) -> u64 {
        self.heap.get_size()
    }

    // SAFETY: at this point there are no mutators and all garbage collected
    // values have been marked with the current_mark
    unsafe fn sweep(&self) {
        gc_debug("Sweeping...");
        self.heap.sweep(self.get_current_mark());
    }
}
