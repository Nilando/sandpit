use super::allocate::{Allocate, GenerationalArena};
use super::gc_ptr::GcPtr;
use super::mutator::MutatorScope;
use super::trace::Trace;
use super::tracer_controller::TracerController;
use std::sync::Arc;

pub struct Gc<A: Allocate, Root: Trace> {
    arena: Arc<A::Arena>,
    tracer: Arc<TracerController<A>>,
    root: GcPtr<Root>,
}

impl<A: Allocate, T: Trace> Gc<A, T> {
    pub fn build(callback: fn(&mut MutatorScope<A>) -> GcPtr<T>) -> Self {
        let arena = Arc::new(A::Arena::new());
        let tracer = Arc::new(TracerController::<A>::new());
        let mut scope = MutatorScope::new(arena.as_ref(), tracer.clone());
        let root = callback(&mut scope);

        Self {
            arena,
            tracer,
            root,
        }
    }

    pub fn mutate(&mut self, callback: fn(&GcPtr<T>, &mut MutatorScope<A>)) {
        let mut scope = MutatorScope::new(self.arena.as_ref(), self.tracer.clone());

        callback(&self.root, &mut scope);
    }

    pub fn collect(&mut self) {
        self.tracer.full_collection(self.arena.as_ref(), self.root);
    }
}