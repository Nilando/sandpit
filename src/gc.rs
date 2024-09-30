use super::trace::Trace;
use crate::mutator::Mutator;
use crate::header::GcHeader;
use crate::barrier::WriteBarrier;
use crate::pointee::{GcPointee, Thin};

use std::alloc::Layout;
use std::marker::PhantomData;
use std::ops::Deref;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::ptr::{null_mut, NonNull};


// A Gc points to a valid T within a GC Arena which is also succeeded by its 
// GC header which may or may not be padded.
// This holds true for GcMut as well as GcNullMut if it is not null.
//
//                                         Gc<T>
//                                          |
//                                          V
// [ <T as GcPointee>::GcHeader ][ padding ][ T value ]
//
// Since Gc cannot be mutated and therefore has no need to be atomic, 
// it is able to be a wide pointer.


/// A shared reference to generic garbage collected value that is branded with 
/// a mutation context lifetime. 
///
/// An object may only be referenced by a [`Gc`] if it implements `Trace` see [`crate::trace::Trace`] for more details.
///
/// A [`Gc`] can safely dereference into 
/// a `&'gc T`, but provides no option to obtain mutable references to it's
/// inner value. Due to all GC values sharing the same 'gc lifetime,
/// any number of GC values are allowed to reference each other at anytime. This
/// is beneficial in easing the creation of graphs and cyclical data structures,
/// but means any mutation of a GC value requires some form of interior mutatbility.
///
/// A [`Gc`] is itself immutable in that it's inner pointer may never be
/// changed. The [`GcMut`] and [`GcNullMut`] types allow for updating
/// which value it is referencing through the means of a write barrier.
/// A [`Gc`] may also point at a garbage collected array like `Gc<'gc, [T]>`. A Gc referencing an
/// array can be obtained via the mutator by using one of several array allocation methods
/// including [`crate::mutator::Mutator::alloc_array`].
pub struct Gc<'gc, T: Trace + ?Sized> {
    ptr: *mut T,
    _no_send: PhantomData<&'gc T>
}

impl<'gc, T: Trace + ?Sized> Copy for Gc<'gc, T> {}

impl<'gc, T: Trace + ?Sized> Clone for Gc<'gc, T> {
    fn clone(&self) -> Self {
        Self { 
            ptr: self.ptr,
            _no_send: PhantomData::<&'gc T>
        }
    }
}

impl<'gc, T: Trace + ?Sized> From<GcMut<'gc, T>> for Gc<'gc, T> {
    fn from(gc_mut: GcMut<'gc, T>) -> Self {
        let thin = gc_mut.ptr.load(Ordering::Relaxed);
        
        Self {
            ptr: <T as GcPointee>::as_fat(NonNull::new(thin).unwrap()) as *mut T,
            _no_send: PhantomData::<&'gc T>
        }
    }
}

impl<'gc, T: Trace + ?Sized> Deref for Gc<'gc, T> {
    type Target = T;

    /// Get a reference to a garbage collected value.
    /// The lifetime of the reference is restricted to the lifetime of the `&'a Gc<_>`
    /// but the by using [`crate::gc::Gc::scoped_deref`] the lifetime of the reference can be
    /// extended to be that of the entire mutation context.
    ///
    /// May either dereference into a `&'gc [T]` or a sized `&'gc T`. 
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.ptr }
    }
}

impl<'gc, T: Trace + ?Sized> Gc<'gc, T> {
    pub(crate) fn get_layout(&self) -> Layout {
        <T as GcPointee>::get_header(self.as_thin()).get_alloc_layout()
    }
}

impl<'gc, T: Trace + ?Sized> Gc<'gc, T> {
    /// Get a reference to a garabage collected value with the lifetime of the mutation.
    pub fn scoped_deref(&self) -> &'gc T {
        unsafe { &*self.ptr }
    }
    // SAFETY: the pointer must have a valid GcHeader for T, and be allocated
    // within a GC Arena
    pub(crate) unsafe fn from_ptr(ptr: *const T) -> Self {
        Self { 
            ptr: ptr as *mut T,
            _no_send: PhantomData::<&'gc T>
        }
    }

    pub(crate) fn get_header(&self) -> &<T as GcPointee>::GcHeader {
        <T as GcPointee>::get_header(self.as_thin())
    }

    // THIS EXIST FOR PROVENANCE DON't REMOVE
    pub(crate) fn get_header_ptr(&self) -> *const <T as GcPointee>::GcHeader {
        <T as GcPointee>::get_header_ptr(self.as_thin())
    }

    pub(crate) fn as_thin(&self) -> NonNull<Thin<T>> {
        NonNull::new(self.ptr as *const T as *const Thin<T> as *mut Thin<T>).unwrap()
    }

    pub fn write_barrier<F>(&self, mu: &'gc Mutator, f: F) 
    where
        F: FnOnce(&WriteBarrier<T>)
    {
        let barrier = unsafe { WriteBarrier::new(&**self) };

        f(&barrier);

        mu.retrace(*self);
    }
}

impl<'gc, T: Trace> Gc<'gc, T> {
    /// Provides a way to allocate a value into the GC arena, returning a `Gc<T>`.
    /// This method is equivalent to calling [`crate::mutator::Mutator::alloc`].
    pub fn new(m: &'gc Mutator<'gc>, obj: T) -> Self {
        m.alloc(obj)
    }
}

