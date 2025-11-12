use super::trace::Trace;
use super::trace_job::TraceJob;
use super::collector::Collector;
use crate::debug::{gc_debug, gc_trace};
use crate::heap::mark;
use crate::gc::Gc;
use crate::header::{GcHeader, GcMark};
use alloc::vec::Vec;
use alloc::vec;
use alloc::format;

/// Internal type used by the GC to perform tracing.
pub struct Tracer<'a> {
    collector: &'a Collector,
    mark: GcMark,
    pub mark_count: usize,
    work: Vec<TraceJob>,
}

impl<'a> Tracer<'a> {
    pub(crate) fn new(collector: &'a Collector, mark: GcMark) -> Self {
        Self {
            collector,
            mark,
            mark_count: 0,
            work: vec![],
        }
    }

    pub(crate) fn mark<T: Trace + ?Sized>(&mut self, gc: Gc<'_, T>) -> bool {
        gc_trace(&format!("marking\t{}\tptr\t{:#x}\theader\t{:#x}",
            core::any::type_name::<T>(),
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

        unsafe { mark(alloc_ptr as *mut u8, alloc_layout, self.mark) };

        return true;
    }

    pub(crate) fn mark_and_trace<T: Trace + ?Sized>(&mut self, gc: Gc<'_, T>) {
        if !self.mark(gc.clone()) || T::IS_LEAF {
            return;
        }

        self.work.push(TraceJob::new(gc.as_thin()));
    }

    pub(crate) fn trace_loop(&mut self) -> usize {
        loop {
            if self.work.is_empty() {
                match self.collector.recv_work() {
                    Some(work) => self.work = work,
                    None => break,
                }
            }

            self.do_work();
            self.share_work();
        }

        gc_debug("Tracer Exiting");

        self.mark_count
    }

    fn do_work(&mut self) {
        for _ in 0..self.collector.config().trace_chunk_size {
            match self.work.pop() {
                Some(job) => job.trace(self),
                None => break,
            }
        }
    }

    fn share_work(&mut self) {
        if self.collector.config().trace_share_min >= self.work.len() || self.collector.has_work() {
            return;
        }

        let split_at = (self.work.len() as f32 * self.collector.get_trace_share_ratio()).floor() as usize;
        let share_work = self.work.split_off(split_at);

        if !share_work.is_empty() {
            self.collector.send_work(share_work);
        }
    }

    fn increment_mark_count(&mut self) {
        self.mark_count += 1;
    }

    pub(crate) fn get_mark(&self) -> GcMark {
        self.mark
    }
}
