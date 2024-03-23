use std::ptr::NonNull;

pub struct RawPtr<T> {
    ptr: NonNull<T>
}
