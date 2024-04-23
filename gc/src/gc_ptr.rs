use std::ptr::NonNull;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::ops::Deref;

use super::mutator::Mutator;
use super::trace::Trace;

unsafe impl<T: Trace + Send> Send for GcPtr<T> {}
unsafe impl<T: Trace + Sync> Sync for GcPtr<T> {}

pub struct GcPtr<T: Trace> {
    ptr: AtomicPtr<T>,
}

impl<T: Trace> Deref for GcPtr<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe {
            let ptr = self.as_ptr();
            if ptr.is_null() { panic!("Attempt to deref a null GC ptr") }

            NonNull::new_unchecked(ptr).as_ref()
        }
    }
}

impl<T: Trace> GcPtr<T> {
    pub fn new(ptr: NonNull<T>) -> Self {
        Self { ptr: AtomicPtr::from(ptr.as_ptr()) }
    }

    pub fn null() -> Self {
        Self { ptr: AtomicPtr::new(std::ptr::null_mut()) }
    }

    pub fn set_null(&self) {
        self.ptr.store(std::ptr::null_mut(), Ordering::Relaxed)
    }

    pub unsafe fn as_ptr(&self) -> *mut T {
        self.ptr.load(Ordering::Relaxed)
    }

    pub unsafe fn as_nonnull(&self) -> NonNull<T> {
        let ptr = self.ptr.load(Ordering::Relaxed);

        NonNull::new(ptr).unwrap()
    }

    pub unsafe fn cast<V: Trace>(&self) -> GcPtr<V> {
        GcPtr::new(NonNull::new_unchecked(self.as_ptr().cast()))
    }

    pub fn is_null(&self) -> bool {
        unsafe { self.as_ptr().is_null() }
    }

    pub fn trigger_write_barrier<M: Mutator>(&self, mutator: &M) {
        unsafe { mutator.write_barrier(self.as_nonnull()) }
    }

    pub fn write_barrier<V: Trace, M: Mutator>(
        &self,
        mutator: &M,
        new_ptr: GcPtr<V>,
        callback: fn(&T) -> &GcPtr<V>,
    ) {
        unsafe {
            let ptr = self.as_nonnull();
            let old_ptr = callback(ptr.as_ref());

            old_ptr.unsafe_set(new_ptr);
            mutator.write_barrier(ptr);
        }
    }

    pub unsafe fn unsafe_set(&self, new_ptr: GcPtr<T>) {
        self.ptr.store(new_ptr.ptr.load(Ordering::Relaxed), Ordering::Relaxed)
    }
}

impl<T: Trace> Clone for GcPtr<T> {
    fn clone(&self) -> Self {
        Self { ptr: AtomicPtr::new(self.ptr.load(Ordering::Relaxed)) }
    }
}
