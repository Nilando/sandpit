use std::ptr::NonNull;
use super::trace::Trace;

pub struct GcPtr<T: Trace> {
    ptr: NonNull<T>,
}

unsafe impl<T: Trace> Trace for GcPtr<T> {
    fn trace(&self) {
        todo!()
    }
}
