use super::config::Config;
use super::metrics::Metrics;
use super::mutator::Mutator;
use super::trace::Trace;
use crate::trace::Collector;

use alloc::boxed::Box;
use alloc::sync::Arc;
use higher_kinded_types::ForLt;

/// A concurrently garbage collected arena with a single root type.
///
/// The root type of an arena must be a Higher Kinded Type(HTK), which
/// can easily be created by using the [`crate::Root!`] macro.
///
/// # Example
/// ```rust
/// use sandpit::{Arena, Root, Gc};
///
/// let arena: Arena<Root![Gc<'_, usize>]> = Arena::new(|mu| {
///     Gc::new(mu, 42)
/// });
/// ```
///
/// This macro is a re-export of [`ForLt!`] from the [`higher_kinded_types`] crate, go
/// check it out for more details on HTK's, and how to use `Root/FotLt`.
pub struct Arena<R: ForLt + 'static>
where
    for<'a> <R as ForLt>::Of<'a>: Trace,
{
    collector: Arc<Collector>,
    root: Box<R::Of<'static>>,
    #[cfg(feature = "multi_threaded")]
    monitor_thread: Option<std::thread::JoinHandle<()>>,
}

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
    /// use sandpit::{Arena, Root, Gc};
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
        let config = Config::default();

        Self::new_with_config(config, f)
    }

    // eventually it would be cool to allow the user to pass in their own config
    pub fn new_with_config<F>(config: Config, f: F) -> Self
    where
        F: for<'gc> FnOnce(&'gc Mutator<'gc>) -> R::Of<'gc>,
    {
        // This is function is extremely sketchy.
        //
        // I have a very fragile understanding of whats going on here, and
        // I truly don't know whether this is safe.

        let collector = Arc::new(Collector::new(config));

        let collector_ref: &'static Collector = unsafe { &*(&*collector as *const Collector) };
        let mutator = Mutator::new(collector_ref);
        let mutator_ref: &'static Mutator<'static> =
            unsafe { &*(&mutator as *const Mutator<'static>) };

        let root = Box::new(f(mutator_ref));

        drop(mutator);

        #[cfg(feature = "multi_threaded")]
        {
            let monitor_thread = collector.clone().spawn_monitor_thread(root.as_ref());

            Self {
                collector,
                root,
                monitor_thread,
            }
        }

        #[cfg(not(feature = "multi_threaded"))]
        {
            Self { collector, root }
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
    /// use sandpit::{Arena, Root, Gc};
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
    /// ## Mutator 'gc Lifetime
    ///
    /// References to values stored in the arena are not allowed to escape the
    /// mutation context. Doing so would be unsafe, as any value that is not
    /// traced may be freed by the GC outside of a mutation context.
    /// In order to ensure references to the arena do not escape the mutation
    /// context they are commonly branded with the `'gc` lifetime.
    ///
    /// ```compile_fail
    /// # use sandpit::{Arena, Root, Gc};
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
    pub fn mutate<F>(&self, f: F)
    where
        F: for<'gc> FnOnce(&'gc Mutator<'gc>, &'gc R::Of<'gc>),
    {
        let mutator = self.new_mutator();
        let root = unsafe { self.scoped_root() };

        f(&mutator, root);

        // TODO:
        // if single threaded mode, collect here on mutation exit
    }

    /// you can view the root but you don't have a mutator, therefore collection
    /// can happen while viewing
    pub fn view<F>(&self, f: F)
    where
        F: for<'gc> FnOnce(&'gc R::Of<'gc>),
    {
        let root = unsafe { self.scoped_root() };

        f(root);
    }

    /// Synchronously trigger a major collection. A major collection means that
    /// the trace will trace *ALL* objects reachable from the root. This will free
    /// as much memory as possible but may take longer than a minor collection
    ///
    /// The operation will block for any ongoing collections and mutations to
    /// end at which point the collection will begin. This method will automatically
    /// be called by the monitor.
    pub fn major_collect(&self) {
        self.collector.major_collect(self.root.as_ref());
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
        self.collector.minor_collect(self.root.as_ref());
    }

    /// Starts a monitor in a separate thread if it is not already started.
    /// The monitor will automatically and concurrently trigger major and
    /// minor collections when appropriate.
    /*
    pub fn start_monitor(&self) {
        self.monitor.clone().start();
    }

    /// Signal for the monitor thread to stop, and block until it does so.
    pub fn stop_monitor(&self) {
        self.monitor.stop();
    }
    */

    /// Returns a copy of the the Config that the arena was created with.
    /// Currently there is no way to update the Config after the arena
    /// has been created.
    /*
    fn get_config(&self) -> Config {
        self.config
    }
    */

    /// Returns a snap short of the GC's current metrics that provide information
    /// about how the GC is running.
    pub fn metrics(&self) -> &Metrics {
        self.collector.metrics()
    }

    // fingers crossed this works! lol
    unsafe fn scoped_root<'gc>(&self) -> &'gc R::Of<'gc> {
        core::mem::transmute::<&R::Of<'static>, &R::Of<'gc>>(self.root.as_ref())
    }

    fn new_mutator<'a>(&'a self) -> Mutator<'a> {
        Mutator::new(&*self.collector)
    }
}

#[cfg(feature = "multi_threaded")]
impl<R: ForLt + 'static> Drop for Arena<R>
where
    for<'a> <R as ForLt>::Of<'a>: Trace,
{
    fn drop(&mut self) {
        // Signal the monitor thread to shut down
        self.collector.shutdown();

        // Join the monitor thread if it exists to ensure it shuts down
        // before the root is dropped
        if let Some(handle) = self.monitor_thread.take() {
            let _ = handle.join();
        }
    }
}