// GcMut may be updated to point somewhere else which requires it to be atomic 
// in order to sync with the tracing threads.
pub struct GcMut<'gc, T: Trace + ?Sized> {
    ptr: AtomicPtr<Thin<T>>,
    scope: PhantomData<&'gc *mut T>,
}

impl<'gc, T: Trace + ?Sized> Deref for GcMut<'gc, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        let thin_ptr = self.ptr.load(Ordering::Relaxed);

        <T as GcPointee>::deref(NonNull::new(thin_ptr).unwrap())
    }
}

impl<'gc, T: Trace + ?Sized> From<Gc<'gc, T>> for GcMut<'gc, T> {
    fn from(gc: Gc<'gc, T>) -> Self {
        Self {
            ptr: AtomicPtr::new(gc.as_thin().as_ptr()),
            scope: PhantomData::<&'gc *mut T>
        }
    }
}

impl<'gc, T: Trace + ?Sized> Clone for GcMut<'gc, T> {
    fn clone(&self) -> Self {
        let ptr = self.ptr.load(Ordering::Relaxed);

        Self {
            ptr: AtomicPtr::new(ptr),
            scope: PhantomData::<&'gc *mut T>,
        }
    }
}

impl<'gc, T: Trace> GcMut<'gc, T> {
    pub fn new(m: &'gc Mutator<'gc>, obj: T) -> Self {
        m.alloc(obj).into()
    }
}

impl<'gc, T: Trace + ?Sized> GcMut<'gc, T> {
    pub(crate) unsafe fn set(&self, new_gc: impl Into<Gc<'gc, T>>) {
        let thin_ptr = new_gc.into().as_thin().as_ptr();

        self.ptr.store(thin_ptr, Ordering::Relaxed);
    }

    pub fn scoped_deref(&self) -> &'gc T {
        let thin_ptr = self.ptr.load(Ordering::Relaxed);

        <T as GcPointee>::deref(NonNull::new(thin_ptr).unwrap())
    }
}

pub struct GcNullMut<'gc, T: Trace + ?Sized> {
    ptr: AtomicPtr<Thin<T>>,
    scope: PhantomData<&'gc *mut T>,
}

impl<'gc, T: Trace + ?Sized> From<Gc<'gc, T>> for GcNullMut<'gc, T> {
    fn from(gc: Gc<'gc, T>) -> Self {
        Self {
            ptr: AtomicPtr::new(gc.as_thin().as_ptr()),
            scope: PhantomData::<&'gc *mut T>,
        }
    }
}

impl<'gc, T: Trace + ?Sized> From<GcMut<'gc, T>> for GcNullMut<'gc, T> {
    fn from(gc: GcMut<'gc, T>) -> Self {
        Self {
            ptr: AtomicPtr::new(gc.ptr.load(Ordering::Relaxed)),
            scope: PhantomData::<&'gc *mut T>,
        }
    }
}

impl<'gc, T: Trace + ?Sized> Clone for GcNullMut<'gc, T> {
    fn clone(&self) -> Self {
        Self {
            ptr: AtomicPtr::new(self.ptr.load(Ordering::Relaxed)),
            scope: PhantomData::<&'gc *mut T>,
        }
    }
}

impl<'gc, T: Trace + ?Sized> GcNullMut<'gc, T> {
    pub fn new_null(_m: &'gc Mutator<'gc>) -> Self {
        Self {
            ptr: AtomicPtr::new(null_mut()),
            scope: PhantomData::<&'gc *mut T>,
        }
    }

    pub fn is_null(&self) -> bool {
        self.ptr.load(Ordering::Relaxed).is_null()
    }

    // If the tracers have already traced this pointer, than the new pointer
    // must be retraced before the end of the mutation context.
    //
    // Use a write barrier to call this method safely.
    pub(crate) unsafe fn set(&self, new: GcNullMut<'gc, T>) {
        let thin_ptr = new.ptr.load(Ordering::Relaxed);

        self.ptr.store(thin_ptr, Ordering::Relaxed);
    }

    // safe because setting to null doesn't require anything to be retraced!
    pub fn set_null(&self) {
        self.ptr.store(null_mut(), Ordering::Relaxed)
    }

    pub fn as_option(&self) -> Option<GcMut<'gc, T>> {
        if self.is_null() {
            None
        } else {
            Some(
                GcMut {
                    ptr: AtomicPtr::new(self.ptr.load(Ordering::Relaxed)),
                    scope: PhantomData::<&'gc *mut T>,
                }
            )
        }
    }
}

impl<'gc, T: Trace> GcNullMut<'gc, T> {
    pub fn new(m: &'gc Mutator<'gc>, obj: T) -> Self {
        m.alloc(obj).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Arena, Root};
    use crate::header::GcMark;

    #[test]
    fn valid_sized_header() {
        let _: Arena<Root![_]> = Arena::new(|mu| {
            let gc = Gc::new(mu, 69); 
            let header = gc.get_header();

            assert!(*gc == 69);
            assert_eq!(header.get_mark(), GcMark::Blue);
            header.set_mark(GcMark::Green);
            assert_eq!(header.get_mark(), GcMark::Green);
            assert!(*gc == 69);
        });
    }

    #[test]
    fn gc_from_gcmut() {
        let _: Arena<Root![_]> = Arena::new(|mu| {
            let gc = Gc::new(mu, 69); 
            let gc_mut = GcMut::from(gc);
            let gc = Gc::from(gc_mut);
            let header = gc.get_header();

            assert!(*gc == 69);
            assert_eq!(header.get_mark(), GcMark::Blue);
        });
    }
}
