use super::allocator::{Allocate, GenerationalArena, Marker};
use super::error::GcError;
use super::gc_ptr::GcPtr;
use super::trace::{Trace, TraceJob, TraceMarker, TracerController};

use std::time::SystemTime;
use std::alloc::Layout;
use std::cell::RefCell;
use std::ptr::write;
use std::sync::RwLockReadGuard;

/// An interface for the mutator type which allows for interaction with the
/// Gc inside a `gc.mutate(...)` context.
pub trait Mutator {
    fn alloc<T: Trace>(&self, obj: T) -> Result<GcPtr<T>, GcError>;

    // TODO: remove this method! or make it private somehow.
    fn alloc_layout(&self, layout: Layout) -> Result<GcPtr<()>, GcError>;

    fn write_barrier<A: Trace, B: Trace>(
        &self,
        update: GcPtr<A>,
        new: GcPtr<B>,
        callback: fn(&A) -> &GcPtr<B>,
    );
    fn rescan<T: Trace>(&self, ptr: GcPtr<T>);
    fn yield_requested(&self) -> bool;
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
            // TODO: this could probably be something other than a mutex
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

    fn alloc<T: Trace>(&self, obj: T) -> Result<GcPtr<T>, GcError> {
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

                Ok(GcPtr::new(ptr.cast()))
            }
            Err(_) => todo!(),
        }
    }

    fn alloc_layout(&self, layout: Layout) -> Result<GcPtr<()>, GcError> {
        match self.allocator.alloc(layout) {
            Ok(ptr) => {
                for i in 0..layout.size() {
                    unsafe { write(ptr.as_ptr().add(i), 0) }
                }

                Ok(GcPtr::new(ptr.cast()))
            }
            Err(_) => todo!(),
        }
    }

    fn write_barrier<X: Trace, Y: Trace>(
        &self,
        update_ptr: GcPtr<X>,
        new_ptr: GcPtr<Y>,
        callback: fn(&X) -> &GcPtr<Y>,
    ) {
        if self.tracer_controller.is_write_barrier_locked() {
            self.tracer_controller.get_write_barrier_lock();
        }

        unsafe {
            let ptr = update_ptr.as_nonnull();
            let old_ptr = callback(ptr.as_ref());

            old_ptr.unsafe_set(new_ptr);

            self.rescan(update_ptr);
        }
    }

    fn rescan<T: Trace>(&self, gc_ptr: GcPtr<T>) {
        let ptr = unsafe { gc_ptr.as_nonnull() };

        if !self.allocator.is_old(ptr) {
            return;
        }

        let new = <<A as Allocate>::Arena as GenerationalArena>::Mark::new();
        A::set_mark(ptr, new);

        self.rescan.borrow_mut().push(TraceJob::new(ptr));

        if self.rescan.borrow().len() >= self.tracer_controller.mutator_share_min {
            let work = self.rescan.take();
            self.tracer_controller.send_work(work);
        }
    }
}
