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
        let binding = tracer.clone();
        let yield_lock = binding.get_yield_lock();
        let mut scope = MutatorScope::new(arena.as_ref(), tracer.clone(), yield_lock);
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
        let yield_lock = self.tracer.get_yield_lock();
        let mut scope = MutatorScope::new(self.arena.as_ref(), self.tracer.clone(), yield_lock);

        callback(&self.root, &mut scope);
    }

    pub fn collect(&mut self) {
        self.tracer.full_collection(self.arena.as_ref(), self.root);
    }

    fn monitor(&self) {
        let mut monitor = Monitor::new(
            self.arena.clone(),
            self.tracer.clone(),
            self.root.clone()
        );

        thread::spawn(move || {
            monitor.monitor();
        });
    }
}
