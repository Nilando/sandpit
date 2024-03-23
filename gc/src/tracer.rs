use std::marker::PhantomData;
use super::allocate::Allocate;
use super::tracer_handle::TracerHandle;

#[derive(Copy, Clone)]
pub struct Tracer<A: Allocate> {
    _allocator: PhantomData<A>
    // tracer needs A: Allocate so that it can mark gc ptrs correctly
    // and so that it can signal to the allocator when to begin partial or full tracing
    // unscanned objects(work) *should be divided up into work packets
    // yield lock
    // yield flag
    // metrics?
    // threads?
}

impl<A: Allocate> Tracer<A> {
    pub fn new() -> Self {
        Self {
            _allocator: PhantomData::<A>
        }
    }

    pub fn new_handle(&self) -> TracerHandle<A> {
        TracerHandle::new()
    }
}
