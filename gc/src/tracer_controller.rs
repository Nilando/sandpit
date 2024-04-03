use super::allocate::{Allocate, GenerationalArena};
use super::tracer::Tracer;
use super::tracer::TracerWorker;
use super::GcPtr;
use super::Trace;
use std::ptr::NonNull;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    RwLock, RwLockReadGuard,
};

pub const TRACE_PACKET_SIZE: usize = 100;
pub const WORKER_COUNT: usize = 5;

pub type UnscannedPtr<T> = (NonNull<()>, fn(NonNull<()>, &mut T));
pub struct TracePacket<T>([Option<UnscannedPtr<T>>; TRACE_PACKET_SIZE]);
use std::sync::{Arc, Mutex};

impl<T: Tracer> TracePacket<T> {
    pub fn new() -> Self {
        Self([None; TRACE_PACKET_SIZE])
    }

    pub fn get(&self, idx: usize) -> Option<UnscannedPtr<T>> {
        self.0[idx]
    }

    pub fn set(&mut self, idx: usize, job: Option<UnscannedPtr<T>>) {
        self.0[idx] = job;
    }
}

pub struct TracerController<A: Allocate> {
    yield_flag: AtomicBool,
    yield_lock: RwLock<()>,
    unscanned: Arc<Mutex<Vec<TracePacket<TracerWorker<A>>>>>, // TODO: store in GcArray instead of vec
                                                              // metrics?
}

impl<A: Allocate> TracerController<A> {
    pub fn new() -> Self {
        Self {
            yield_lock: RwLock::new(()),
            yield_flag: AtomicBool::new(false),
            unscanned: Arc::new(Mutex::new(vec![])),
        }
    }

    pub fn get_yield_lock(&self) -> RwLockReadGuard<()> {
        self.yield_lock.read().unwrap()
    }

    pub fn get_yield_flag(&self) -> bool {
        self.yield_flag.load(Ordering::Relaxed)
    }

    pub fn eden_collection<T: Trace>(&self, arena: &<A as Allocate>::Arena, root: GcPtr<T>) {
        let current_mark = arena.current_mark();

        // create the first unscanned packet with the root as the only job
        let mut packet = TracePacket::<TracerWorker<A>>::new();
        let obj_ptr: NonNull<()> = root.as_ptr().cast();
        let job: UnscannedPtr<TracerWorker<A>> = (obj_ptr, T::dyn_trace);
        packet.set(0, Some(job));
        self.unscanned.lock().unwrap().push(packet);

        // TODO: Start a "space and time" manager
        // ie we need a way to request a yield in case tracing is taking too long

        self.run_tracers(current_mark);
        self.yield_flag.store(true, Ordering::Relaxed);
        let _lock = self.yield_lock.write().unwrap();

        // we need to trace again, b/c now that the tracer handles have been dropped,
        // there may be more work to do
        self.run_tracers(current_mark);

        arena.refresh();
    }

    pub fn full_collection<T: Trace>(&self, arena: &<A as Allocate>::Arena, root: GcPtr<T>) {
        arena.rotate_mark();
        self.eden_collection(arena, root);
    }

    fn run_tracers(&self, mark: <<A as Allocate>::Arena as GenerationalArena>::Mark) {
        std::thread::scope(|s| {
            for _ in 0..WORKER_COUNT {
                let unscanned = self.unscanned.clone();
                let mut worker = TracerWorker::new(unscanned, mark);

                s.spawn(move || {
                    worker.trace()
                });
            }
        });
    }
}
