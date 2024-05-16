use super::trace::Trace;
use crate::allocator::{Allocate, GenerationalArena, Marker as AllocMarker};
use std::ptr::NonNull;
use std::sync::atomic::{AtomicUsize, Ordering};

pub trait Marker: Send + Sync {
    type Mark: AllocMarker;

    fn set_mark<T: Trace>(&self, ptr: NonNull<T>) -> bool;
    fn get_mark_count(&self) -> usize;
}

impl<A: Allocate> Marker for TraceMarker<A> {
    type Mark = <<A as Allocate>::Arena as GenerationalArena>::Mark;

    fn set_mark<T: Trace>(&self, ptr: NonNull<T>) -> bool {
        let mark = A::get_mark(ptr);

        if mark == self.mark {
            return false
        }

        self.mark_count.fetch_add(1, Ordering::Relaxed);

        A::set_mark(ptr, self.mark);

        true
    }

    fn get_mark_count(&self) -> usize {
        self.mark_count.load(Ordering::Relaxed)
    }
}

pub struct TraceMarker<A: Allocate> {
    mark: <<A as Allocate>::Arena as GenerationalArena>::Mark,
    mark_count: AtomicUsize,
}

impl<A: Allocate> TraceMarker<A> {
    pub fn new(mark: <<A as Allocate>::Arena as GenerationalArena>::Mark) -> Self {
        Self { mark, mark_count: AtomicUsize::new(0) }
    }
}
