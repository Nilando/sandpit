use super::trace::Trace;
use crate::allocator::{Allocate, GenerationalArena, Marker as AllocMarker};
use std::ptr::NonNull;

pub trait Marker: Clone {
    type Mark: AllocMarker;

    fn is_marked<T: Trace>(&self, ptr: NonNull<T>) -> bool;
    fn is_rescan<T: Trace>(&self, ptr: NonNull<T>) -> bool;
    fn set_mark<T: Trace>(&self, ptr: NonNull<T>);
}

impl<A: Allocate> Marker for TraceMarker<A> {
    type Mark = <<A as Allocate>::Arena as GenerationalArena>::Mark;

    fn is_marked<T: Trace>(&self, ptr: NonNull<T>) -> bool {
        A::get_mark(ptr) == self.mark
    }

    fn is_rescan<T: Trace>(&self, ptr: NonNull<T>) -> bool {
        A::get_mark(ptr).is_rescan()
    }

    fn set_mark<T: Trace>(&self, ptr: NonNull<T>) {
        A::set_mark(ptr, self.mark);
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
