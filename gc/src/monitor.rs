use super::allocate::{GenerationalArena, Allocate};
use super::tracer_controller::TracerController;
use super::trace::Trace;
use super::gc_ptr::GcPtr;

use std::thread;
use std::time;
use std::sync::Arc;

pub struct Monitor<A: Allocate, R: Trace> {
    arena: Arc<A::Arena>,
    root: GcPtr<R>,
    controller: Arc<TracerController<A>>,
    prev_block_count: usize,
    debt: f64
}

unsafe impl<A: Allocate, R: Trace> Send for Monitor<A, R> {}
unsafe impl<A: Allocate, R: Trace> Sync for Monitor<A, R> {}

const DEBT_CEILING: f64 = 10.0;
const DEBT_INTEREST_RATE: f64 = 1.5;

impl<A: Allocate, R: Trace> Monitor<A, R> {
    pub fn new(arena: Arc<A::Arena>, controller: Arc<TracerController<A>>, root: GcPtr<R>) -> Self {
        Self {
            arena,
            controller,
            root,
            prev_block_count: 0,
            debt: 0.0
        }
    }

    pub fn monitor(&mut self) {
        loop {
            self.sleep();

            let test = std::sync::Arc::<<A as Allocate>::Arena>::get_mut(&mut self.arena);
            if test.is_some() {
                return;
            }

            self.calculate_debt();
            self.prev_block_count = self.arena.block_count();

            if self.debt >= DEBT_CEILING {
                self.controller.full_collection(self.arena.as_ref(), self.root);
                self.debt = 0.0;
            }
        }
    }

    fn calculate_debt(&mut self) {
        self.debt = self.debt * DEBT_INTEREST_RATE;

        if self.prev_block_count < self.arena.block_count() {
            let new_debt = self.arena.block_count() - self.prev_block_count;

            self.debt = self.debt + new_debt as f64;
        }
    }

    fn sleep(&self) {
        let millis = time::Duration::from_millis(500);

        thread::sleep(millis);
    }
}
