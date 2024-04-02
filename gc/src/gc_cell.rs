use std::cell::Cell;
use super::trace::TraceLeaf;

// a gc cell neeeds to be traceleaf to avoid concurrency issues with the tracer
// If a gccell could contain gcptrs and you could mutate that cell while
// the tracer is tracing it could break the tracer
pub struct GcCell<T: TraceLeaf> {
    cell: Cell<T>
}

impl<T: TraceLeaf> GcCell<T> {
    pub fn new(val: T) -> Self {
        Self {
            cell: Cell::new(val)
        }
    }

    pub fn set(&self, new_val: T) {
        self.cell.set(new_val)
    }
}

impl <T: TraceLeaf + Copy> GcCell<T> {
    pub fn get(&self) -> T {
        self.cell.get()
    }
}
