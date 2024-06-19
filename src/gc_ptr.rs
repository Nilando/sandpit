use std::marker::PhantomData;
use std::ops::Deref;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicPtr, Ordering};

use super::trace::Trace;

/// A pointer to a object stored in a Gc arena.
pub struct GcPtr<T: Trace> {
    ptr: AtomicPtr<T>,
    _mark: PhantomData<*const ()>,
}

impl<T: Trace> Deref for GcPtr<T> {
    type Target = T;

    // this is safe b/c we can only have a gcptr within a mutation context,
    // and gcptrs are guaranteed not to be swept during that context
    fn deref(&self) -> &Self::Target {
        let ptr = self.as_ptr();

        if ptr.is_null() {
            panic!("Attempt to deref a null GC ptr")
        }

        unsafe { &*ptr }
    }
}

impl<T: Trace> GcPtr<T> {
    pub(crate) fn new(ptr: NonNull<T>) -> Self {
        Self {
            ptr: AtomicPtr::from(ptr.as_ptr()),
            _mark: PhantomData::<*const ()>,
        }
    }

    pub(crate) fn null() -> Self {
        Self {
            ptr: AtomicPtr::new(std::ptr::null_mut()),
            _mark: PhantomData::<*const ()>,
        }
    }

    pub fn set_null(&self) {
        self.ptr.store(std::ptr::null_mut(), Ordering::SeqCst)
    }

    pub fn as_ptr(&self) -> *mut T {
        self.ptr.load(Ordering::SeqCst)
    }

    pub fn as_nonnull(&self) -> NonNull<T> {
        let ptr = self.ptr.load(Ordering::SeqCst);

        NonNull::new(ptr).unwrap()
    }

    pub fn is_null(&self) -> bool {
        self.as_ptr().is_null()
    }

    // This is unsafe b/c if the object holding this ptr has already been scanned,
    // then if the collector finishes collection with out rescanning that object,
    // this will become dangling.
    //
    // Either the swapped pointer must be guaranteed to not exist before the end
    // of this mutation scope, or the object containing this ptr must be rescanned
    //
    // Instead of using this directly, use the mutators write_barrier, which is safe
    // b/c it ensured the object is rescanned
    pub unsafe fn swap(&self, new_ptr: GcPtr<T>) {
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
