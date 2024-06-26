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
pub struct GcArena<T: Trace> {
    collector: Arc<Collector<Allocator, T>>,
    monitor: Arc<Monitor<Collector<Allocator, T>>>,
    config: GcConfig,
}

unsafe impl<T: Send + Trace> Send for GcArena<T> {}
unsafe impl<T: Sync + Trace> Sync for GcArena<T> {}

impl<T: Trace> Drop for GcArena<T> {
    fn drop(&mut self) {
        self.stop_monitor();
        self.collector.wait_for_collection();
    }
}

impl<T: Trace> GcArena<T> {
    // The build callback must return a root of type T, which will permanently be the
    // arena's root type.
    pub fn build<I: TraceLeaf>(input: I, callback: fn(&mut MutatorScope<Allocator>, I) -> T) -> Self {
        let config = GcConfig::default();
        let collector: Arc<Collector<Allocator, T>> = Arc::new(Collector::build(input, callback, &config));
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
    pub fn mutate<I: TraceLeaf, O: TraceLeaf>(&self, input: I, callback: fn(&T, &mut MutatorScope<Allocator>, I) -> O) -> O {
        self.collector.mutate(input, callback)
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
