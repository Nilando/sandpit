use super::marker::Marker;
use super::trace::Trace;
use super::trace_job::TraceJob;
use super::tracer_controller::TracerController;
use std::ptr::NonNull;
use std::sync::Arc;

const MIN_SHARE_WORK: usize = 1_000;
const WORK_CHUNK_SIZE: usize = 10_000;
//const SHARE_RATIO: f64 = 0.5;

pub trait Tracer {
    fn trace<T: Trace>(&mut self, ptr: NonNull<T>);
}

pub struct TraceWorker<M: Marker> {
    controller: Arc<TracerController<M>>,
    marker: Arc<M>,
    work: Vec<TraceJob<M>>,
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

    fn do_work(&mut self) {
        for _ in 0..WORK_CHUNK_SIZE {
            match self.work.pop() {
                Some(job) => job.trace(self),
                None => break,
            }
        }
    }

    fn share_work(&mut self) {
        if self.work.len() < MIN_SHARE_WORK || self.controller.has_work() {
            return;
        }

        let mut share_work = vec![];
        for _ in 0..(self.work.len() / 2) {
            let job = self.work.pop().unwrap();
            share_work.push(job);
        }

        self.controller.send_work(share_work);
    }

    pub fn trace_loop(&mut self) {
        loop {
            if self.work.is_empty() {
                // TODO:
                // self.controller.recv_work();
                //
                // if self.controller.is_trace_complete() {
                //   break;
                // }
                //
                self.controller.start_waiting();
                if self.controller.is_trace_completed() {
                    self.controller.stop_waiting();
                    break;
                }

                loop {
                    match self.controller.recv_work() {
                        Some(work) => {
                            self.work = work;
                            self.controller.stop_waiting();
                            self.controller.incr_recv();
                            break;
                        }
                        None => {
                            if self.controller.is_trace_completed() {
                                self.controller.stop_waiting();
                                break;
                            }
                        }
                    }
                }
            }

            self.do_work();
            self.share_work();
        }

        debug_assert_eq!(self.work.len(), 0);
        debug_assert_eq!(self.controller.sent(), self.controller.received());
        debug_assert_eq!(self.controller.has_work(), false);
        debug_assert_eq!(self.controller.is_trace_completed(), true);
        //debug_assert_eq!(self.controller.mutators_stopped(), true); TODO: why isn't this true?
    }
}
