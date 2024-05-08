use super::trace::Trace;
use crate::allocator::{Allocate, GenerationalArena, Marker as AllocMarker};
use std::ptr::NonNull;

pub trait Marker: Clone {
    type Mark: AllocMarker;

    fn needs_trace<T: Trace>(&self, ptr: NonNull<T>) -> bool;
    fn set_mark(&mut self, mark: Self::Mark);
}

impl<A: Allocate> Marker for TraceMarker<A> {
    type Mark = <<A as Allocate>::Arena as GenerationalArena>::Mark;

    fn needs_trace<T: Trace>(&self, ptr: NonNull<T>) -> bool {
        if A::get_mark(ptr) == self.mark {
            return false;
        }

        A::set_mark(ptr, self.mark);

        T::needs_trace()
    }

    fn set_mark(&mut self, mark: Self::Mark) {
        self.mark = mark;
    }
}

pub struct TraceMarker<A: Allocate> {
    mark: <<A as Allocate>::Arena as GenerationalArena>::Mark,
}

impl<A: Allocate> TraceMarker<A> {
    pub fn new(mark: <<A as Allocate>::Arena as GenerationalArena>::Mark) -> Self {
        Self { mark }
    }
}

impl<A: Allocate> Clone for TraceMarker<A> {
    fn clone(&self) -> Self {
        Self { mark: self.mark }
    }
}
