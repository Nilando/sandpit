use super::allocate::{Allocate, GenerationalArena};
use super::trace_packet::TracePacket;
use super::tracer::TracerWorker;
use super::GcPtr;
use super::Trace;
use std::ptr::NonNull;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    RwLock, RwLockReadGuard,
    Arc, Mutex
};

pub const WORKER_COUNT: usize = 2;

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
        packet.push_gc_ptr(root);
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
