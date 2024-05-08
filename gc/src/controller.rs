use super::allocator::{Allocate, GenerationalArena};
use super::mutator::{Mutator, MutatorScope};
use super::trace::{Trace, TraceMarker, TracerController};
use super::collector::{Collector, Collect};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub trait GcController: Collect {
    type Root: Trace;
    type Mutator<'scope>: Mutator;

    fn build(callback: fn(&mut Self::Mutator<'_>) -> Self::Root) -> Self;
    fn mutate(&self, callback: fn(&Self::Root, &mut Self::Mutator<'_>));
    fn metrics(&self) -> HashMap<String, usize>;
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

        Self {
            arena,
            tracer,
            root,
            lock: Mutex::new(()),
        }
    }

    fn mutate(&self, callback: fn(&Self::Root, &mut Self::Mutator<'_>)) {
        let collection_lock = self.lock.lock().unwrap();
        let mut mutator = Self::Mutator::new(&self.arena, &self.tracer);
        let _yield_lock = self.tracer.yield_lock();

        drop(collection_lock);

        callback(&self.root, &mut mutator);
    }

    fn metrics(&self) -> HashMap<String, usize> {
        todo!();
        /*
        HashMap::from([
            // ("memory_blocks".into(), tracer_metrics.objects_marked),
            // ("large_objects".into(), tracer_metrics.objects_marked),
            ("prev_marked_objects".into(), tracer_metrics.objects_marked),
            ("prev_marked_space".into(), tracer_metrics.objects_marked),
            // ("prev_objects_freed".into(), tracer_metrics.objects_marked),
            ("arena_size".into(), self.arena.get_size()),
            // ("full_collections".into(), *self.full_collections.lock().unwrap()),
            // ("eden_collections".into(), *self.eden_collections.lock().unwrap())
        ])
        */
    }
}
