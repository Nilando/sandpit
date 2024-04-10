use super::allocate::{Allocate, GenerationalArena};
use super::trace_packet::TracePacket;
use super::tracer::TracerWorker;
use super::GcPtr;
use super::Trace;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex, RwLock, RwLockReadGuard,
};

pub const WORKER_COUNT: usize = 1;

pub struct TracerController<A: Allocate> {
    yield_flag: AtomicBool,
    yield_lock: RwLock<()>,
    trace_flag: AtomicBool,
    unscanned: Arc<Mutex<Vec<TracePacket<TracerWorker<A>>>>>, // TODO: store in GcArray instead of vec
}

impl<A: Allocate> TracerController<A> {
    pub fn new() -> Self {
        Self {
            yield_lock: RwLock::new(()),
            yield_flag: AtomicBool::new(false),
            trace_flag: AtomicBool::new(false),
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
        let is_tracing = self.trace_flag.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst);
        if is_tracing.is_err() { return; }

        self.trace(arena, root);
        self.trace_flag.store(false, Ordering::SeqCst);
    }

    pub fn full_collection<T: Trace>(&self, arena: &<A as Allocate>::Arena, root: GcPtr<T>) {
        let is_tracing = self.trace_flag.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst);
        if is_tracing.is_err() { return; }

        arena.rotate_mark();
        self.eden_collection(arena, root);

        self.trace_flag.store(false, Ordering::SeqCst);
    }

    pub fn push_packet(&self, packet: TracePacket::<TracerWorker<A>>) {
        self.unscanned.lock().unwrap().push(packet);
    }

    fn trace<T: Trace>(&self, arena: &<A as Allocate>::Arena, root: GcPtr<T>) {
        self.init_unscanned(root);
        self.run_tracers(arena.current_mark());
        self.final_trace(arena);
    }

    fn final_trace(&self, arena: &<A as Allocate>::Arena) {
        self.yield_flag.store(true, Ordering::SeqCst);
        let _lock = self.yield_lock.write().unwrap();

        self.run_tracers(arena.current_mark());
        arena.refresh();

        self.yield_flag.store(false, Ordering::SeqCst);
    }

    fn init_unscanned<T: Trace>(&self, root: GcPtr<T>) {
        let mut packet = TracePacket::<TracerWorker<A>>::new();

        packet.push_gc_ptr(root);
        self.unscanned.lock().unwrap().push(packet);
    }

    fn run_tracers(&self, mark: <<A as Allocate>::Arena as GenerationalArena>::Mark) {
        // TODO: better divide work between threads
        let unscanned = self.unscanned.clone();
        let mut worker = TracerWorker::new(unscanned, mark);

        worker.trace();
        /*
        std::thread::scope(|s| {
            for _ in 0..WORKER_COUNT {
                let unscanned = self.unscanned.clone();
                let mut worker = TracerWorker::new(unscanned, mark);

                let thread = s.spawn(move || worker.trace());
                if thread.join().is_err() {
                    println!("tracer panicked");
                    panic!("A tracer panicked");
                }
            }
        });
        */
    }
}
