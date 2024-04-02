use super::allocate::{Allocate, GenerationalArena};
use super::tracer::Tracer;
use super::tracer::TracerWorker;
use super::GcPtr;
use super::Trace;
use std::marker::PhantomData;
use std::ptr::NonNull;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    RwLock, RwLockReadGuard,
};

pub const WORK_PACKET_SIZE: usize = 420;
pub const WORKER_COUNT: usize = 5;

pub type UnscannedPtr<T> = (NonNull<()>, fn(NonNull<()>, &T));
pub struct TracePacket<T>([Option<UnscannedPtr<T>>; WORK_PACKET_SIZE]);
use std::sync::{Arc, Mutex};

impl<T: Tracer> TracePacket<T> {
    pub fn new() -> Self {
        Self([None; WORK_PACKET_SIZE])
    }
}

unsafe impl<T> Send for TracePacket<T> {}
unsafe impl<T> Sync for TracePacket<T> {}

pub struct TracerController<A: Allocate> {
    _allocator: PhantomData<A>,
    yield_flag: AtomicBool,
    yield_lock: RwLock<()>,
    unscanned: Arc<Mutex<Vec<TracePacket<TracerWorker<A>>>>>, // TODO: store in GcArray instead of vec
                                                              // metrics?
}

impl<A: Allocate> TracerController<A> {
    pub fn new() -> Self {
        Self {
            _allocator: PhantomData::<A>,
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

    pub fn eden_collection(&self) {
        todo!()
        // grab
        // collect roots
        // remove yield
        // add the roots to unscanned objects
        // spin up worker threads to go through unscanned objects
        // once work is about gone
        // request a yield
        // free memory
        // remove yield
    }

    pub fn full_collection<T: Trace>(&self, arena: &<A as Allocate>::Arena, root: GcPtr<T>) {
        arena.rotate_mark();
        let current_mark = arena.current_mark();

        // create the first unscanned packet with the root as the only job
        let mut packet = TracePacket::<TracerWorker<A>>::new();
        let obj_ptr: NonNull<()> = root.as_ptr().cast();
        let job: UnscannedPtr<TracerWorker<A>> = (obj_ptr, T::dyn_trace);
        packet.0[0] = Some(job);
        self.unscanned.lock().unwrap().push(packet);

        // TODO: Start a "space and time" manager
        // ie we need a way to request a yield in case tracing is taking too long

        self.run_trace(current_mark);
        self.yield_flag.store(true, Ordering::Relaxed);
        let _lock = self.yield_lock.write().unwrap();

        // we need to trace again, b/c now that the tracer handles have been dropped,
        // there may be more work to do
        self.run_trace(current_mark);

        arena.refresh();
    }

    fn run_trace(&self, mark: <<A as Allocate>::Arena as GenerationalArena>::Mark) {
        std::thread::scope(|s| {
            for _ in 0..WORKER_COUNT {
                let unscanned = self.unscanned.clone();
                let tracer_mark = mark.clone();

                s.spawn(move || {
                    TracerWorker::<A>::spawn(unscanned, tracer_mark);
                });
            }
        });
    }
}
