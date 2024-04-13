use super::tracer::Tracer;
use std::ptr::NonNull;

pub const TRACE_PACKET_SIZE: usize = 100;

pub type UnscannedPtr<T> = (NonNull<()>, fn(NonNull<()>, &mut T));

#[derive(Copy, Clone)]
pub struct TracePacket<T> {
    jobs: [Option<UnscannedPtr<T>>; TRACE_PACKET_SIZE],
    len: usize,
}

impl<T: Tracer> TracePacket<T> {
    pub fn new() -> Self {
        Self {
            jobs: [None; TRACE_PACKET_SIZE],
            len: 0,
        }
    }

    pub fn pop(&mut self) -> Option<UnscannedPtr<T>> {
        if self.len == 0 {
            return None;
        }

        self.len -= 1;
        self.jobs[self.len]
    }

    pub fn push(&mut self, job: Option<UnscannedPtr<T>>) {
        self.jobs[self.len] = job;
        self.len += 1;
    }

    pub fn is_full(&self) -> bool {
        self.len == TRACE_PACKET_SIZE
    }
}
