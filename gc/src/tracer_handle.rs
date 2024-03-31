use super::tracer::Tracer;
use super::trace::Trace;
use std::sync::Arc;
use std::ptr::NonNull;

const WORK_PACKET_SIZE: usize = 420;

type Work<T> = (NonNull<()>, fn(NonNull<()>, &T));

pub struct TracerHandle<T: Tracer> {
    tracer: Arc<T>,
    work_packet: [
        Option<Work<T>>;
        WORK_PACKET_SIZE
    ]
}

impl<T: Tracer> TracerHandle<T> {
    pub fn new(tracer: Arc<T>) -> Self {
        Self { 
            tracer,
            work_packet: [None; WORK_PACKET_SIZE]
        }
    }

    pub fn send_to_unscanned<O: Trace>(&mut self, obj: &O) {
        let obj_ptr: NonNull<()> = NonNull::from(obj).cast();
        let job: Work<T> = (obj_ptr, O::dyn_trace);

        self.work_packet[0] = Some(job);
    }

    pub fn do_work(&self) {
        if let Some((ptr, trace_func)) = self.work_packet[0] {
            trace_func(ptr, &self.tracer.as_ref());
        }
    }
}
