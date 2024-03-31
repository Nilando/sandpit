use std::marker::PhantomData;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    RwLock,
    RwLockReadGuard
};
use super::allocate::Allocate;

pub struct Tracer<A: Allocate> {
    _allocator: PhantomData<A>,
    yield_flag: AtomicBool,
    yield_lock: RwLock<()>,
    // Used so that multiple mutator scopes can hold the lock,
    // but only on tracer can hold
    // tracer needs A: Allocate so that it can mark gc ptrs correctly
    // and so that it can signal to the allocator when to begin partial or full tracing
    // unscanned objects(work) *should be divided up into work packets
    // yield lock
    // yield flag
    // metrics?
    // threads?
    // how would the tracer wait 
}

impl<A: Allocate> Tracer<A> {
    pub fn new() -> Self {
        Self {
            _allocator: PhantomData::<A>,
            yield_lock: RwLock::new(()),
            yield_flag: AtomicBool::new(false),
        }
    }

    /*
    pub fn get_yield_lock(&self) -> &RwLockReadGuard<()> {
        &self.yield_lock.read().unwrap()
    }
    */

    pub fn get_yield_flag(&self) -> &AtomicBool {
        &self.yield_flag
    }

    pub fn eden_collection(&self) {
        self.yield_flag.store(true, Ordering::Relaxed);
        self.yield_lock.write().unwrap();
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

    pub fn full_collection(&self) {
        // reqeust a yield
        // collect roots
        // remove yield
        // add the roots to unscanned objects
        // spin up worker threads to go through unscanned objects
        // once work is about gone
        // request a yield
        // free memory
        // remove yield
    }
}
