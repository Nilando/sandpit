use super::marker::Marker;
use super::trace::Trace;
use super::trace_packet::TracePacket;
use super::tracer_controller::TracerController;
use std::ptr::NonNull;
use std::sync::Arc;

pub trait Tracer {
    fn trace<T: Trace>(&mut self, ptr: NonNull<T>);
}

pub struct TraceWorker<M: Marker> {
    controller: Arc<TracerController<M>>,
    marker: Arc<M>,
    switch: bool,
    p1: TracePacket<M>,
    p2: TracePacket<M>,
}

unsafe impl<M: Marker> Send for TraceWorker<M> {}
unsafe impl<M: Marker> Sync for TraceWorker<M> {}

impl<M: Marker> Tracer for TraceWorker<M> {
    fn trace<T: Trace>(&mut self, ptr: NonNull<T>) {
        if !self.marker.set_mark(ptr) {
            return;
        }

        if !T::needs_trace() {
            return;
        }

        if self.next_packet().is_full() {
            self.send_packet();
        }

        self.push_job(ptr);
    }
}

impl<M: Marker> TraceWorker<M> {
    pub fn new(controller: Arc<TracerController<M>>, marker: Arc<M>) -> Self {
        Self {
            controller,
            marker,
            switch: false,
            p1: TracePacket::new(),
            p2: TracePacket::new(),
        }
    }

    pub fn trace_obj<T: Trace>(&mut self, obj: &T) {
        obj.trace(self);
    }

    pub fn trace_loop(&mut self) {
        loop {
            if self.current_packet().is_empty() {
                self.switch = !self.switch;
                if self.current_packet().is_empty() {
                    self.get_new_packet();
                }

                if self.current_packet().is_empty() {
                    break;
                }
            }

            self.trace_packet();

            self.switch = !self.switch;
        }
    }

    fn get_new_packet(&mut self) {
        if let Some(new_tracing_packet) = self.controller.pop_packet() {
            if self.switch {
                self.p1 = new_tracing_packet;
            } else {
                self.p2 = new_tracing_packet;
            }
        }
    }

    fn current_packet(&self) -> &TracePacket<M> {
        if self.switch {
            &self.p1
        } else {
            &self.p2
        }
    }

    fn next_packet(&mut self) -> &TracePacket<M> {
        if !self.switch {
            &self.p1
        } else {
            &self.p2
        }
    }

    fn trace_packet(&mut self) {
        if self.switch {
            loop {
                match self.p1.pop() {
                    Some(job) => job.trace(self),
                    None => break,
                }
            }
        } else {
            loop {
                match self.p2.pop() {
                    Some(job) => job.trace(self),
                    None => break,
                }
            }
        }
    }

    fn push_job<T: Trace>(&mut self, ptr: NonNull<T>) {
        if !self.switch {
            self.p1.push(ptr);
        } else {
            self.p2.push(ptr);
        }
    }

    fn send_packet(&mut self) {
        if !self.switch {
            self.controller.push_packet(self.p1.clone());
            self.p1.drain();
        } else {
            self.controller.push_packet(self.p2.clone());
            self.p2.drain();
        }
    }
}
