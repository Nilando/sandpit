use super::collector::{Collect, Collector};
use super::config::GcConfig;
use super::metrics::GcMetrics;
use super::monitor::Monitor;
use super::mutator::Mutator;
use super::trace::Trace;

use higher_kinded_types::ForLt;
use std::sync::Arc;

/// A garbage collected arena where objects can be allocated into.
pub struct Arena<R: ForLt + 'static>
where
    for<'a> <R as ForLt>::Of<'a>: Trace,
{
    collector: Arc<Collector<R>>,
    monitor: Arc<Monitor<Collector<R>>>,
    config: GcConfig,
}

impl<R: ForLt> Drop for Arena<R>
where
    for<'a> <R as ForLt>::Of<'a>: Trace,
{
    fn drop(&mut self) {
        self.stop_monitor();
        self.collector.wait_for_collection();
    }
}

impl<R: ForLt + 'static> Arena<R>
where
    for<'a> <R as ForLt>::Of<'a>: Trace,
{
    // The build callback must return a root of type T, which will permanently be the
    // arena's root type.
    pub fn new<F>(f: F) -> Self
    where
        F: for<'gc> FnOnce(&'gc Mutator<'gc>) -> R::Of<'gc>,
    {
        let config = GcConfig::default();

        Self::new_with_config(config, f)
    }

    pub fn new_with_config<F>(config: GcConfig, f: F) -> Self
    where
        F: for<'gc> FnOnce(&'gc Mutator<'gc>) -> R::Of<'gc>,
    {
        let collector: Arc<Collector<R>> = Arc::new(Collector::new(f, &config));
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

    pub fn mutate<F, O>(&self, f: F) -> O
    where
        F: for<'gc> FnOnce(&'gc Mutator<'gc>, &'gc R::Of<'gc>) -> O,
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
        GcMetrics {
            //
            state: self.collector.get_state(),

            // Collector Metrics:

            // Running count of how many times major/minor collections have happend.
            major_collections: self.collector.get_major_collections(),
            minor_collections: self.collector.get_minor_collections(),

            // Average collect times in milliseconds.
            major_collect_avg_time: self.collector.get_major_collect_avg_time(),
            minor_collect_avg_time: self.collector.get_minor_collect_avg_time(),

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
