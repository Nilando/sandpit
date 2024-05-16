use super::marker::Marker;
use super::trace::Trace;
use super::tracer::TraceWorker;
use std::ptr::NonNull;

pub const TRACE_PACKET_SIZE: usize = 64;

pub struct TraceJob<M: Marker> {
    ptr: NonNull<()>,
    dyn_trace: fn(NonNull<()>, &mut TraceWorker<M>),
}

impl<M: Marker> TraceJob<M> {
    const fn new_virtual() -> Self {
        Self {
            ptr: NonNull::<()>::dangling(),
            dyn_trace: TraceJob::<M>::virtual_trace,
        }
    }

    pub fn new<T: Trace>(ptr: NonNull<T>) -> Self {
        Self {
            ptr: ptr.cast(),
            dyn_trace: T::dyn_trace,
        }
    }

    fn virtual_trace(_: NonNull<()>, _: &mut TraceWorker<M>) {
        unreachable!();
    }

    pub fn trace(&self, tracer: &mut TraceWorker<M>) {
        (self.dyn_trace)(self.ptr, tracer);
    }
}

pub struct TracePacket<M: Marker> {
    jobs: [TraceJob<M>; TRACE_PACKET_SIZE],
    len: usize,
}

impl<M: Marker> TracePacket<M> {
    pub fn new() -> Self {
        Self {
            jobs: [const { TraceJob::new_virtual() }; TRACE_PACKET_SIZE],
            len: 0,
        }
    }

    pub fn pop(&mut self) -> Option<TraceJob<M>> {
        if self.len == 0 {
            return None;
        }

        self.len -= 1;

        Some(self.jobs[self.len].clone())
    }

    pub fn push<T: Trace>(&mut self, ptr: NonNull<T>) {
        self.jobs[self.len] = TraceJob::new(ptr);
        self.len += 1;
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_full(&self) -> bool {
        self.len == TRACE_PACKET_SIZE
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn drain(&mut self) {
        self.len = 0;
    }
}

impl<M: Marker> Clone for TracePacket<M> {
    fn clone(&self) -> Self {
        Self {
            len: self.len,
            jobs: self.jobs.clone(),
        }
    }
}

impl<M: Marker> Clone for TraceJob<M> {
    fn clone(&self) -> Self {
        Self {
            ptr: self.ptr,
            dyn_trace: self.dyn_trace,
        }
    }
}
