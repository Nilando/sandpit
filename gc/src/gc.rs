use super::mutator::{MutatorRunner, MutatorScope};
use super::tracer::Tracer;
use super::allocate::Allocate;

pub struct Gc<A: Allocate> {
    arena: A::Arena,
    tracer: Tracer<A>,
}

impl<A: Allocate> Gc<A> {
    pub fn new() -> Self {
        Self {
            arena: A::new_arena(),
            tracer: Tracer::<A>::new(),
        }
    }

    pub fn mutate<T: MutatorRunner>(&self, mutator: &mut T) {
        let scope = self.create_scope();

        mutator.run(&scope);
    }

    fn create_scope(&self) -> MutatorScope<A> {
        let allocator = A::new_allocator(&self.arena);
        let tracer_handle = self.tracer.new_handle();

        MutatorScope::new(allocator, tracer_handle)
    }
}
