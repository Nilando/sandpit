use std::cell::Cell;
use super::trace::Trace;

pub struct GcCell<T> {
    cell: Cell<T>
}

unsafe impl<T> Trace for GcCell<T> {
    fn trace(&self) {
        todo!()
    }
}
