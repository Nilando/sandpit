use super::trace::Trace;
use super::tracer::Tracer;
use std::ptr::NonNull;

unsafe impl Send for TraceJob {}
unsafe impl Sync for TraceJob {}

pub struct TraceJob {
    ptr: NonNull<()>,
    dyn_trace: fn(NonNull<()>, &mut Tracer),
}

impl TraceJob {
    pub fn new<T: Trace>(ptr: NonNull<T>) -> Self {
        Self {
            ptr: ptr.cast(),
            dyn_trace: T::dyn_trace,
        }
    }

    pub fn trace(&self, tracer: &mut Tracer) {
        (self.dyn_trace)(self.ptr, tracer);
    }
}
