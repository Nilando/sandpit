use crate::heap::Allocator;

use super::gc::Gc;
use super::header::{GcHeader, GcMark, SizedHeader, SliceHeader};
use super::pointee::Thin;
use super::pointee::{sized_alloc_layout, slice_alloc_layout};
use super::trace::{Trace, TraceJob, TracerController};
use super::trace::tracer_controller::YieldLockGuard;
use core::cell::RefCell;
use core::ptr::{write, copy, NonNull};
use alloc::vec::Vec;
use alloc::vec;

/// Allows for allocation and mutation within the GC arena.
///
/// A mutator is acquired through a the mutation callback on [`crate::Arena::mutate`].
///
/// # Example
/// ```rust
/// use sandpit::{Arena, Gc, Root};
///
/// let arena: Arena<Root![Gc<'_, usize>]> = Arena::new(|mu| {
///    Gc::new(mu, 123)
/// });
///
/// // By calling mutate we get access to a mutator and the root!
/// arena.mutate(|mu, root| {
///     // we can use a mutator to allocate into the arena.
///     let a = Gc::new(mu, 456);
///
///     // we can also use the mutator to create write barriers.
///     a.write_barrier(mu, |barrier| {
///         // in this case there isn't anything useful to do with the barrier...
///     });
/// });
/// ```
///
/// # Calling `gc_yield` is Critical!
///
/// In order for the GC to efficiently free memory any long lasting mutation
/// which is allocating memory must periodically call [`Mutator::gc_yield`].
///
/// `gc_yield` fulfills 2 operations
/// - it will block the mutator if tracers are currently struggling to complete a trace.
/// - if gc_yield returns true it signals that memory is ready to be freed and the mutation callbacks must be exited in order to do so.
///
/// While the GC is concurrent, meaning that the tracer may perform a trace
/// while mutation is happening, the GC is unable to actually free
/// any memory while mutation is occuring. This is because the Gc needs to be certain
/// that when it frees memory it has traced all existing references into
/// the arena. Unfortunately it is quite difficult to account for
/// references to GC values which exist on the stack within a mutation context.
///
/// So in order to ensure that all references are traced, the GC requires
/// all mutation contexts to exit, guaranteeing that the only Gc reference to exist(outside of the tracer threads)
/// is the singular arena root.
///
/// The mutator therefore holds a lock, that the tracer threads use
/// to be able to identify if the mutation contexts have all exited at
/// which point the Gc will free memory and then allow for mutation to resume.
pub struct Mutator<'gc> {
    tracer_controller: &'gc TracerController,
    allocator: Allocator,
    rescan: RefCell<Vec<TraceJob>>,
    _lock: YieldLockGuard<'gc>,
    mark: GcMark,
}

impl<'gc> Drop for Mutator<'gc> {
    fn drop(&mut self) {
        let work = self.rescan.take();
        self.tracer_controller.send_work(work);
    }
}

impl<'gc> Mutator<'gc> {
    pub(crate) fn new(
        tracer_controller: &'gc TracerController
    ) -> Self {
        let mark = tracer_controller.prev_mark();
        let allocator = tracer_controller.new_allocator();
        let _lock = tracer_controller.yield_lock();

        Self {
            allocator,
            tracer_controller,
            rescan: RefCell::new(vec![]),
            mark,
            _lock,
        }
    }

    /// The underlying method to [`crate::gc::Gc::new`] which allocates into the arena.
    ///
    /// This may panic if the allocation fails which may be because of
    /// lack of memory or object allocated is too large.
    ///
    /// # Example
    /// ```rust
    /// # use sandpit::{Arena, Gc, Root};
    /// # let arena: Arena<Root![Gc<'_, usize>]> = Arena::new(|mu| {
    /// #    Gc::new(mu, 123)
    /// # });
    /// arena.mutate(|mu, root| {
    ///     let a = mu.alloc(456);
    /// });
    /// ```
    pub fn alloc<T: Trace>(&self, value: T) -> Gc<'gc, T> {
        let (alloc_layout, val_offset) = sized_alloc_layout::<T>();

