use std::marker::PhantomData;
use std::ops::Deref;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicPtr, Ordering};

use super::trace::Trace;

pub struct GcPtr<T: Trace> {
    ptr: AtomicPtr<T>,
    _mark: PhantomData<*const ()>,
}

impl<T: Trace> Deref for GcPtr<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe {
            let ptr = self.as_ptr();
            if ptr.is_null() {
                panic!("Attempt to deref a null GC ptr")
            }

            NonNull::new_unchecked(ptr).as_ref()
        }
    }
}

impl<T: Trace> GcPtr<T> {
    pub fn new(ptr: NonNull<T>) -> Self {
        Self {
            ptr: AtomicPtr::from(ptr.as_ptr()),
            _mark: PhantomData::<*const ()>,
        }
    }

    pub fn null() -> Self {
        Self {
            ptr: AtomicPtr::new(std::ptr::null_mut()),
            _mark: PhantomData::<*const ()>,
        }
    }

    pub fn set_null(&self) {
        self.ptr.store(std::ptr::null_mut(), Ordering::SeqCst)
    }

    pub unsafe fn as_ptr(&self) -> *mut T {
        self.ptr.load(Ordering::SeqCst)
    }

    pub unsafe fn as_nonnull(&self) -> NonNull<T> {
        let ptr = self.ptr.load(Ordering::SeqCst);

        NonNull::new(ptr).unwrap()
    }

    pub unsafe fn cast<V: Trace>(&self) -> GcPtr<V> {
        GcPtr::new(NonNull::new_unchecked(self.as_ptr().cast()))
    }

    pub fn is_null(&self) -> bool {
        unsafe { self.as_ptr().is_null() }
    }

    pub unsafe fn unsafe_set(&self, new_ptr: GcPtr<T>) {
        self.ptr
            .store(new_ptr.ptr.load(Ordering::SeqCst), Ordering::SeqCst)
    }
}

impl<T: Trace> Clone for GcPtr<T> {
    fn clone(&self) -> Self {
        Self {
            ptr: AtomicPtr::new(self.ptr.load(Ordering::SeqCst)),
            _mark: PhantomData::<*const ()>,
        }
    }
}
