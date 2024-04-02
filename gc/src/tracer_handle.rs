use super::allocate::Allocate;
use super::trace::Trace;
use super::tracer::TracerWorker;
use super::tracer_controller::{TracePacket, TracerController, UnscannedPtr};
use std::ptr::NonNull;
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
        let obj_ptr: NonNull<()> = NonNull::from(obj).cast();
        let job: UnscannedPtr<TracerWorker<T>> = (obj_ptr, O::dyn_trace);

        //self.work_packet[0] = Some(job);
    }

    pub fn do_work(&self) {
        /*
        if let Some((ptr, trace_func)) = self.work_packet[0] {
            trace_func(ptr, &self.tracer.as_ref());
        }
        */
    }
}
