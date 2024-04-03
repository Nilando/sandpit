use super::allocate::Allocate;
use super::trace::Trace;
use super::tracer::TracerWorker;
use super::tracer_controller::{TracePacket, TracerController, UnscannedPtr};
use std::sync::Arc;

pub struct TracerHandle<A: Allocate> {
    controller: Arc<TracerController<A>>,
    work_packet: TracePacket<TracerWorker<A>>,
}

// TODO: impl drop to send work packet to tracercontroller

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

    pub fn do_work(&self) {
        todo!()
        /*
        if let Some((ptr, trace_func)) = self.work_packet[0] {
            trace_func(ptr, &self.tracer.as_ref());
        }
        */
    }
}