        unsafe {
            let ptr = self.allocator.alloc(alloc_layout) as *mut u8;
            // SAFETY: the alloc layout was extended to have capacity
            // for the header and object to be written into.

            // Creating the Gc<T> from the obj_ptr is safe, b/c it upholds
            // the Gc invariant that a Gc<T> points to a T with a padded header.
            
            let val_ptr = ptr.add(val_offset).cast();
            let header_ptr = ptr.cast();

            write(val_ptr, value);
            write(header_ptr, SizedHeader::<T>::new(self.mark));

            Gc::from_ptr(val_ptr)
        }
    }

    /// Alloc a `Gc<[T]>` with specified length and with each index set to value.
    ///
    /// # Example
    /// ```rust
    /// # use sandpit::{Arena, Gc, Root};
    /// # let arena: Arena<Root![Gc<'_, usize>]> = Arena::new(|mu| {
    /// #    Gc::new(mu, 123)
    /// # });
    /// arena.mutate(|mu, root| {
    ///     let gc_slice = mu.alloc_array(333, 100);
    ///
    ///     assert!(gc_slice.len() == 100);
    ///
    ///     for i in gc_slice.iter() {
    ///         assert!(*i == 333);
    ///     }
    /// });
    /// ```
    pub fn alloc_array<T: Trace + Copy>(&'gc self, value: T, len: usize) -> Gc<'gc, [T]> {
        let (alloc_layout, slice_offset) = slice_alloc_layout::<T>(len);

        unsafe {
            let ptr = self.allocator.alloc(alloc_layout) as *mut u8;
            let header_ptr = ptr.cast();
            let slice_ptr: *mut T = ptr.add(slice_offset).cast();

            for i in 0..len {
                write(slice_ptr.add(i), value);
            }

            let slice: *const [T] = core::ptr::slice_from_raw_parts(slice_ptr, len);
            write(header_ptr, SliceHeader::<T>::new(self.mark, slice.len()));

            Gc::from_ptr(slice)
        }
    }

    /// Alloc a `Gc<[T]>` by copying an existing slice.
    ///
    /// ```rust
    /// # use sandpit::{Arena, Gc, Root};
    /// # let arena: Arena<Root![Gc<'_, usize>]> = Arena::new(|mu| {
    /// #    Gc::new(mu, 123)
    /// # });
    /// arena.mutate(|mu, root| {
    ///     let data = vec![0, 1, 2, 3, 4, 5];
    ///     let gc_slice = mu.alloc_array_from_slice(&data);
    ///
    ///     for (i, n) in gc_slice.iter().enumerate() {
    ///         assert!(i == *n);
    ///     }
    /// });
    /// ```
    pub fn alloc_array_from_slice<T: Trace + Copy>(&'gc self, slice: &[T]) -> Gc<'gc, [T]> {
        let (alloc_layout, slice_offset) = slice_alloc_layout::<T>(slice.len());

        unsafe {
            let ptr = self.allocator.alloc(alloc_layout) as *mut u8;
            let header_ptr = ptr.cast();
            let slice_ptr: *mut T = ptr.add(slice_offset).cast();

            copy(slice.as_ptr(), slice_ptr, slice.len());

            let slice: *const [T] = core::ptr::slice_from_raw_parts(slice_ptr, slice.len());
            write(header_ptr, SliceHeader::<T>::new(self.mark, slice.len()));

            Gc::from_ptr(slice)
        }
    }

    /// Alloc a `Gc<[T]>` by using a closure that sets the value for each index.
    ///
    /// # Example
    /// ```rust
    /// # use sandpit::{Arena, Gc, Root};
    /// # let arena: Arena<Root![Gc<'_, usize>]> = Arena::new(|mu| {
    /// #    Gc::new(mu, 123)
    /// # });
    /// arena.mutate(|mu, root| {
    ///     let gc_slice = mu.alloc_array_from_fn(100, |idx| idx % 2);
    ///
    ///     for (i, n) in gc_slice.iter().enumerate() {
    ///         assert!((i % 2) == *n);
    ///     }
    /// });
    /// ```
    pub fn alloc_array_from_fn<T, F>(&'gc self, len: usize, mut cb: F) -> Gc<'gc, [T]>
    where
        T: Trace,
        F: FnMut(usize) -> T,
    {
        let (alloc_layout, slice_offset) = slice_alloc_layout::<T>(len);

        unsafe {
            let ptr = self.allocator.alloc(alloc_layout) as *mut u8;
            let header_ptr = ptr.cast();
            let slice_ptr: *mut T = ptr.add(slice_offset).cast();

            for i in 0..len {
                let item = cb(i);
                write(slice_ptr.add(i), item);
            }

            let slice: *const [T] = core::ptr::slice_from_raw_parts(slice_ptr, len);
            write(header_ptr, SliceHeader::<T>::new(self.mark, len));

            Gc::from_ptr(slice)
        }
    }

    /// This fn will return true when a trace is near completion.
    /// The mutation callback should be exited if gc_yield returns true.
    ///
    /// # Example
    /// ```rust
    /// # use sandpit::{Arena, Gc, Root, Mutator};
    /// # let arena: Arena<Root![Gc<'_, usize>]> = Arena::new(|mu| {
    /// #    Gc::new(mu, 123)
    /// # });
    /// # fn vm_execute_loop<'gc>(mu: &'gc Mutator, root: &Gc<'gc, usize>) { Gc::new(mu, 69); }
    /// arena.mutate(|mu, root| {
    ///     loop {
    ///         // the vm_execute_loop would need to return every so often
    ///         vm_execute_loop(mu, root);
    ///
    ///         // if the gc_yield returns true,
    ///         // memory is ready to be freed
    ///         if mu.gc_yield() {
    ///             // exit the mutation
    ///             break;
    ///         }
    ///     }
    /// });
    /// ```
    pub fn gc_yield(&self) -> bool {
        if self.tracer_controller.yield_flag() {
            return true;
        }

        self.tracer_controller.minor_trigger() || self.tracer_controller.major_trigger()
    }

    pub(crate) fn has_marked<T: Trace + ?Sized>(&self, gc_ptr: &Gc<'gc, T>) -> bool {
        gc_ptr.get_header().get_mark() == self.tracer_controller.get_current_mark()
    }

    pub(crate) fn get_mark(&self) -> GcMark {
       self.tracer_controller.get_current_mark()
    }

    pub(crate) fn get_prev_mark(&self) -> GcMark {
       self.tracer_controller.get_current_mark().rotate().rotate()
    }

    pub fn retrace<T: Trace + ?Sized>(&self, obj: &'gc T) {
        let ptr: NonNull<Thin<T>> = NonNull::from(obj).cast(); // safe b/c of implicit Sized bound
        let trace_job = TraceJob::new::<T>(ptr);

        self.rescan.borrow_mut().push(trace_job);

        if self.rescan.borrow().len() >= self.tracer_controller.config.mutator_share_min {
            let work = self.rescan.take();
            self.tracer_controller.send_work(work);
        }
    }
}
