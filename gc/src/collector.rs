use super::allocator::{Allocate, GenerationalArena};
use super::mutator::MutatorScope;
use super::trace::{Marker, Trace, TraceMarker, TracerController};

use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};

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
    old_objects: AtomicUsize,
}

impl<A: Allocate, T: Trace> Collect for Collector<A, T> {
    fn major_collect(&self) {
        let _lock = self.lock.lock().unwrap();
        self.old_objects.store(0, Ordering::SeqCst);
        self.major_collections.fetch_add(1, Ordering::SeqCst);
        self.collect(TraceMarker::new(self.arena.rotate_mark()).into());
    }

    fn minor_collect(&self) {
        let _lock = self.lock.lock().unwrap();
        self.minor_collections.fetch_add(1, Ordering::SeqCst);
        self.collect(TraceMarker::new(self.arena.current_mark()).into());
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
        self.old_objects.load(Ordering::SeqCst)
    }
}

impl<A: Allocate, T: Trace> Collector<A, T> {
    pub fn build(callback: fn(&mut MutatorScope<A>) -> T) -> Self {
        let arena = A::Arena::new();
        let tracer = Arc::new(TracerController::new());
        let lock = tracer.yield_lock();
        let mut mutator = MutatorScope::new(&arena, &tracer, lock);
        let root = callback(&mut mutator);

        drop(mutator);

        Self {
            arena,
            tracer,
            root,
            lock: Mutex::new(()),
            major_collections: AtomicUsize::new(0),
            minor_collections: AtomicUsize::new(0),
            old_objects: AtomicUsize::new(0),
        }
    }

    pub fn mutate(&self, callback: fn(&T, &mut MutatorScope<A>)) {
        let mut mutator = self.new_mutator();

        callback(&self.root, &mut mutator);
    }

    fn new_mutator(&self) -> MutatorScope<A> {
        let lock = self.tracer.yield_lock();

        MutatorScope::new(&self.arena, self.tracer.as_ref(), lock)
    }

    fn collect(&self, marker: Arc<TraceMarker<A>>) {
        self.tracer.clone().trace(&self.root, marker.clone());
        self.old_objects
            .fetch_add(marker.get_mark_count(), Ordering::SeqCst);
        self.arena.refresh();
    }
}
