use super::allocator::{Allocate, GenerationalArena, Marker};
use super::error::GcError;
use super::gc::Gc;
use super::trace::{Trace, TraceJob, TraceMarker, TracerController};

use std::alloc::Layout;
use std::cell::RefCell;
use std::mem::{align_of, size_of};
use std::ptr::write;
use std::sync::RwLockReadGuard;

/// An interface for the mutator type which allows for interaction with the
/// Gc inside a `gc.mutate(...)` context.
pub trait Mutator {
    // several ways to obtain Gcs
    fn alloc<T: Trace>(&self, obj: T) -> Result<Gc<T>, GcError>;
    fn alloc_array<T: Trace>(&self, size: usize) -> Result<Gc<T>, GcError>;
    fn new_null<T: Trace>(&self) -> Gc<T>;

    // Signal that GC is ready to free memory and the current mutation
    // callback should be exited.
    fn yield_requested(&self) -> bool;

    // Useful for implementing write barriers.
    fn retrace<T: Trace>(&self, ptr: Gc<T>);
    fn is_marked<T: Trace>(&self, ptr: Gc<T>) -> bool;
}

pub struct MutatorScope<'scope, A: Allocate> {
    allocator: A,
    tracer_controller: &'scope TracerController<TraceMarker<A>>,
    rescan: RefCell<Vec<TraceJob<TraceMarker<A>>>>,
    _lock: RwLockReadGuard<'scope, ()>,
}

impl<'scope, A: Allocate> MutatorScope<'scope, A> {
    pub fn new(
        arena: &A::Arena,
        tracer_controller: &'scope TracerController<TraceMarker<A>>,
        _lock: RwLockReadGuard<'scope, ()>,
    ) -> Self {
        let allocator = A::new(arena);

        Self {
            allocator,
            tracer_controller,
            rescan: RefCell::new(vec![]),
            _lock,
        }
    }
}

impl<'scope, A: Allocate> Drop for MutatorScope<'scope, A> {
    fn drop(&mut self) {
        let work = self.rescan.take();
        self.tracer_controller.send_work(work);
    }
}

impl<'scope, A: Allocate> Mutator for MutatorScope<'scope, A> {
    fn yield_requested(&self) -> bool {
        self.tracer_controller.yield_flag()
    }

    fn alloc<T: Trace>(&self, obj: T) -> Result<Gc<T>, GcError> {
        if self.tracer_controller.is_alloc_lock() {
            drop(self.tracer_controller.get_alloc_lock());
        }

        const {
            assert!(
                !std::mem::needs_drop::<T>(),
                "A type must not need dropping to be allocated in a GcArena"
            )
        };

        let layout = Layout::new::<T>();
        match self.allocator.alloc(layout) {
            Ok(ptr) => {
                unsafe { write(ptr.as_ptr().cast(), obj) }

                Ok(Gc::new(ptr.cast()))
            }
            Err(()) => Err(GcError::Oom),
        }
    }

    fn alloc_array<T: Trace>(&self, size: usize) -> Result<Gc<T>, GcError> {
        if self.tracer_controller.is_alloc_lock() {
            drop(self.tracer_controller.get_alloc_lock());
        }

        const {
            assert!(
                !std::mem::needs_drop::<T>(),
                "A type must not need dropping to be allocated in a GcArena"
            )
        };

        let layout = Layout::from_size_align(size_of::<T>() * size, align_of::<T>()).unwrap();
        match self.allocator.alloc(layout) {
            Ok(ptr) => {
                let byte_ptr = ptr.as_ptr();

                for i in 0..layout.size() {
                    unsafe {
                        *byte_ptr.add(i) = 0;
                    }
                }

                Ok(Gc::new(ptr.cast()))
            }
            Err(()) => Err(GcError::Oom),
        }
    }

    fn new_null<T: Trace>(&self) -> Gc<T> {
        Gc::null()
    }

    fn retrace<T: Trace>(&self, gc_ptr: Gc<T>) {
        if gc_ptr.is_null() {
            return;
        }

        let ptr = gc_ptr.as_nonnull();

        let new = <<A as Allocate>::Arena as GenerationalArena>::Mark::new();
        A::set_mark(ptr, new);

        if !T::is_leaf() {
            self.rescan.borrow_mut().push(TraceJob::new(ptr));
        }

        if self.rescan.borrow().len() >= self.tracer_controller.mutator_share_min {
            let work = self.rescan.take();
            self.tracer_controller.send_work(work);
        }
    }

    fn is_marked<T: Trace>(&self, ptr: Gc<T>) -> bool {
        !ptr.is_null() && self.allocator.is_old(ptr.as_nonnull())
    }
}
