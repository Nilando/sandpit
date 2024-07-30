use super::mutator::Mutator;
use std::marker::PhantomData;
use std::ops::Deref;
use std::ptr::NonNull;

use super::trace::Trace;

/// A pointer to a object stored in a Gc arena.
#[derive(Clone)]
pub struct Gc<'a, T: Trace> {
    ptr: NonNull<T>,
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
        let ptr = mu.alloc(obj);

        Self {
            ptr,
            _scope: PhantomData::<&'a ()>,
        }
    }

    pub unsafe fn as_ptr(&self) -> *mut T {
        self.ptr.as_ptr()
    }

    pub unsafe fn as_nonnull(&self) -> NonNull<T> {
        self.ptr
    }
}
