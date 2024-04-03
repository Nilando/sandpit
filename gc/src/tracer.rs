use super::allocate::{Allocate, GenerationalArena};
use super::tracer_controller::{TRACE_PACKET_SIZE, TracePacket, TracerController};
use std::sync::{Arc, Mutex};
use std::ptr::NonNull;
use super::trace::Trace;

pub trait Tracer {
    fn send_unscanned<T: Trace>(&mut self, ptr: NonNull<T>);
}

impl<A: Allocate> Tracer for TracerWorker<A> {
    fn send_unscanned<T: Trace>(&mut self, ptr: NonNull<T>) {
    }
}

pub struct TracerWorker<A: Allocate> {
        unscanned: Arc<Mutex<Vec<TracePacket<TracerWorker<A>>>>>,
        mark: <<A as Allocate>::Arena as GenerationalArena>::Mark,
        new_packet: Option<TracePacket<TracerWorker<A>>>
}

unsafe impl<T: Allocate> Send for TracerWorker<T> {}
unsafe impl<T: Allocate> Sync for TracerWorker<T> {}

impl<A: Allocate> TracerWorker<A> {
    pub fn new(
        unscanned: Arc<Mutex<Vec<TracePacket<TracerWorker<A>>>>>,
        mark: <<A as Allocate>::Arena as GenerationalArena>::Mark,
    ) -> Self {
        Self{ 
            unscanned,
            mark,
            new_packet: None
        }
    }

    pub fn trace_packet(&mut self, packet: TracePacket<TracerWorker<A>>) {
        for i in 0..TRACE_PACKET_SIZE {
            match packet.get(i) {
                Some((ptr, trace_fn)) => {
                    A::set_mark(ptr, self.mark);
                    trace_fn(ptr, self)
                },
                None => break
            }
        }
    }

    pub fn trace(&mut self) {
        loop {
            let packet = self.unscanned.as_ref().lock().unwrap().pop();

            match packet {
                Some(packet) => self.trace_packet(packet),
                None => {
                    match self.new_packet.take() {
                        Some(packet) => self.trace_packet(packet),
                        None => break
                    }
                }
            }
        }
    }
}
