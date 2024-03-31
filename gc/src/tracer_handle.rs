use super::allocate::Allocate;
use super::tracer::Tracer;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    RwLockReadGuard
};

pub struct TracerHandle {
    //yield_lock: &'a RwLockReadGuard<'a, ()>,
    //yield_flag: &'a AtomicBool
    // work_packet for building
    // place to send work 
}

impl TracerHandle {
    pub fn new<A: Allocate>(tracer: &Tracer<A>) -> Self {
        Self { 
            //yield_lock: tracer.get_yield_lock(),
            //yield_flag: tracer.get_yield_flag()
        }
    }
}
