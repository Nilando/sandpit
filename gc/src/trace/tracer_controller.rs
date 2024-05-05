use super::trace_metrics::TraceMetrics;
use super::trace_packet::TracePacket;
use super::tracer::Tracer;
use super::trace::Trace;
use std::sync::{
    Mutex, RwLock, RwLockReadGuard,
    atomic::{AtomicBool, Ordering},
};
use crate::allocator::{Allocate, GenerationalArena};

const NUM_TRACER_THREADS: usize = 1;

pub struct TracerController {
    yield_flag: AtomicBool,
    yield_lock: RwLock<()>,
    unscanned: Mutex<Vec<TracePacket>>, // TODO: store in GcArray instead of vec
    metrics: Mutex<TraceMetrics>,

    // By having the marking function be dyn, we can avoid having Tracers, TracePackets, and
    // the TracerController be generic on an Allocate type
}

impl TracerController {
    pub fn new() -> Self {
        Self {
            yield_flag: AtomicBool::new(false),
            yield_lock: RwLock::new(()),
            unscanned: Mutex::new(vec![]),
            metrics: Mutex::new(TraceMetrics::new()),
        }
    }

    pub fn yield_flag(&self) -> bool {
        self.yield_flag.load(Ordering::SeqCst)
    }

    pub fn yield_lock(&self) -> RwLockReadGuard<()> {
        self.yield_lock.read().unwrap()
    }

    pub fn push_packet(&self, packet: TracePacket) {
        self.unscanned.lock().unwrap().push(packet);
    }

    pub fn pop_packet(&self) -> Option<TracePacket> {
        self.unscanned.lock().unwrap().pop()
    }

    pub fn metrics(&self) -> TraceMetrics {
        *self.metrics.lock().unwrap()
    }

    pub fn trace<T: Trace, A: Allocate>(&self, root: &T, mark: <<A as Allocate>::Arena as GenerationalArena>::Mark) {
        self.spawn_tracers::<T, A>(Some(root), mark);
        self.yield_flag.store(true, Ordering::SeqCst);
        let _lock = self.yield_lock.write().unwrap();
        self.spawn_tracers::<(), A>(None, mark);
        self.yield_flag.store(false, Ordering::SeqCst);
    }

    pub fn spawn_tracers<T: Trace, A: Allocate>(&self, root: Option<&T>, mark: <<A as Allocate>::Arena as GenerationalArena>::Mark) {
        std::thread::scope(|scope| {
            for i in 0..NUM_TRACER_THREADS {
                let mut tracer = Tracer::new(self);
                if i == 0 && root.is_some() {
                    tracer.init(root.unwrap())
                }

                scope.spawn(move || {
                    tracer.trace::<A>(mark);
                });
            }
        });
    }
}
