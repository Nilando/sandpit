use std::ptr::NonNull;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::ops::Deref;

use super::mutator::Mutator;
use super::trace::Trace;

unsafe impl<T: Trace + Send> Send for GcPtr<T> {}
unsafe impl<T: Trace + Sync> Sync for GcPtr<T> {}

//unsafe impl<T: Trace + Send> Send for GcNonNull<T> {}
//unsafe impl<T: Trace + Sync> Sync for GcNonNull<T> {}

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

    pub unsafe fn deref_unchecked(&self) -> &T {
        debug_assert!(self.is_null());

        NonNull::new_unchecked(self.as_ptr()).as_ref()
    }

    pub fn is_null(&self) -> bool {
        unsafe { self.as_ptr().is_null() }
    }

    pub fn write_barrier<V: Trace, M: Mutator>(
        &self,
        mutator: &mut M,
        new_ptr: GcPtr<V>,
        callback: fn(&T) -> &GcPtr<V>,
    ) {
        let self_ref = self.deref();

        unsafe {
            let old_ptr = callback(self_ref);

            old_ptr.unsafe_set(new_ptr)
        }

        mutator.write_barrier(NonNull::from(self_ref));
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
