use super::allocator::{Allocator, Allocate, GenerationalArena};
use super::mutator::{Mutator, MutatorScope};
use super::trace::{Trace, TraceMarker, TracerController};

use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLockReadGuard, atomic::{AtomicUsize, Ordering}};

pub trait Collect {
    fn major_collect(&self);
    fn minor_collect(&self);
    fn get_old_objects_count(&self) -> usize;
    fn get_arena_size(&self) -> usize;
    fn get_major_collections(&self) -> usize;
    fn get_minor_collections(&self) -> usize;
}

pub struct Collector<A: Allocate, T: Trace> {
    arena: A::Arena,
    tracer: Arc<TracerController<TraceMarker<A>>>,
    lock: Mutex<()>,
    root: T,
    major_collections: AtomicUsize,
    minor_collections: AtomicUsize,
}

impl<A: Allocate, T: Trace> Collect for Collector<A, T> {
    fn major_collect(&self) {
        let _lock = self.lock.lock().unwrap();
        self.major_collections.fetch_add(1, Ordering::SeqCst);
        self.collect(TraceMarker::new(self.arena.rotate_mark()));
    }

    fn minor_collect(&self) {
        let _lock = self.lock.lock().unwrap();
        self.minor_collections.fetch_add(1, Ordering::SeqCst);
        self.collect(TraceMarker::new(self.arena.current_mark()));
    }

    fn get_major_collections(&self) -> usize {
        self.major_collections.load(Ordering::SeqCst)
    }

    fn get_minor_collections(&self) -> usize {
        self.minor_collections.load(Ordering::SeqCst)
    }

    fn get_arena_size(&self) -> usize {
        self.arena.get_size()
    }
    
    fn get_old_objects_count(&self) -> usize {
        0
    }
}

impl<A: Allocate, T: Trace> Collector<A, T> {
    pub fn build(callback: fn(&mut MutatorScope<A>) -> T) -> Self {
        let arena = A::Arena::new();
        let tracer = Arc::new(TracerController::new());
        let mut mutator = MutatorScope::new(&arena, &tracer);
        let root = callback(&mut mutator);

        drop(mutator);

        Self {
            arena,
            tracer,
            root,
            lock: Mutex::new(()),
            major_collections: AtomicUsize::new(0),
            minor_collections: AtomicUsize::new(0),
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
