use super::raw_ptr::RawPtr;
use super::trace::Trace;

pub struct GcPtr<T> {
    ptr: RawPtr<T>,
}

unsafe impl<T> Trace for GcPtr<T> {
    fn trace(&self) {
        todo!()
    }
}
