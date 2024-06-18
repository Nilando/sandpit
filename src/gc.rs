use super::collector::{Collect, Collector};
use super::config::GcConfig;
use super::metrics::GcMetrics;
use super::monitor::Monitor;
use super::mutator::MutatorScope;
use super::trace::Trace;
use super::trace::TraceLeaf;
use std::sync::Arc;

use super::allocator::Allocator;

/// A garbage collected arena where objects can be allocated into.
pub struct Gc<T: Trace> {
    collector: Arc<Collector<Allocator, T>>,
    monitor: Arc<Monitor<Collector<Allocator, T>>>,
    config: GcConfig,
}

unsafe impl<T: Send + Trace> Send for Gc<T> {}
unsafe impl<T: Sync + Trace> Sync for Gc<T> {}

impl<T: Trace> Drop for Gc<T> {
    fn drop(&mut self) {
        self.stop_monitor();
        self.collector.wait_for_collection()
    }
}

impl<T: Trace> Gc<T> {
    // The build callback must return a root of type T, which will permanently be the
    // Gc's root type.
    pub fn build(callback: fn(&mut MutatorScope<Allocator>) -> T) -> Self {
        let config = GcConfig::default();
        let collector: Arc<Collector<Allocator, T>> = Arc::new(Collector::build(callback, &config));
        let monitor = Arc::new(Monitor::new(collector.clone(), &config));

        if config.monitor_on {
            monitor.clone().start();
        }

        Self {
            collector,
            monitor,
            config,
        }
    }

    // MutatorScope is a sealed type but the user utilize it through the public
    // Mutator trait which in implements. Here &T is the root.
    pub fn mutate(&self, callback: fn(&T, &mut MutatorScope<Allocator>)) {
        self.collector.mutate(callback);
    }

    pub fn mutate_io<I: TraceLeaf, O: TraceLeaf>(
        &self,
        callback: fn(&T, &mut MutatorScope<Allocator>, input: I) -> O,
        input: I,
    ) -> O {
        self.collector.mutate_io(callback, input)
    }

    pub fn insert<L: TraceLeaf>(&self, value: L, callback: fn(&T, L)) {
        self.collector.insert(callback, value);
    }

    pub fn extract<L: TraceLeaf>(&self, callback: fn(&T) -> L) -> L {
        self.collector.extract(callback)
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

    pub fn get_config(&self) -> GcConfig {
        self.config
    }

    pub fn metrics(&self) -> GcMetrics {
        self.monitor.metrics()
    }
}
