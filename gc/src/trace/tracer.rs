use super::trace::Trace;
use super::tracer_controller::TracerController;
use super::trace_packet::{TracePacket, TRACE_PACKET_SIZE};
use std::ptr::NonNull;
use crate::allocator::{Allocate, GenerationalArena};

pub struct Tracer<'a> {
    controller: &'a TracerController,
    tracing_packet: TracePacket,
    new_packet: TracePacket,
}

unsafe impl<'a> Send for Tracer<'a> {}
unsafe impl<'a> Sync for Tracer<'a> {}

impl<'a> Tracer<'a> {
    pub fn new(controller: &'a TracerController) -> Self {
        Self {
            controller,
            new_packet: TracePacket::new(),
            tracing_packet: TracePacket::new(),
        }
    }

    pub fn send_unscanned<T: Trace>(&mut self, ptr: NonNull<T>) {
        if !T::needs_trace() { return }

        if self.new_packet.is_full() {
            self.send_packet();
        }

        self.new_packet.push(Some((ptr.cast(), T::dyn_trace)));
    }

    pub fn init<T: Trace>(&mut self, root: &T) {
        root.trace(self);
    }

    pub fn trace<A: Allocate>(&mut self, mark: <<A as Allocate>::Arena as GenerationalArena>::Mark) {
        loop {
            if self.tracing_packet.is_empty() {
                if !self.new_packet.is_empty(){
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

            self.trace_packet::<A>(mark);
        }
    }

    fn trace_packet<A: Allocate>(&mut self, mark: <<A as Allocate>::Arena as GenerationalArena>::Mark) {
        for _ in 0..TRACE_PACKET_SIZE {
            match self.tracing_packet.pop() {
                Some((ptr, trace_fn)) => {
                    if mark != A::get_mark(ptr) {
                        A::set_mark(ptr, mark);

                        trace_fn(ptr, self)
                    }
                }
                None => break,
            }
        }
    }

    fn send_packet(&mut self) {
        self.controller.push_packet(self.new_packet.clone());
        self.new_packet.drain();
    }
}
