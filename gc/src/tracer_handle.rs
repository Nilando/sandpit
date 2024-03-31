use super::allocate::Allocate;
use super::tracer::Tracer;
use std::sync::Arc;

pub struct TracerHandle<A: Allocate> {
    tracer: Arc<Tracer<A>>
    // work_packet for building
}

impl<A: Allocate> TracerHandle<A> {
    pub fn new(tracer: Arc<Tracer<A>>) -> Self {
        Self { 
            tracer
        }
    }
}
