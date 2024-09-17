use super::trace::Trace;
use crate::mutator::Mutator;

use std::marker::PhantomData;
use std::ops::Deref;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicPtr, Ordering};

// Gc points to a valid T within a GC Arena which
// and is also succeeded by its GC header (which may or may not be padded).
//
//                                Gc<T>
//                                 |
//                                 V
// [ GC Header ][ ... padding ... ][ T object ]
//
//
// The padding len is determined by a call to `std::alloc::Layout::extend`
// By extending the layout of GC Header with the layout of T.
//
// Gc<T>
// GcMut<T> // can be mutated via fn set, and is atomic in order to sync with tracers
// GcNullMut<T> // may be a null pointer
// GcArray<T>
//
// A GcArray is headed by A DynHeader which includes the layout of the GcArray
// in the header

pub struct Gc<'gc, T: Trace> {
    ptr: &'gc T,
}

impl<'gc, T: Trace> Copy for Gc<'gc, T> {}

impl<'gc, T: Trace> Clone for Gc<'gc, T> {
    fn clone(&self) -> Self {
        Self { ptr: self.ptr }
    }
}

impl<'gc, T: Trace> From<GcMut<'gc, T>> for Gc<'gc, T> {
    fn from(value: GcMut<'gc, T>) -> Self {
        unsafe { Gc::from_nonnull(NonNull::new_unchecked(value.as_ptr())) }
    }
}

impl<'gc, T: Trace> From<Gc<'gc, T>> for *mut T {
    fn from(value: Gc<'gc, T>) -> Self {
        value.ptr as *const T as *mut T
    }
}

impl<'gc, T: Trace> Deref for Gc<'gc, T> {
    type Target = T;

    // safe b/c of 'gc lifetime!
    fn deref(&self) -> &'gc Self::Target {
        self.ptr
    }
}

impl<'gc, T: Trace> Gc<'gc, T> {
    // SAFETY: The NonNull must specifically be a NonNull obtained from
    // the mutator alloc function!
    pub unsafe fn from_nonnull(ptr: NonNull<T>) -> Self {
        Self { ptr: ptr.as_ref() }
    }

    pub fn as_nonnull(&self) -> NonNull<T> {
        self.ptr.into()
    }

    pub fn new(m: &'gc Mutator<'gc>, obj: T) -> Self {
        m.alloc(obj)
    }
}

// GcMut may be updated to point somewhere else
// needs to be atomic to
pub struct GcMut<'gc, T: Trace> {
    ptr: AtomicPtr<T>,
    scope: PhantomData<&'gc *mut T>,
}

impl<'gc, T: Trace> Deref for GcMut<'gc, T> {
    type Target = T;

    // safe b/c of 'gc lifetime!
    fn deref(&self) -> &'gc Self::Target {
        unsafe { &*self.as_ptr() }
    }
}

impl<'gc, T: Trace> From<Gc<'gc, T>> for GcMut<'gc, T> {
    fn from(gc: Gc<'gc, T>) -> Self {
        Self {
            ptr: AtomicPtr::new(gc.into()),
            scope: PhantomData::<&'gc *mut T>,
        }
    }
}

impl<'gc, T: Trace> Clone for GcMut<'gc, T> {
    fn clone(&self) -> Self {
        Self {
            ptr: AtomicPtr::new(self.as_ptr()),
            scope: PhantomData::<&'gc *mut T>,
        }
    }
}

impl<'gc, T: Trace> GcMut<'gc, T> {
    pub unsafe fn from_nonnull(ptr: NonNull<T>) -> Self {
        Self {
            ptr: AtomicPtr::new(ptr.as_ptr()),
            scope: PhantomData::<&'gc *mut T>,
        }
    }

    pub fn new(m: &'gc Mutator<'gc>, obj: T) -> Self {
        m.alloc(obj).into()
    }

    pub fn as_ptr(&self) -> *mut T {
        self.ptr.load(Ordering::SeqCst)
    }

    pub fn as_nonnull(&self) -> NonNull<T> {
        unsafe { NonNull::new_unchecked(self.as_ptr()) }
    }

    pub unsafe fn set(&self, new: impl Into<Gc<'gc, T>>) {
        let gc = new.into();
        self.ptr.store(gc.into(), Ordering::SeqCst)
    }

    pub fn scoped_deref(&self) -> &'gc T {
        unsafe { &*self.as_ptr() }
    }
}

pub struct GcNullMut<'gc, T: Trace> {
    ptr: AtomicPtr<T>,
    scope: PhantomData<&'gc *mut T>,
}

impl<'gc, T: Trace> From<Gc<'gc, T>> for GcNullMut<'gc, T> {
    fn from(gc: Gc<'gc, T>) -> Self {
        Self {
            ptr: AtomicPtr::new(gc.into()),
            scope: PhantomData::<&'gc *mut T>,
        }
    }
}

impl<'gc, T: Trace> Clone for GcNullMut<'gc, T> {
    fn clone(&self) -> Self {
        Self {
            ptr: AtomicPtr::new(self.as_ptr()),
            scope: PhantomData::<&'gc *mut T>,
        }
    }
}

impl<'gc, T: Trace> GcNullMut<'gc, T> {
    pub unsafe fn from_ptr(ptr: *mut T) -> Self {
        Self {
            ptr: AtomicPtr::new(ptr),
            scope: PhantomData::<&'gc *mut T>,
        }
    }

    pub fn new(m: &'gc Mutator<'gc>, obj: T) -> Self {
        m.alloc(obj).into()
    }

    pub fn new_null(_m: &'gc Mutator<'gc>) -> Self {
        unsafe { Self::from_ptr(std::ptr::null_mut()) }
    }

    pub fn as_ptr(&self) -> *mut T {
        self.ptr.load(Ordering::SeqCst)
    }

    pub fn is_null(&self) -> bool {
        self.as_ptr().is_null()
    }

    // If the tracers have already traced this pointer, than the new pointer
    // must be retraced before the end of the mutation context.
    //
    // Use a write barrier to call this method safely.
    pub unsafe fn set(&self, new: GcNullMut<'gc, T>) {
        self.ptr.store(new.as_ptr(), Ordering::SeqCst)
    }

    // safe because setting to null doesn't require anything to be retraced!
    pub fn set_null(&self) {
        self.ptr.store(std::ptr::null_mut(), Ordering::SeqCst)
    }

    pub fn as_option(&self) -> Option<GcMut<'gc, T>> {
        let ptr = self.as_ptr();

        if ptr.is_null() {
            None
        } else {
            unsafe { Some(GcMut::from_nonnull(NonNull::new_unchecked(ptr))) }
        }
    }
}
