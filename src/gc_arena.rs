use super::collector::{Collect, Collector};
use super::config::GcConfig;
use super::metrics::GcMetrics;
use super::monitor::Monitor;
use super::mutator::MutatorScope;
use super::trace::Trace;
use super::trace::TraceLeaf;

use std::sync::Arc;
use higher_kinded_types::ForLt;

use super::allocator::Allocator;

/// A garbage collected arena where objects can be allocated into.
pub struct GcArena<R: ForLt + 'static> 
where 
    for<'a> <R as ForLt>::Of<'a>: Trace
{
    collector: Arc<Collector<Allocator, R>>,
    monitor: Arc<Monitor<Collector<Allocator, R>>>,
    config: GcConfig,
}

impl<R: ForLt> Drop for GcArena<R> 
where 
    for<'a> <R as ForLt>::Of<'a>: Trace
{
    fn drop(&mut self) {
        self.stop_monitor();
        self.collector.wait_for_collection();
    }
}

impl<R: ForLt + 'static> GcArena<R>
where 
    for<'a> <R as ForLt>::Of<'a>: Trace
{
    // The build callback must return a root of type T, which will permanently be the
    // arena's root type.
    pub fn new<F>(f: F) -> Self
    where
        F: for<'gc> FnOnce(&'gc MutatorScope<'gc, Allocator>) -> R::Of<'gc>,
    {
        let config = GcConfig::default();
        let collector: Arc<Collector<Allocator, R>> = Arc::new(Collector::new(f, &config));
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

    pub fn mutate<F, O: TraceLeaf>(&self, f: F) -> O 
    where
        F: for<'gc> FnOnce(&'gc MutatorScope<'gc, Allocator>, &'gc R::Of<'gc>) -> O
    {
        self.collector.mutate(f)
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
