use super::collector::{Collect, Collector};
use super::config::GcConfig;
use super::metrics::GcMetrics;
use super::monitor::Monitor;
use super::mutator::Mutator;
use super::trace::Trace;

use higher_kinded_types::ForLt;
use std::sync::Arc;

/// A concurrently garbage collected arena with a single root type.
///
/// See the [module-level documentation](./index.html) for more details.
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

unsafe impl<R: ForLt + Send + 'static> Send for Arena<R> 
where
    for<'a> <R as ForLt>::Of<'a>: Trace
{}
unsafe impl<R: ForLt + Sync + 'static> Sync for Arena<R>
where
    for<'a> <R as ForLt>::Of<'a>: Trace
{}

impl<R: ForLt + 'static> Arena<R>
where
    for<'a> <R as ForLt>::Of<'a>: Trace,
{
    /// Creates a new Arena via a callback which provides a mutator for the
    /// ability of allocating the root of the arena.
    /// 
    /// # Examples
    ///
    /// ```
    /// use sandpit::{Arena, Root, gc::Gc};
    ///
    /// let arena: Arena<Root![Gc<'_, usize>]> = Arena::new(|mu| {
    ///     Gc::new(mu, 42)
    /// });
    ///
    /// ```
    ///
    /// The Root type of an Arena must be a "Higher Kinded Type" (HTK), meaning it is a
    /// generic which itself holds a generic lifetime. This is so because the
    /// Root needs to simultaneously have a lifetime that is of the Arena it is
    /// contained in, and also so that it can be branded with the lifetime of a
    /// mutation context. In order to ease the creation of HKT Root there is the 
    /// [`crate::Root!`] macro, which can take a type and convert it into an HTK.
    ///
    pub fn new<F>(f: F) -> Self
    where
        F: for<'gc> FnOnce(&'gc Mutator<'gc>) -> R::Of<'gc>,
    {
        let config = GcConfig::default();

        Self::new_with_config(config, f)
    }

    // eventually it would be cool to allow the user to pass in their own config
    fn new_with_config<F>(config: GcConfig, f: F) -> Self
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

    /// Provides a [`crate::mutator::Mutator`] and the arena's root within a 
    /// mutation context which allows for the allocation and mutation of values 
    /// within the arena. 
    ///
    ///
    /// # Examples
    ///
    /// ```
    /// use sandpit::{Arena, Root, gc::Gc};
    ///
    /// let arena: Arena<Root![Gc<'_, usize>]> = Arena::new(|mu| {
    ///     Gc::new(mu, 42)
    /// });
    ///
    /// arena.mutate(|mu, root| {
    ///     assert!(**root == 42);
    ///
    ///     let new_gc_value = Gc::new(mu, 420);
    /// });
    ///
    /// ```
    ///
    /// ## Mutator 'gc Lifetime
    /// References to values stored in the arena are not allowed to escape the
    /// mutation context. Doing so would be unsafe, as any value that is not
    /// traced may be freed by the GC outside of a mutation context.
    /// In order to ensure references to the arena do not escape the mutation
    /// context they are commonly branded with the `'gc` lifetime.
    ///
    /// ```compile_fail
    /// # use sandpit::{Arena, Root, gc::Gc};
    /// #
    /// # let arena: Arena<Root![Gc<'_, usize>]> = Arena::new(|mu| {
    /// #    Gc::new(mu, 42)
    /// # });
    /// # let x = 123;
    /// # let mut outside_reference: &usize = &x;
    /// arena.mutate(|mu, root| {
    ///     // Gc values cannot escape the mutation context as they are branded
    ///     // with the 'gc lifetime. Doing so would be unsafe, as any untraced
    ///     // values may be freed at the end of the mutation.
    ///     outside_reference = &*root;
    /// });
    ///
    /// ```
    pub fn mutate<F, O>(&self, f: F) -> O
    where
        F: for<'gc> FnOnce(&'gc Mutator<'gc>, &'gc R::Of<'gc>) -> O,
    {
        self.collector.mutate(f)
    }

    /// Synchronously trigger a major collection. A major collection means that
    /// the trace will trace *ALL* objects reachable from the root. This will free
    /// as much memory as possible but may take longer than a minor collection
    ///
    /// The operation will block for any ongoing collections and mutations to 
    /// end at which point the collection will begin. This method will automatically
    /// be called by the monitor.
    pub fn major_collect(&self) {
        self.collector.major_collect();
    }

    /// Synchronously trigger a minor collection. A minor collection will only
    /// trace *new* objects, which are objects that have been allocated since
    /// the last collection. It will likely take less time than a major collection,
    /// but free less memory.
    ///
    /// The operation will block for any ongoing collections and mutations to 
    /// end at which point the collection will begin. This method will automatically
    /// be called by the monitor.
    pub fn minor_collect(&self) {
        self.collector.minor_collect();
    }

    /// Starts a monitor in a separate thread if it is not already started. 
    /// The monitor will automatically and concurrently trigger major and
    /// minor collections when appropriate.
    pub fn start_monitor(&self) {
        self.monitor.clone().start();
    }

    /// Signal for the monitor thread to stop, and block until it does so.
    pub fn stop_monitor(&self) {
        self.monitor.stop();
    }

    /// Returns a copy of the the GcConfig that the arena was created with.
    /// Currently there is no way to update the GcConfig after the arena
    /// has been created.
    /*
    fn get_config(&self) -> GcConfig {
        self.config
    }
    */

    /// Returns a snap short of the GC's current metrics that provide information
    /// about how the GC is running.
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
