use super::mutator::{MutatorRunner, MutatorScope};
use super::tracer::Tracer;
use super::allocate::Allocate;
use std::sync::Arc;

pub struct Gc<A: Allocate> {
    arena: Arc<A::Arena>,
    tracer: Tracer<A>,
}

impl<A: Allocate> Gc<A> {
    pub fn new() -> Self {
        Self {
            arena: Arc::new(A::new_arena()),
            tracer: Tracer::<A>::new(),
        }
    }

    pub fn mutate<T: MutatorRunner>(&self, runner: &mut T) {
        let mut scope = self.create_scope();
        let root = runner.get_root();

        T::run(root, &mut scope);
    }

    fn create_scope(&self) -> MutatorScope<A> {
        let allocator = A::new_allocator(&self.arena);
        let tracer_handle = self.tracer.new_handle();

        MutatorScope::new(allocator, tracer_handle)
    }
}
