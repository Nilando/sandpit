use super::allocator::Allocate;
use super::collector::Collector;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

use std::thread;
use std::time;

pub struct Monitor<A: Allocate> {
    collector: Arc<Collector<A>>,
    monitor_lock: Mutex<()>,
}

unsafe impl<A: Allocate> Send for Monitor<A> {}
unsafe impl<A: Allocate> Sync for Monitor<A> {}


impl<A: Allocate> Monitor<A> {
    pub fn new(collector: Arc<Collector<A>>) -> Self {
        Self {
            collector,
            monitor_lock: Mutex::new(())
        }
    }

    pub fn stop(&self) {
    }

    pub fn start(&self) {
    }
}
