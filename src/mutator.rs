use super::allocator::Allocator;
use super::gc::Gc;
use super::pointee::Thin;
use super::header::{GcMark, GcHeader, SizedHeader, SliceHeader};
use super::trace::{Trace, TraceJob, TracerController};
use std::alloc::Layout;
use std::cell::RefCell;
use std::ptr::{write, NonNull};
use std::sync::RwLockReadGuard;

/// The mutator allows allocation into the Gc arena, as well as the
/// mutating of existing Gc pointer types. 
///
/// A mutator is acquired through a the mutation callback on [`crate::arena::Arena::mutate`].
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
    _lock: RwLockReadGuard<'gc, ()>,
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
        allocator: Allocator,
        tracer_controller: &'gc TracerController,
        _lock: RwLockReadGuard<'gc, ()>,
    ) -> Self {
        let mark = tracer_controller.prev_mark();

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
    pub fn alloc<T: Trace>(&self, value: T) -> Gc<'gc, T> {
        let header_layout = Layout::new::<SizedHeader<T>>();
        let val_layout = Layout::new::<T>();
        let (alloc_layout_not_padded, val_offset) = header_layout
            .extend(val_layout)
            .expect("failed GC alloc: invalid layout");
        let alloc_layout = alloc_layout_not_padded.pad_to_align();

        unsafe {
            let ptr = self.allocator.alloc(alloc_layout).expect("failed GC alloc");
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

    /// Alloc a gc array with specified length with each index set to value 
    ///
    /// Due to the reference restraints of Gc<T>, this is only really
    /// useful if T has some form of interior mutability, for example,
    /// Cell<usize>
    ///
    /// Also
    /// ```
    /// // let value = GcNullMut::new_null();
    /// // creates an array of null pointers which may be updated to
    /// // point somewhere else
    /// //let gc_array = mutator.alloc_array(value, len);
    ///
    /// ```
    ///
    pub fn alloc_array<T: Trace + Clone>(&'gc self, value: T, len: usize) -> Gc<[T]> {
        self.alloc_array_from_fn(len, |_| value.clone())
    }

    pub fn alloc_array_from_slice<T: Trace + Clone>(&'gc self, slice: &[T]) -> Gc<[T]> {
        self.alloc_array_from_fn(slice.len(), |idx| {
            slice[idx].clone()
        })
    }

    pub fn alloc_array_from_fn<T, F>(&'gc self, len: usize, mut cb: F) -> Gc<[T]> 
    where
        T: Trace,
        F: FnMut(usize) -> T
    {
        let header_layout = Layout::new::<SliceHeader<T>>();
        let slice_layout = Layout::array::<T>(len).expect("bad GC alloc: invalid layout");
        let (alloc_layout_not_padded, slice_offset) = header_layout
            .extend(slice_layout)
            .expect("bad GC alloc: invalid layout");
        let alloc_layout = alloc_layout_not_padded.pad_to_align();

        unsafe {
            let ptr = self.allocator.alloc(alloc_layout).expect("failed GC alloc");
            let header_ptr = ptr.cast();
            let slice_ptr: *mut T = ptr.add(slice_offset).cast();

            for i in 0..len {
                let item = cb(i);
                write(slice_ptr.add(i), item);
            }

            let slice: *const [T] = std::ptr::slice_from_raw_parts(slice_ptr, len);
            write(header_ptr, SliceHeader::<T>::new(self.mark, len));

            Gc::from_ptr(slice)
        }
    }

    /// This fn will return true when a trace is near completion.
    /// The mutation callback should be exited if yield_requested returns true.
    pub fn gc_yield(&self) -> bool {
        if self.tracer_controller.yield_flag() {
            return true;
        } else {
            let _lock = self.tracer_controller.get_time_slice_lock();
            self.tracer_controller.yield_flag()
        }
    }

    pub(crate) fn retrace<T: Trace + ?Sized>(&self, gc_ptr: Gc<'gc, T>) {
        let ptr: NonNull<Thin<T>> = gc_ptr.as_thin();
        let trace_job = TraceJob::new(ptr);

        self.rescan.borrow_mut().push(trace_job);

        if self.rescan.borrow().len() >= self.tracer_controller.mutator_share_min {
            let work = self.rescan.take();
            self.tracer_controller.send_work(work);
        }
    }

    pub(crate) fn is_marked<T: Trace + ?Sized>(&self, gc_ptr: Gc<'gc, T>) -> bool {
        gc_ptr.get_header().get_mark() == self.tracer_controller.get_current_mark()
    }
}
