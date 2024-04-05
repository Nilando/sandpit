use super::allocate::Allocate;
use super::error::GcError;
use super::gc_ptr::GcPtr;
use super::trace::Trace;
use super::tracer_controller::TracerController;
use super::tracer_handle::TracerHandle;
use std::ptr::NonNull;
use std::sync::Arc;

pub trait Mutator {
    fn alloc<T: Trace>(&self, obj: T) -> Result<GcPtr<T>, GcError>;
    fn write_barrier<T: Trace>(&self, obj: NonNull<T>);
    fn yield_requested(&mut self) -> bool;
    // fn alloc_sized(&mut self, len: u32) -> Result<NonNull<u8>, Self::Error>;
    // fn alloc_vec<T: Trace>(len, capacity, T) -> GcVec<T>;
    // fn alloc_grow
    // fn alloc_shrink
}

pub struct MutatorScope<A: Allocate> {
    allocator: A,
    tracer_handle: TracerHandle<A>,
}

impl<A: Allocate> MutatorScope<A> {
    pub fn new(arena: &A::Arena, tracer: Arc<TracerController<A>>) -> Self {
        let allocator = A::new(arena);
        let tracer_handle = TracerHandle::new(tracer);

        Self {
            allocator,
            tracer_handle,
        }
    }
}

impl<A: Allocate> Mutator for MutatorScope<A> {
    fn yield_requested(&mut self) -> bool {
        true
    }

    fn alloc<T: Trace>(&self, obj: T) -> Result<GcPtr<T>, GcError> {
        match self.allocator.alloc(obj) {
            Ok(ptr) => Ok(GcPtr::new(ptr)),
            Err(_) => todo!(),
        }
    }

    fn write_barrier<T: Trace>(&self, obj: NonNull<T>) {
        // get the header
        // check if header is marked
    }
}
