use super::trace::Trace;
use super::trace_job::TraceJob;
use super::tracer_controller::TracerController;
use std::sync::Arc;
use std::ptr::NonNull;
use std::cell::Cell;
use crate::allocator::{GenerationalArena, Allocator, Allocate};

pub struct Tracer {
    controller: Arc<TracerController>,
    mark: <<Allocator as Allocate>::Arena as GenerationalArena>::Mark,
    mark_count: Cell<usize>,
    work: Vec<TraceJob>,

}

impl Tracer {
    pub fn new(
        controller: Arc<TracerController>, 
        mark: <<Allocator as Allocate>::Arena as GenerationalArena>::Mark
    ) -> Self {
        Self {
            controller,
            mark,
            mark_count: Cell::new(0),
            work: vec![],
        }
    }

    pub fn get_mark_count(&self) -> usize {
        self.mark_count.get()
    }

    pub fn trace<T: Trace>(&mut self, ptr: NonNull<T>) {
        if !self.set_mark(ptr) {
            return;
        }

        if T::IS_LEAF {
            return;
        }

        self.work.push(TraceJob::new(ptr));
    }

    pub fn flush_work(&mut self) {
        let mut work = vec![];

        std::mem::swap(&mut work, &mut self.work);

        self.controller.send_work(work);
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

    fn set_mark<T: Trace>(&self, ptr: NonNull<T>) -> bool {
        let mark = <Allocator as Allocate>::get_mark(ptr);

        if mark == self.mark {
            return false;
        }

        self.increment_mark_count();

        <Allocator as Allocate>::set_mark(ptr, self.mark);

        true
    }

    fn increment_mark_count(&self) {
        self.mark_count.set(self.mark_count.get() + 1);
    }
}
