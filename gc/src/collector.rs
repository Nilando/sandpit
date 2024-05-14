use super::allocator::{Allocator, Allocate, GenerationalArena};
use super::mutator::{Mutator, MutatorScope};
use super::trace::{Trace, TraceMarker, TracerController};

use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLockReadGuard};

pub trait Collect {
    fn major_collect(&self);
    fn minor_collect(&self);
    fn old_objects_count(&self) -> usize;
    fn arena_size(&self) -> usize;
}

pub struct Collector<A: Allocate, T: Trace> {
    arena: A::Arena,
    tracer: Arc<TracerController<TraceMarker<A>>>,
    marker: TraceMarker<A>,
    lock: Mutex<()>,
    root: T,
}

impl<A: Allocate, T: Trace> Collect for Collector<A, T> {
    fn major_collect(&self) {
        let _lock = self.lock.lock().unwrap();
        self.collect(TraceMarker::new(self.arena.rotate_mark()));
    }

    fn minor_collect(&self) {
        let _lock = self.lock.lock().unwrap();
        self.collect(TraceMarker::new(self.arena.current_mark()));
    }

    fn arena_size(&self) -> usize {
        self.arena.get_size()
    }
    
    fn old_objects_count(&self) -> usize {
        0
    }
}

impl<A: Allocate, T: Trace> Collector<A, T> {
    pub fn build(callback: fn(&mut MutatorScope<A>) -> T) -> Self {
        let arena = A::Arena::new();
        let tracer = Arc::new(TracerController::new());
        let mut mutator = MutatorScope::new(&arena, &tracer);
        let root = callback(&mut mutator);
        let marker = TraceMarker::new(arena.current_mark());

        drop(mutator);

        Self {
            arena,
            marker,
            tracer,
            root,
            lock: Mutex::new(()),
        }
    }

    pub fn mutate(&self, callback: fn(&T, &mut MutatorScope<A>)) {
        let (mut mutator, _lock) = self.new_mutator();

        callback(&self.root, &mut mutator);
    }

    fn new_mutator(&self) -> (MutatorScope<A>, RwLockReadGuard<()>) {
        let lock = self.tracer.yield_lock();

        (MutatorScope::new(&self.arena, self.tracer.as_ref()), lock)
    }

    fn collect(&self, marker: TraceMarker<A>) {
        self.tracer.clone().trace(&self.root, marker);
        self.arena.refresh();
    }
}
