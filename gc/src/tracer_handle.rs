use super::allocate::Allocate;
use super::trace::Trace;
use super::trace_packet::TracePacket;
use super::tracer::TracerWorker;
use super::tracer_controller::TracerController;
use std::sync::Arc;

pub struct TracerHandle<A: Allocate> {
    controller: Arc<TracerController<A>>,
    work_packet: TracePacket<TracerWorker<A>>,
}

impl<T: Allocate> TracerHandle<T> {
    pub fn new(controller: Arc<TracerController<T>>) -> Self {
        Self {
            controller,
            work_packet: TracePacket::new(),
        }
    }

    pub fn send_to_unscanned<O: Trace>(&mut self, obj: &O) {
        todo!()
    }
}
