use super::allocator::{Allocate, GenerationalArena};
use super::mutator::{Mutator, MutatorScope};
use super::trace::{Trace, TraceMarker, TracerController};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// Collect is moved into a separate trait from the GcController, so that the monitor can work with
// a dynamic Collect type without needing to define the associated types of root and mutator
pub trait Collect: 'static {
    fn major_collect(&self);
    fn minor_collect(&self);
    fn arena_size(&self) -> usize;
}

unsafe impl<A: Allocate, T: Trace + Send> Send for Collector<A, T> {}
unsafe impl<A: Allocate, T: Trace + Sync> Sync for Collector<A, T> {}

pub struct Collector<A: Allocate, T: Trace> {
    pub arena: A::Arena,
    pub tracer: Arc<TracerController<TraceMarker<A>>>,
    pub root: T,
    pub lock: Mutex<()>,
}

impl<A: Allocate, T: Trace> Collect for Collector<A, T> {
    fn major_collect(&self) {
        let _lock = self.lock.lock().unwrap();
        let marker = TraceMarker::new(self.arena.rotate_mark());
        self.collect(marker);
    }

    fn minor_collect(&self) {
        let _lock = self.lock.lock().unwrap();
        let marker = TraceMarker::new(self.arena.current_mark());
        self.collect(marker);
    }

    fn arena_size(&self) -> usize {
        self.arena.get_size()
    }
}

impl<A: Allocate, T: Trace> Collector<A, T> {
    fn collect(&self, marker: TraceMarker<A>) {
        self.tracer.clone().trace(&self.root, marker);
        self.arena.refresh();
    }
}
