use super::allocator::{Allocate, GenerationalArena};
use super::mutator::{Mutator, MutatorScope};
use super::trace::{Trace, TracerController, TraceMarker};
use std::collections::HashMap;
use std::sync::{Mutex, Arc};

// Collect is moved into a separate trait from the GcController, so that the monitor can work with
// a dynamic Collect type without needing to define the associated types of root and mutator
pub trait Collect: 'static {
    fn major_collect(&self);
    fn minor_collect(&self);
    fn arena_size(&self) -> usize;
}

pub trait GcController: Collect {
    type Root: Trace;
    type Mutator<'scope>: Mutator;

    fn build(callback: fn(&mut Self::Mutator<'_>) -> Self::Root) -> Self;
    fn mutate(&self, callback: fn(&Self::Root, &mut Self::Mutator<'_>));
    fn metrics(&self) -> HashMap<String, usize>;
}

unsafe impl<A: Allocate, T: Trace + Send> Send for Collector<A, T> {}
unsafe impl<A: Allocate, T: Trace + Sync> Sync for Collector<A, T> {}

pub struct Collector<A: Allocate, T: Trace> {
    arena: A::Arena,
    tracer: Arc<TracerController<TraceMarker<A>>>,
    root: T,
    lock: Mutex<()>
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

impl<A: Allocate, T: Trace> GcController for Collector<A, T> {
    type Root = T;
    type Mutator<'scope> = MutatorScope<'scope, A>;

    fn build(callback: fn(&mut Self::Mutator<'_>) -> T) -> Self {
        let arena = A::Arena::new();
        let tracer = Arc::new(TracerController::new());
        let mut scope = Self::Mutator::new(&arena, tracer.as_ref());
        let root = callback(&mut scope);

        drop(scope);

        let gc = Self {
            arena,
            tracer,
            root,
            lock: Mutex::new(()),
        };

        gc
    }

    fn mutate(&self, callback: fn(&Self::Root, &mut Self::Mutator<'_>)) {
        let collection_lock = self.lock.lock().unwrap();
        let mut mutator = Self::Mutator::new(&self.arena, &self.tracer);
        let _yield_lock = self.tracer.yield_lock();

        drop(collection_lock);

        callback(&self.root, &mut mutator);
    }

    fn metrics(&self) -> HashMap<String, usize> {
        let tracer_metrics = self.tracer.metrics();

        HashMap::from([
          //("memory_blocks".into(), tracer_metrics.objects_marked),
          //("large_objects".into(), tracer_metrics.objects_marked),
          ("prev_marked_objects".into(), tracer_metrics.objects_marked),
          ("prev_marked_space".into(), tracer_metrics.objects_marked),
          // ("prev_objects_freed".into(), tracer_metrics.objects_marked),
          ("arena_size".into(), self.arena.get_size()),
          // ("full_collections".into(), *self.full_collections.lock().unwrap()),
          // ("eden_collections".into(), *self.eden_collections.lock().unwrap())
        ])
    }
}

impl<A: Allocate, T: Trace> Collector<A, T> {
    fn collect(&self, marker: TraceMarker<A>) {
        self.tracer.clone().trace(&self.root, marker);
        self.arena.refresh();
    }
}
