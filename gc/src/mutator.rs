use super::tracer_handle::TracerHandle;
use super::allocate::Allocate;
use super::gc_ptr::GcPtr;
use super::trace::Trace;

pub trait MutatorRunner  {
    type Root: Trace;

    fn get_root(&mut self) -> &mut Self::Root;
    fn run<'a, T: Mutator>(root: &mut Self::Root, scope: &'a T);
}

pub trait Mutator {
    fn alloc<T: Trace>(&mut self, obj: T) -> GcPtr<T>;
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
    fn alloc<T: Trace>(&mut self, obj: T) -> GcPtr<T> {
        todo!()
    }
}

// A mutator scope should be able to allocate
// A mutator scope should be able to add object to an unscanned object
// A needs a scoped lifetime, which can create refs of that lifetime 
//
// using the handle the mutator can send unscanned objects to the tracer
// Using the allocator the 
