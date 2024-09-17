use super::trace::Trace;
use super::trace_job::TraceJob;
use super::tracer_controller::TracerController;
use crate::allocator::Allocator;
use crate::header::{GcMark, Header};
use log::debug;
use std::alloc::Layout;
use std::cell::Cell;
use std::ptr::NonNull;
use std::sync::Arc;

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

    pub fn trace<T: Trace>(&mut self, ptr: NonNull<T>) {
        debug!("(TRACER: {}) OBJ = {}, ADDR = {:?}", self.id, std::any::type_name::<T>(), ptr.as_ptr());
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

    fn set_mark<T: Trace>(&self, ptr: NonNull<T>) -> bool {
        // TODO if T: DYN_HEADER
        // instead of getting a normal header
        // get a dyn header
        let header = unsafe { &*Header::get_ptr(ptr) };
        let mark = header.get_mark();

        if mark == self.mark {
            return false;
        }

        self.increment_mark_count();
        let header_layout = Layout::new::<Header>();
        let object_layout = Layout::new::<T>();
        let (alloc_layout, _object_offset) = header_layout
            .extend(object_layout)
            .expect("Bad Alloc Layout");

        header.set_mark(self.mark);

        Allocator::mark(header as *const Header as *mut u8, alloc_layout, self.mark)
            .expect("set mark failure");

        true
    }

    fn increment_mark_count(&self) {
        self.mark_count.set(self.mark_count.get() + 1);
    }
}
