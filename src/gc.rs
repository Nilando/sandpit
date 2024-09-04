use super::mutator::Mutator;
use crate::barrier::WriteBarrier;
use std::marker::PhantomData;
use std::ops::Deref;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicPtr, Ordering};

use super::trace::Trace;

/// A pointer to a object stored in a Gc arena.
pub struct Gc<'a, T: Trace> {
    ptr: AtomicPtr<T>,
    _scope: PhantomData<&'a ()>,
}

impl<'a, T: Trace + 'a> Deref for Gc<'a, T> {
    type Target = T;

    // this is safe b/c we can only have a gcptr within a mutation context,
    // and gcptrs are guaranteed not to be swept during that context
    fn deref(&self) -> &'a Self::Target {
        unsafe { &*self.as_ptr() }
    }
}

impl<'a, T: Trace> Gc<'a, T> {
    pub fn new<M: Mutator<'a>>(mu: &'a M, obj: T) -> Self {
        const {
            assert!(!std::mem::needs_drop::<T>(), "Types that need drop cannot be GC")
        };

        let ptr = mu.alloc(obj);

        Self {
            ptr: AtomicPtr::new(ptr.as_ptr()),
            _scope: PhantomData::<&'a ()>,
        }
    }

    pub unsafe fn from_nonnull<M: Mutator<'a>>(mu: &'a M, ptr: NonNull<T>) -> Self {
        Self {
            ptr: AtomicPtr::new(ptr.as_ptr()),
            _scope: PhantomData::<&'a ()>,
        }
    }

    pub unsafe fn as_ptr(&self) -> *mut T {
        self.ptr.load(Ordering::SeqCst)
    }

    pub unsafe fn as_nonnull(&self) -> NonNull<T> {
        NonNull::new(self.ptr.load(Ordering::SeqCst)).unwrap()
    }

    pub unsafe fn set(&self, new: Gc<'a, T>) {
        self.ptr.store(new.as_ptr(), Ordering::SeqCst)
    }

    pub fn write_barrier<F, M: Mutator<'a>>(&self, mu: &'a M, f: F) 
    where
        F: FnOnce(&WriteBarrier<T>)
    {
        let barrier = WriteBarrier::new(self.deref());
        f(&barrier);

        if mu.is_marked(self.clone()) {
            mu.retrace(self.clone());
        }
    }
}

impl<'a, T: Trace> Clone for Gc<'a, T> {
    fn clone(&self) -> Self {
        Self {
            ptr: AtomicPtr::new(self.ptr.load(Ordering::SeqCst)),
            _scope: PhantomData::<&'a ()>,
        }
    }
}
