use super::marker::Marker;
use super::trace::Trace;
use super::tracer::TraceWorker;
use std::ptr::NonNull;

unsafe impl<M: Marker> Send for TraceJob<M> {}
unsafe impl<M: Marker> Sync for TraceJob<M> {}

pub struct TraceJob<M: Marker> {
    ptr: NonNull<()>,
    dyn_trace: fn(NonNull<()>, &mut TraceWorker<M>),
}

impl<M: Marker> TraceJob<M> {
    pub fn new<T: Trace>(ptr: NonNull<T>) -> Self {
        Self {
            ptr: ptr.cast(),
            dyn_trace: T::dyn_trace,
        }
    }

    pub fn trace(&self, tracer: &mut TraceWorker<M>) {
        (self.dyn_trace)(self.ptr, tracer);
    }
}
