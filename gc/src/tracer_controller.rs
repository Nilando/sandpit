use super::allocate::{Allocate, GenerationalArena};
use super::trace_metrics::TraceMetrics;
use super::trace_packet::TracePacket;
use super::tracer::TracerWorker;
use super::Trace;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard,
};

pub struct TracerController<A: Allocate> {
    yield_lock: RwLock<()>,
    yield_flag: AtomicBool,
    trace_lock: RwLock<()>,
    unscanned: Arc<Mutex<Vec<TracePacket<TracerWorker<A>>>>>, // TODO: store in GcArray instead of vec
    metrics: Mutex<TraceMetrics>,
}

impl<A: Allocate> TracerController<A> {
    pub fn new() -> Self {
        Self {
            yield_lock: RwLock::new(()),
            yield_flag: AtomicBool::new(false),
            trace_lock: RwLock::new(()),
            unscanned: Arc::new(Mutex::new(vec![])),
            metrics: Mutex::new(TraceMetrics::new()),
        }
    }

    pub fn get_yield_lock(&self) -> RwLockReadGuard<()> {
        self.yield_lock.read().unwrap()
    }

    pub fn is_tracing(&self) -> bool {
        self.trace_lock.try_read().is_err()
    }

    pub fn wait_for_trace(&self) {
        drop(self.trace_lock.read().unwrap());
    }

    pub fn get_yield_flag(&self) -> bool {
        self.yield_flag.load(Ordering::SeqCst)
    }

    pub fn eden_collection<T: Trace>(&self, arena: &<A as Allocate>::Arena, root: &T) {
        let _lock = self.start_trace();

        self.trace(arena, root);
    }

    pub fn full_collection<T: Trace>(&self, arena: &<A as Allocate>::Arena, root: &T) {
        let _lock = self.start_trace();

        arena.rotate_mark();

        self.trace(arena, root);
    }

    pub fn push_packet(&self, packet: TracePacket<TracerWorker<A>>) {
        self.unscanned.lock().unwrap().push(packet);
    }

    pub fn metrics(&self) -> TraceMetrics {
        *self.metrics.lock().unwrap()
    }

    fn start_trace(&self) -> Result<RwLockWriteGuard<()>, ()> {
        if self.is_tracing() {
            Err(()) 
        } else {
            Ok(self.trace_lock.write().unwrap())
        }
    }

    fn trace<T: Trace>(&self, arena: &<A as Allocate>::Arena, root: &T) {
        let unscanned = self.unscanned.clone();
        let mut worker = TracerWorker::new(unscanned, arena.current_mark());
        worker.init(root);
        worker.trace();

        self.yield_flag.store(true, Ordering::SeqCst);
        let _lock = self.yield_lock.write().unwrap();

        worker.trace();
        arena.refresh();
        *self.metrics.lock().unwrap() = worker.get_metrics();

        self.yield_flag.store(false, Ordering::SeqCst);
    }
}
