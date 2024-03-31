use std::cell::Cell;
use super::allocate::Allocate;
use super::mutator::MutatorScope;
use super::trace::TraceLeaf;

pub struct GcCell<T: TraceLeaf> {
    cell: Cell<T>
}

impl<T: TraceLeaf> GcCell<T> {
    pub fn new(val: T) -> Self {
        Self {
            cell: Cell::new(val)
        }
    }

    pub fn set<A: Allocate>(&self, _: &MutatorScope<A>, new_val: T) {
        self.cell.set(new_val)
    }

    pub fn replace<A: Allocate>(&self, _: &MutatorScope<A>, new_val: T) -> T {
        self.cell.replace(new_val)
    }
}
