use super::trace::Trace;
use super::tracer::Tracer;
use crate::pointee::Thin;
use core::ptr::NonNull;
use std::hash::Hash;

unsafe impl Send for TraceJob {}
unsafe impl Sync for TraceJob {}

#[derive(Clone)]
pub struct TraceJob {
    ptr: NonNull<Thin<()>>,
    dyn_trace: fn(NonNull<Thin<()>>, &mut Tracer),
}

impl PartialEq for TraceJob {
    fn eq(&self, other: &Self) -> bool {
        self.ptr == other.ptr
    }
}

impl Eq for TraceJob {}

impl Hash for TraceJob {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.ptr.hash(state)
    }
}

impl TraceJob {
    pub fn new<T: Trace + ?Sized>(ptr: NonNull<Thin<T>>) -> Self {
        Self {
            ptr: ptr.cast(),
            dyn_trace: T::dyn_trace,
        }
    }

    pub fn trace(&self, tracer: &mut Tracer) {
        (self.dyn_trace)(self.ptr, tracer);
    }
}
