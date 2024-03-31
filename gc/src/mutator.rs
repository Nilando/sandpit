use super::tracer_handle::TracerHandle;
use super::allocate::Allocate;
use super::gc_ptr::GcPtr;
use super::trace::Trace;
use super::tracer::Tracer;
use super::error::GcError;

pub trait Mutator {
    fn alloc<T: Trace>(&mut self, obj: T) -> Result<GcPtr<T>, GcError>;
    //fn alloc_sized(&mut self, len: u32) -> Result<NonNull<u8>, Self::Error>;
    // fn alloc_sized
    // fn alloc_grow
    // fn alloc_shrink
}

pub struct MutatorScope<A: Allocate> {
    allocator: A,
    tracer_handle: TracerHandle
}

impl<A: Allocate> MutatorScope<A> {
    pub fn new(arena: &A::Arena, tracer: &Tracer<A>) -> Self {
        let allocator = A::new(arena);
        let tracer_handle = TracerHandle::new(tracer);

        Self { allocator, tracer_handle }
    }
}

impl<A: Allocate> Mutator for MutatorScope<A> {
    fn alloc<T: Trace>(&mut self, obj: T) -> Result<GcPtr<T>, GcError> {
        match self.allocator.alloc(obj) {
            Ok(ptr) => Ok(GcPtr::new(ptr)),
            Err(_) => todo!()
        }
    }
}
