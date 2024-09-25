use super::trace::Trace;
use super::trace_job::TraceJob;
use super::tracer_controller::TracerController;
use crate::allocator::Allocator;
use crate::header::{GcHeader, GcMark};
use log::debug;
use std::cell::Cell;
use std::sync::Arc;
use std::ptr::NonNull;
use crate::gc::Gc;

pub struct Tracer {
    id: usize,
    controller: Arc<TracerController>,
    mark: GcMark,
    mark_count: Cell<usize>,
    work: Vec<TraceJob>,
}

impl Tracer {
    pub fn new(controller: Arc<TracerController>, mark: GcMark, id: usize) -> Self {
        Self {
            id,
            controller,
            mark,
            mark_count: Cell::new(0),
            work: vec![],
        }
    }

    pub fn id(&self) -> usize {
        self.id
    }

    pub fn get_mark_count(&self) -> usize {
        self.mark_count.get()
    }

    pub fn mark_and_trace_slice<'gc, T: Trace>(&mut self, gc: Gc<'gc, [T]>) {
        let header = gc.get_header();
        let alloc_layout = gc.get_layout();
        let alloc_ptr = header.as_ptr();
        let gc_ptr = &*gc as *const [T] as *mut [T];

        if header.get_mark() == self.mark {
            return;
        }

        header.set_mark(self.mark);

        self.increment_mark_count();

        unsafe { Allocator::mark(alloc_ptr, alloc_layout, self.mark).expect("set mark failure") };

        if T::IS_LEAF {
            return;
        }

        // self.work.push(TraceJob::new(NonNull::new(gc_ptr).unwrap()));
        todo!()
    }

    // doesn't work for pointer to dynamically sized types
    pub fn mark_and_trace<'gc, T: Trace>(&mut self, gc: Gc<'gc, T>) {
        //debug!("(TRACER: {}) OBJ = {}, ADDR = {:?}", self.id, std::any::type_name::<T>(), &*gc as *const T as usize);

        let header = gc.get_header();
        let alloc_layout = gc.get_layout();
        let alloc_ptr = header.as_ptr();
        let gc_ptr = &*gc as *const T as *mut T;

        if header.get_mark() == self.mark {
            return;
        }

        header.set_mark(self.mark);

        self.increment_mark_count();

        unsafe { Allocator::mark(alloc_ptr, alloc_layout, self.mark).expect("set mark failure") };

        if T::IS_LEAF {
            return;
        }

        self.work.push(TraceJob::new(NonNull::new(gc_ptr).unwrap()));
    }

    pub fn flush_work(&mut self) {
        let mut work = vec![];

        std::mem::swap(&mut work, &mut self.work);

        self.controller.send_work(work);
    }

    pub fn trace_loop(&mut self) -> usize {
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

        self.mark_count.get()
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

        debug!("(TRACER: {}) SHARING WORK = {}", self.id, share_work.len());

        self.controller.send_work(share_work);
    }

    fn increment_mark_count(&self) {
        self.mark_count.set(self.mark_count.get() + 1);
    }
}
