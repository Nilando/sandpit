use std::marker::PhantomData;
use super::allocate::Allocate;

pub struct TracerHandle<A: Allocate> {
    _allocator: PhantomData<A>
    //tracer: Tracer<A>,
    // work packet, once full send to tracer
}

impl<A: Allocate> TracerHandle<A> {
    pub fn new() -> Self {
        Self { _allocator: PhantomData::<A> }
    }
}
