use super::tracer::Tracer;
use std::ptr::NonNull;

pub const TRACE_PACKET_SIZE: usize = 100;
pub type UnscannedPtr = (NonNull<()>, fn(NonNull<()>, &mut Tracer));

pub struct TracePacket {
    jobs: [Option<UnscannedPtr>; TRACE_PACKET_SIZE],
    len: usize,
}

impl TracePacket {
    pub fn new() -> Self {
        Self {
            jobs: [None; TRACE_PACKET_SIZE],
            len: 0,
        }
    }

    pub fn pop(&mut self) -> Option<UnscannedPtr> {
        if self.len == 0 {
            return None;
        }

        self.len -= 1;
        self.jobs[self.len]
    }

    pub fn push(&mut self, job: Option<UnscannedPtr>) {
        self.jobs[self.len] = job;
        self.len += 1;
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

impl Clone for TracePacket {
    fn clone(&self) -> Self {
        Self {
            len: self.len,
            jobs: self.jobs.clone()
        }
    }
}
