use std::ptr::NonNull;
use super::trace::Trace;

pub struct GcPtr<T> {
    ptr: NonNull<T>,
}

impl<T> GcPtr<T> {
    pub fn new(ptr: NonNull<T>) -> Self {
        Self { ptr }
    }
}

unsafe impl<T: Trace> Trace for GcPtr<T> {
    fn trace(&self) {
        todo!()
    }
}
