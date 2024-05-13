use super::monitor::Monitor as GenericMonitor;
use std::collections::HashMap;
use std::sync::{Arc, RwLockReadGuard};
use super::mutator::MutatorScope;
use super::trace::Trace;
use super::collector::{Collector as GenericCollector};

// This allocator can be swapped out and everything should just work...
use super::allocator::Allocator;

type Collector = GenericCollector<Allocator>;
type Monitor = GenericMonitor<Allocator>;

pub struct Gc<T: Trace> {
    collector: Arc<Collector>,
    monitor: Arc<Monitor>,
    root: T,
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
        let collector = Arc::new(Collector::new());
        let binding = collector.clone();
        let (mut mutator, _lock) = binding.new_mutator();
        let root = callback(&mut mutator);

        drop(mutator);

        let monitor = Arc::new(Monitor::new(collector.clone()));

        Self {
            collector,
            monitor,
            root,
        }
    }

    pub fn mutate(&self, callback: fn(&T, &mut MutatorScope<Allocator>)) {
        let (mut mutator, _lock) = self.collector.new_mutator();

        callback(&self.root, &mut mutator);
    }

    pub fn major_collect(&self) {
        self.collector.major_collect(&self.root);
    }

    pub fn minor_collect(&self) {
        self.collector.minor_collect(&self.root);
    }

    pub fn start_monitor(&self) {
        self.monitor.start();
    }

    pub fn stop_monitor(&self) {
        self.monitor.stop();
    }

    pub fn metrics(&self) -> HashMap<String, usize> {
        todo!()
        //self.collector.metrics()
    }
}
