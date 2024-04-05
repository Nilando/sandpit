use super::allocate::{Allocate, GenerationalArena};
use super::gc_ptr::GcPtr;
use super::mutator::MutatorScope;
use super::trace::Trace;
use super::tracer_controller::TracerController;
use super::monitor::Monitor;

use std::sync::Arc;
use std::thread;

pub struct Gc<A: Allocate, Root: Trace> {
    arena: Arc<A::Arena>,
    tracer: Arc<TracerController<A>>,
    root: GcPtr<Root>,
}

unsafe impl<T: Allocate, Root: Trace + Send> Send for Gc<T, Root> {}
unsafe impl<T: Allocate, Root: Trace + Sync> Sync for Gc<T, Root> {}

impl<A: Allocate, T: Trace> Gc<A, T> {
    pub fn build(callback: fn(&mut MutatorScope<A>) -> GcPtr<T>) -> Self {
        let arena = Arc::new(A::Arena::new());
        let tracer = Arc::new(TracerController::<A>::new());
        let mut scope = MutatorScope::new(arena.as_ref(), tracer.clone());
        let root = callback(&mut scope);
        let gc = Self {
            arena,
            tracer,
            root,
        };

        gc.monitor();

        gc
    }

    pub fn mutate(&mut self, callback: fn(&GcPtr<T>, &mut MutatorScope<A>)) {
        let mut scope = MutatorScope::new(self.arena.as_ref(), self.tracer.clone());

        callback(&self.root, &mut scope);
    }

    pub fn collect(&mut self) {
        self.tracer.full_collection(self.arena.as_ref(), self.root);
    }

    fn monitor(&self) {
        let monitor = Monitor::new(self.arena.clone());

        thread::scope(|s| {
            s.spawn(move || {
                monitor.monitor();
            });
        });
    }
}
