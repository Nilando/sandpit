use super::allocator::{Allocate, GenerationalArena};
use super::mutator::{Mutator, MutatorScope};
use super::trace::{Trace, TraceMarker, TracerController};

use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLockReadGuard};

pub struct Collector<A: Allocate> {
    arena: A::Arena,
    tracer: Arc<TracerController<TraceMarker<A>>>,
    marker: TraceMarker<A>,
    lock: Mutex<()>,
}

impl<A: Allocate> Collector<A> {
    pub fn new() -> Self {
        let arena = A::Arena::new();
        let marker = TraceMarker::new(arena.current_mark());
        Self {
            arena,
            marker,
            tracer: Arc::new(TracerController::new()),
            lock: Mutex::new(()),
        }
    }

    pub fn new_mutator(&self) -> (MutatorScope<A>, RwLockReadGuard<()>) {
        let lock = self.tracer.yield_lock();

        (MutatorScope::new(&self.arena, self.tracer.as_ref()), lock)
    }

    pub fn major_collect<T: Trace>(&self, root: &T) {
        let _lock = self.lock.lock().unwrap();
        self.collect(root, TraceMarker::new(self.arena.rotate_mark()));
    }

    pub fn minor_collect<T: Trace>(&self, root: &T) {
        let _lock = self.lock.lock().unwrap();
        self.collect(root, TraceMarker::new(self.arena.current_mark()));
    }

    pub fn arena_size(&self) -> usize {
        self.arena.get_size()
    }

    fn collect<T: Trace>(&self, root: &T, marker: TraceMarker<A>) {
        self.tracer.clone().trace(root, marker);
        self.arena.refresh();
    }
}
