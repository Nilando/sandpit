use super::mutator::{Mutator, MutatorScope};
use super::tracer::Tracer;
use super::trace::Trace;
use super::allocate::{Allocate, GenerationalArena};
use std::sync::Arc;
use super::gc_ptr::GcPtr;

pub struct Gc<A: Allocate, Root> {
    arena: Arc<A::Arena>,
    tracer: Arc<Tracer<A>>,
    root: GcPtr<Root>,
}

impl<A: Allocate, T: Trace> Gc<A, T> {
    pub fn build(callback: fn(&mut MutatorScope<A>) -> GcPtr<T>) -> Self {
        let arena = A::Arena::new();
        let tracer = Tracer::<A>::new();
        let mut scope = MutatorScope::new(&arena, &tracer);
        let root = callback(&mut scope);

        Self {
            arena: Arc::new(arena),
            tracer: Arc::new(tracer),
            root
        }
    }

    pub fn mutate(&mut self, callback: fn(&GcPtr<T>, &mut MutatorScope<A>)) {
        let mut scope = MutatorScope::new(self.arena.as_ref(), self.tracer.as_ref());

        callback(&self.root, &mut scope);
    }

    pub fn collect(&mut self) {
        self.tracer.full_collection();
    }
}
