use super::trace::Trace;
use super::tracer::Tracer;
use crate::pointee::Thin;
use core::ptr::NonNull;

unsafe impl Send for TraceJob {}
unsafe impl Sync for TraceJob {}

pub struct TraceJob {
    ptr: NonNull<Thin<()>>,
    dyn_trace: fn(NonNull<Thin<()>>, &mut Tracer),
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
