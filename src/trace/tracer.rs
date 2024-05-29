use crossbeam_channel::{Sender, Receiver};
use std::time::Instant;
use super::marker::Marker;
use super::trace::Trace;
use super::trace_packet::TraceJob;
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
    sender: Sender<Vec<TraceJob<M>>>,
    receiver: Receiver<Vec<TraceJob<M>>>,
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
    pub fn new(
        controller: Arc<TracerController<M>>,
        marker: Arc<M>,
        sender: Sender<Vec<TraceJob<M>>>,
        receiver: Receiver<Vec<TraceJob<M>>>,
    ) -> Self {

        Self {
            controller,
            marker,
            work: vec![],
            sender,
            receiver,
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
        if self.work.len() < MIN_SHARE_WORK || !self.sender.is_empty() {
            return
        }

        let mut share_work = vec![];
        for _ in 0..(self.work.len() / 2) {
            let job = self.work.pop().unwrap();
            share_work.push(job);
        }

        self.controller.incr_send();
        self.sender.send(share_work).unwrap();
    }

    pub fn trace_loop(&mut self) {
        loop {
            if self.work.is_empty() {
                self.controller.start_waiting();
                if self.controller.is_trace_completed() {
                    self.controller.stop_waiting();
                    break;
                }

                loop {
                    let duration = std::time::Duration::from_millis(5);
                    let deadline = Instant::now().checked_add(duration).unwrap();

                    match self.receiver.recv_deadline(deadline) {
                        Ok(work) => {
                            self.work = work;
                            self.controller.stop_waiting();
                            self.controller.incr_recv();
                            break;
                        }
                        Err(_) => {
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
        debug_assert_eq!(self.sender.len(), 0);
        debug_assert_eq!(self.controller.yield_flag(), true);
        debug_assert_eq!(self.controller.is_trace_completed(), true);
        debug_assert_eq!(self.controller.mutators_stopped(), true);
    }

    fn receive_work(&mut self) {
    }
}
