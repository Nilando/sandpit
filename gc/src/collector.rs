use super::trace::Trace;
use super::mutator::{Mutator, MutatorScope};
use super::allocate::{Allocate, GenerationalArena};
use super::gc_ptr::GcPtr;
use super::tracer_controller::TracerController;
use std::sync::Arc;

// Collect is moved into a separate trait from the GcController, so that the monitor can work with
// a dynamic Collect type without needing to define the associated types of root and mutator
pub trait Collect: 'static {
    fn collect(&self);
    fn eden_collect(&self);
    fn arena_size(&self) -> usize;
}

pub trait GcController: Collect {
    type Root: Trace;
    type Mutator: Mutator;

    fn build(callback: fn(&mut Self::Mutator) -> GcPtr<Self::Root>) -> Self;
    fn mutate(&self, callback: fn(GcPtr<Self::Root>, &mut Self::Mutator));
}

unsafe impl<A: Allocate, T: Trace + Send> Send for Controller<A, T> {}
unsafe impl<A: Allocate, T: Trace + Sync> Sync for Controller<A, T> {}

pub struct Controller<A: Allocate, T: Trace> {
    arena: Arc<A::Arena>,
    tracer: Arc<TracerController<A>>,
    root: GcPtr<T>,
}

impl<A: Allocate, T: Trace> Collect for Controller<A, T> {
    fn collect(&self) {
        self.tracer.full_collection(self.arena.as_ref(), self.root);
    }

    fn eden_collect(&self) {
        self.tracer.eden_collection(self.arena.as_ref(), self.root);
    }

    fn arena_size(&self) -> usize {
        self.arena.get_size()
    }
}

impl<A: Allocate, T: Trace> GcController for Controller<A, T> {
    type Root = T;
    type Mutator = MutatorScope<A>;

    fn build(callback: fn(&mut Self::Mutator) -> GcPtr<T>) -> Self {
        let arena = Arc::new(A::Arena::new());
        let tracer = Arc::new(TracerController::<A>::new());
        let yield_lock = tracer.get_yield_lock();
        let mut scope = Self::Mutator::new(arena.as_ref(), tracer.clone());
        let root = callback(&mut scope);
        drop(yield_lock);
        let gc = Self {
            arena,
            tracer,
            root,
        };

        gc
    }

    fn mutate(&self, callback: fn(GcPtr<Self::Root>, &mut Self::Mutator)) {
        let _yield_lock = self.tracer.get_yield_lock();
        let mut mutator = Self::Mutator::new(self.arena.as_ref(), self.tracer.clone());

        callback(self.root, &mut mutator);
    }
}
