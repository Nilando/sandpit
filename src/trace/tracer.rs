use super::marker::Marker;
use super::trace::Trace;
use super::trace_job::TraceJob;
use super::tracer_controller::TracerController;
use std::ptr::NonNull;
use std::sync::Arc;

pub trait Tracer {
    fn trace<T: Trace>(&mut self, ptr: NonNull<T>);
    //fn mark<T: Trace>(&mut self, ptr: NonNull<T>);
}

pub struct TraceWorker<M: Marker> {
    controller: Arc<TracerController<M>>,
    marker: Arc<M>,
    work: Vec<TraceJob<M>>,
}

impl<M: Marker> Tracer for TraceWorker<M> {
    fn trace<T: Trace>(&mut self, ptr: NonNull<T>) {
        if !self.marker.set_mark(ptr) {
            return;
        }

        unsafe {
            if !ptr.as_ref().needs_trace() {
                return;
            }
        }

        self.work.push(TraceJob::new(ptr));
    }
}

impl<M: Marker> TraceWorker<M> {
    pub fn new(controller: Arc<TracerController<M>>, marker: Arc<M>) -> Self {
        Self {
            controller,
            marker,
            work: vec![],
        }
    }

    pub fn flush_work(&mut self) {
        self.controller.send_work(self.work.clone());
    }

    pub fn trace_loop(&mut self) {
        loop {
            if self.work.is_empty() {
                match self.controller.recv_work() {
                    Some(work) => self.work = work,
                    None => break,
                }
            }

            self.do_work();
            self.share_work();
        }

        debug_assert_eq!(self.work.len(), 0);
    }

    fn do_work(&mut self) {
        for _ in 0..self.controller.trace_chunk_size {
            match self.work.pop() {
                Some(job) => job.trace(self),
                None => break,
            }
        }
    }

    fn share_work(&mut self) {
        if self.work.len() < self.controller.trace_share_min || self.controller.has_work() {
            return;
        }

        let mut share_work = vec![];
        for _ in 0..(self.work.len() as f32 * self.controller.trace_share_ratio).floor() as usize {
            let job = self.work.pop().unwrap();
            share_work.push(job);
        }

        self.controller.send_work(share_work);
    }
}
