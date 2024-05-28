use crossbeam_channel::{Sender, Receiver};
use super::marker::Marker;
use super::trace::Trace;
use super::trace_packet::TraceJob;
use super::tracer_controller::TracerController;
use std::ptr::NonNull;
use std::sync::Arc;

const MIN_SHARE_WORK: usize = 1_000;

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
        for _ in 0..10_000 {
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
                if self.controller.is_trace_completed() {
                    break;
                }

                self.controller.start_waiting();

                if self.controller.tracers_waiting() == self.controller.num_tracers() &&
                    self.controller.sent() == self.controller.received() {
                    if self.controller.mutators_stopped() {
                        self.controller.signal_trace_end();
                        self.controller.stop_waiting();
                        break;
                    } else {
                        self.controller.wait_for_mutators();
                        self.controller.stop_waiting();
                        continue;
                    }
                }

                println!("recv: {:?}", self.controller.received());
                println!("sent: {:?}", self.controller.sent());
                println!("waiting: {}", self.controller.tracers_waiting());

                let work = self.receiver.recv().unwrap();
                self.work = work;

                self.controller.stop_waiting();
                println!("stopping wait");
                self.controller.incr_recv();
            }

            self.do_work();
            self.share_work();
        }
    }
}
