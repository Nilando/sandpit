use super::allocate::{Allocate, GenerationalArena};
use super::trace::Trace;
use super::trace_metrics::TraceMetrics;
use super::trace_packet::{TracePacket, TRACE_PACKET_SIZE};
use std::ptr::NonNull;
use std::sync::{Arc, Mutex};

pub trait Tracer {
    fn send_unscanned<T: Trace>(&mut self, ptr: NonNull<T>);
}

impl<A: Allocate> Tracer for TracerWorker<A> {
    fn send_unscanned<T: Trace>(&mut self, ptr: NonNull<T>) {
        if A::get_mark(ptr) == self.mark {
            return;
        }

        A::set_mark(ptr, self.mark);

        if self.new_packet.is_some() {
            if self.new_packet.as_ref().unwrap().is_full() {
                let packet = self.new_packet.take().unwrap();
                self.send_packet(packet)
            } else {
                self.new_packet
                    .as_mut()
                    .unwrap()
                    .push(Some((ptr.cast(), T::dyn_trace)));
                return;
            }
        }

        let mut packet = TracePacket::new();
        packet.push(Some((ptr.cast(), T::dyn_trace)));
        self.new_packet = Some(packet);
    }
}

pub struct TracerWorker<A: Allocate> {
    unscanned: Arc<Mutex<Vec<TracePacket<TracerWorker<A>>>>>,
    mark: <<A as Allocate>::Arena as GenerationalArena>::Mark,
    new_packet: Option<TracePacket<TracerWorker<A>>>,
    metrics: TraceMetrics,
}

unsafe impl<T: Allocate> Send for TracerWorker<T> {}
unsafe impl<T: Allocate> Sync for TracerWorker<T> {}

impl<A: Allocate> TracerWorker<A> {
    pub fn new(
        unscanned: Arc<Mutex<Vec<TracePacket<TracerWorker<A>>>>>,
        mark: <<A as Allocate>::Arena as GenerationalArena>::Mark,
    ) -> Self {
        Self {
            unscanned,
            mark,
            new_packet: None,
            metrics: TraceMetrics::new(),
        }
    }

    pub fn init<T: Trace>(&mut self, root: &T) {
        root.trace(self);
    }

    pub fn trace(&mut self) {
        loop {
            let packet = if self.new_packet.is_some() {
                self.new_packet.take()
            } else {
                self.unscanned.lock().unwrap().pop()
            };

            match packet {
                Some(packet) => self.trace_packet(packet),
                None => break,
            }
        }
    }

    pub fn get_metrics(&self) -> TraceMetrics {
        self.metrics
    }

    fn trace_packet(&mut self, mut packet: TracePacket<TracerWorker<A>>) {
        for _ in 0..TRACE_PACKET_SIZE {
            match packet.pop() {
                Some((ptr, trace_fn)) => {
                    // TODO: COMPILE THIS CONDITIONALLY
                    self.metrics.objects_marked += 1;
                    trace_fn(ptr, self)
                }
                None => break,
            }
        }
    }

    fn send_packet(&mut self, packet: TracePacket<TracerWorker<A>>) {
        self.unscanned.lock().unwrap().push(packet);

        // if we have an available worker, that is waiting, give it to them
    }
}
