use super::trace::Trace;
use super::trace_job::TraceJob;
use super::tracer_controller::TracerController;
use crate::debug::{gc_debug, gc_trace};
use crate::heap::Heap;
use crate::gc::Gc;
use crate::header::{GcHeader, GcMark};

/// Internal type used by the GC to perform tracing.
pub struct Tracer<'a> {
    // sometimes ID might be helpful in debugging, but currently not used anywhere
    controller: &'a TracerController,
    mark: GcMark,
    mark_count: usize,
    work: Vec<TraceJob>,
}

impl<'a> Tracer<'a> {
    pub(crate) fn new(controller: &'a TracerController, mark: GcMark) -> Self {
        Self {
            controller,
            mark,
            mark_count: 0,
            work: vec![],
        }
    }

    pub(crate) fn mark<T: Trace + ?Sized>(&mut self, gc: Gc<'_, T>) -> bool {
        let type_name = core::any::type_name::<T>();
        gc_trace(&format!("marking\t{}\tptr\t{:#x}\theader\t{:#x}",
            type_name,
            gc.as_thin().as_ptr() as usize,
            gc.get_header_ptr() as usize));

        let header = gc.get_header();
        let alloc_ptr = gc.get_header_ptr();
        let alloc_layout = gc.get_layout();

        if header.get_mark() == self.mark {
            return false;
        }

        header.set_mark(self.mark);

        self.increment_mark_count();

        unsafe { Heap::mark(alloc_ptr as *mut u8, alloc_layout, self.mark) };

        return true;
    }

    pub(crate) fn mark_and_trace<T: Trace + ?Sized>(&mut self, gc: Gc<'_, T>) {
        if !self.mark(gc.clone()) || T::IS_LEAF {
            return;
        }

        self.work.push(TraceJob::new(gc.as_thin()));
    }

    pub(crate) fn flush_work(&mut self) {
        let mut work = vec![];

        core::mem::swap(&mut work, &mut self.work);

        self.controller.send_work(work);
    }

    pub(crate) fn trace_loop(&mut self) -> usize {
        loop {
            if self.work.is_empty() {
                match self.controller.recv_work() {
                    Some(work) => self.work = work,
                    None => break,
                }
            }

            self.do_work();
            self.share_work();
            self.controller.raise_yield_flag();
        }

        gc_debug("Tracer Exited Successfully");

        debug_assert_eq!(self.work.len(), 0);

        self.mark_count
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
        if self.controller.trace_share_min >= self.work.len() || self.controller.has_work() {
            return;
        }

        let split_at = (self.work.len() as f32 * self.controller.get_trace_share_ratio()).floor() as usize;
        let share_work = self.work.split_off(split_at);

        if !share_work.is_empty() {
            self.controller.send_work(share_work);
        }
    }

    fn increment_mark_count(&mut self) {
        self.mark_count += 1;
    }

    pub(crate) fn get_mark_count(&self) -> usize {
        self.mark_count
    }

    pub(crate) fn get_mark(&self) -> GcMark {
        self.mark
    }
}
