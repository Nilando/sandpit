use super::marker::Marker;
use super::trace::Trace;
use super::trace_packet::{TracePacket};
use super::tracer_controller::TracerController;
use std::ptr::NonNull;
use std::sync::Arc;

pub trait Tracer {
    fn trace<T: Trace>(&mut self, ptr: NonNull<T>);
}

pub struct TraceWorker<M: Marker> {
    controller: Arc<TracerController<M>>,
    marker: M,
    tracing_packet: TracePacket<M>,
    new_packet: TracePacket<M>,
}

unsafe impl<M: Marker> Send for TraceWorker<M> {}
unsafe impl<M: Marker> Sync for TraceWorker<M> {}

impl<M: Marker> Tracer for TraceWorker<M> {
    fn trace<T: Trace>(&mut self, ptr: NonNull<T>) {
        if !self.marker.needs_trace(ptr) {
            return;
        }

        if self.new_packet.is_full() {
            self.send_packet();
        }

        self.new_packet.push(ptr);
    }
}

impl<M: Marker> TraceWorker<M> {
    pub fn new(controller: Arc<TracerController<M>>, marker: M) -> Self {
        Self {
            controller,
            marker,
            new_packet: TracePacket::new(),
            tracing_packet: TracePacket::new(),
        }
    }

    pub fn trace_obj<T: Trace>(&mut self, obj: &T) {
        obj.trace(self);
    }

    pub fn trace_loop(&mut self) {
        loop {
            if self.tracing_packet.is_empty() {
                if !self.new_packet.is_empty() {
                    std::mem::swap(&mut self.tracing_packet, &mut self.new_packet);
                } else {
                    if let Some(new_tracing_packet) = self.controller.pop_packet() {
                        self.tracing_packet = new_tracing_packet;
                    }

                    if self.tracing_packet.is_empty() {
                        break;
                    }
                }
            }

            self.trace_packet();
        }
    }

    fn trace_packet(&mut self) {
        loop {
            match self.tracing_packet.pop() {
                Some(job) => job.trace(self),
                None => break,
            }
        }
    }

    fn send_packet(&mut self) {
        self.controller.push_packet(self.new_packet.clone());
        self.new_packet.drain();
    }
}
