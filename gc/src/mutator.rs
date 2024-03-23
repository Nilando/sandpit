use super::tracer_handle::TracerHandle;
use super::allocate::Allocate;
use super::gc_ptr::GcPtr;

pub trait MutatorRunner  {
    fn run<'a, T: Mutator>(&mut self, scope: &'a T);
    // we want mutator runner to also be able to pass in and out the roots
}

pub trait Mutator {
    fn alloc<T>(&mut self) -> GcPtr<T>;
    // fn alloc_sized
    // fn alloc_grow
    // fn alloc_shrink
}

pub struct MutatorScope<A: Allocate> {
    allocator: A,
    tracer_handle: TracerHandle<A>
}

impl<A: Allocate> MutatorScope<A> {
    pub fn new(allocator: A, tracer_handle: TracerHandle<A>) -> Self {
        Self { allocator, tracer_handle }
    }
}

impl<A: Allocate> Mutator for MutatorScope<A> {
    fn alloc<T>(&mut self) -> GcPtr<T> {
        todo!()
    }
}

// A mutator scope should be able to allocate
// A mutator scope should be able to add object to an unscanned object
// A needs a scoped lifetime, which can create refs of that lifetime 
//
// using the handle the mutator can send unscanned objects to the tracer
// Using the allocator the 
