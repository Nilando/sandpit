use std::marker::PhantomData;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    RwLock,
    RwLockReadGuard
};
use super::allocate::{Allocate, GenerationalArena};
use super::Trace;
use super::GcPtr;

const WORKER_COUNT: usize = 5;

pub trait Tracer {}
impl<A: Allocate> Tracer for TracerController<A> {}

pub struct TracerController<A: Allocate> {
    _allocator: PhantomData<A>,
    yield_flag: AtomicBool,
    yield_lock: RwLock<()>,
    // Used so that multiple mutator scopes can hold the lock,
    // but only on tracer can hold
    // tracer needs A: Allocate so that it can mark gc ptrs correctly
    // unscanned objects(work) *should be divided up into work packets
    // metrics?
    // threads?
}

impl<A: Allocate> TracerController<A> {
    pub fn new() -> Self {
        Self {
            _allocator: PhantomData::<A>,
            yield_lock: RwLock::new(()),
            yield_flag: AtomicBool::new(false),
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

    pub fn full_collection<G: GenerationalArena, T: Trace>(&self, arena: &G, root: GcPtr<T>) {
        // add the roots to unscanned objects
        // spin up worker threads
        //
        for i in 0..WORKER_COUNT {

        }
        let thread = std::thread::spawn(|| {
            // get unscanned work, scan it
            // if a work packet is filled, send it to the controller
            // if no unscanned work left, grab one from the controller
            // if no unscanned work from the controller, or a certain amount of time/debt has
            // passed, then request a yield
        });
        thread.join().unwrap();
        self.yield_flag.store(true, Ordering::Relaxed);
        let _lock = self.yield_lock.write().unwrap();
        arena.refresh();
    }
}
