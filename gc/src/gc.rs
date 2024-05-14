use super::monitor::Monitor;
use std::collections::HashMap;
use std::sync::{Arc, RwLockReadGuard};
use super::mutator::MutatorScope;
use super::trace::Trace;
use super::collector::{Collector, Collect};

use super::allocator::Allocator;

pub struct Gc<T: Trace> {
    collector: Arc<Collector<Allocator, T>>,
    monitor: Arc<Monitor<Collector<Allocator, T>>>,
}

unsafe impl<T: Send + Trace> Send for Gc<T> {}
unsafe impl<T: Sync + Trace> Sync for Gc<T> {}

impl<T: Trace> Drop for Gc<T> {
    fn drop(&mut self) {
        self.stop_monitor()
    }
}

impl<T: Trace> Gc<T> {
    pub fn build(callback: fn(&mut MutatorScope<Allocator>) -> T) -> Self {
        let collector: Arc<Collector<Allocator, T>> = Arc::new(Collector::build(callback));
        let monitor = Arc::new(Monitor::new(collector.clone()));

        Self {
            collector,
            monitor,
        }
    }

    pub fn mutate(&self, callback: fn(&T, &mut MutatorScope<Allocator>)) {
        self.collector.mutate(callback);
    }

    pub fn major_collect(&self) {
        self.collector.major_collect();
    }

    pub fn minor_collect(&self) {
        self.collector.minor_collect();
    }

    pub fn start_monitor(&self) {
        self.monitor.clone().start();
    }

    pub fn stop_monitor(&self) {
        self.monitor.stop();
    }

    pub fn metrics(&self) -> HashMap<String, usize> {
        todo!()
    }
}
