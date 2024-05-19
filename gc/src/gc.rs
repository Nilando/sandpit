use super::collector::{Collect, Collector};
use super::metrics::GcMetrics;
use super::monitor::Monitor;
use super::mutator::MutatorScope;
use super::trace::Trace;
use std::sync::Arc;

use super::allocator::Allocator;

/// A garbage collected arena through objects can be allocated into.
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
    // The build callback must return a root of type T, which will permanently be the
    // Gc's root type.
    pub fn build(callback: fn(&mut MutatorScope<Allocator>) -> T) -> Self {
        let collector: Arc<Collector<Allocator, T>> = Arc::new(Collector::build(callback));
        let monitor = Arc::new(Monitor::new(collector.clone()));

        Self { collector, monitor }
    }

    // MutatorScope is a sealed type but the user utilize it through the public
    // Mutator trait which in implements. Here &T is the root.
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

    pub fn metrics(&self) -> GcMetrics {
        GcMetrics {
            // Collector Metrics:

            // Running count of how many times major/minor collections have happend.
            major_collections: self.collector.get_major_collections(),
            minor_collections: self.collector.get_minor_collections(),

            // How many old objects there were as per the last trace.
            old_objects_count: self.collector.get_old_objects_count(),
            // The current size of the arena including large objects and blocks.
            arena_size: self.collector.get_arena_size(),

            // Monitor Metrics:

            // How many old objects must exist before a major collection is triggered.
            // If you divide this number by the monitor's 'MAX_OLD_GROWTH_RATE, you get the number
            // of old objects at the end of the last major collection
            max_old_objects: self.monitor.get_max_old_objects(),
            // The size of the arena at the end of the last collection.
            prev_arena_size: self.monitor.get_prev_arena_size(),
        }
    }
}
