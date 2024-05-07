use super::marker::Marker;
use super::trace::Trace;
use super::trace_metrics::TraceMetrics;
use super::trace_packet::TracePacket;
use super::tracer::TraceWorker;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex, RwLock, RwLockReadGuard,
};

const NUM_TRACER_THREADS: usize = 2;

pub struct TracerController<M: Marker> {
    yield_flag: AtomicBool,
    yield_lock: RwLock<()>,
    unscanned: Mutex<Vec<TracePacket<M>>>, // TODO: store in GcArray instead of vec
    metrics: Mutex<TraceMetrics>,
    // optional start time
}

impl<M: Marker> Default for TracerController<M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M: Marker> TracerController<M> {
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

    pub fn push_packet(&self, packet: TracePacket<M>) {
        self.unscanned.lock().unwrap().push(packet);
    }

    pub fn pop_packet(&self) -> Option<TracePacket<M>> {
        self.unscanned.lock().unwrap().pop()
    }

    pub fn metrics(&self) -> TraceMetrics {
        *self.metrics.lock().unwrap()
    }

    pub fn trace<T: Trace>(self: Arc<Self>, root: &T, marker: M) {
        // Perform initial trace.
        self.clone().spawn_tracers(Some(root), &marker);

        // We are about to begin the final trace, first, we signal to the mutators
        // to yield.
        //
        // The yield flag may have already have bin raised if the initial
        // trace had been running for a long time, or if space is running low.
        self.yield_flag.store(true, Ordering::SeqCst);

        // Now that the yield flag is set, the mutators *should* yield once they
        // see the yield_flag.
        // Then we wait until all mutators have stopped by grabbing the yield lock.
        let _lock = self.yield_lock.write().unwrap();

        // Now that all mutators are stopped we do a final trace.
        // This final trace ensures we trace any remaining objects that were
        self.clone().spawn_tracers(None as Option<&T>, &marker);

        // tracing
        self.yield_flag.store(false, Ordering::SeqCst);
    }

    pub fn spawn_tracers<T: Trace>(self: Arc<Self>, root: Option<&T>, marker: &M) {
        std::thread::scope(|scope| {
            for i in 0..NUM_TRACER_THREADS {
                let mut tracer = TraceWorker::new(self.clone(), marker.clone());

                if i == 0 && root.is_some() {
                    tracer.trace_obj(root.unwrap())
                }

                scope.spawn(move || {
                    tracer.trace_loop();
                });
            }
        });
    }
}
