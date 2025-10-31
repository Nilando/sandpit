//! Module containing the three types of GC pointers.
//!
//! ## Overview
//! * An object may only be referenced by a GC pointer if it implements [`crate::Trace`].
//! * Any object capable of containing a GC pointer may not impl [`crate::TraceLeaf`].
//!
//! [`Gc`] and [`GcOpt`] may be updated via a [`crate::WriteBarrier`] to point
//! at different values.
use super::trace::Trace;
use crate::barrier::WriteBarrier;
use crate::header::GcHeader;
use crate::mutator::Mutator;
use crate::pointee::{GcPointee, Thin};

use alloc::alloc::Layout;
use core::marker::PhantomData;
use core::ops::Deref;
use core::ptr::{null_mut, NonNull};
use core::sync::atomic::{AtomicPtr, Ordering};

// A Gc points to a valid T within a GC Arena which is also succeeded by its
// GC header which may or may not be padded.
// This holds true for Gc as well as GcOpt if it is not null.
//
//                                         Gc<T>
//                                          |
//                                          V
// [ <T as GcPointee>::GcHeader ][ padding ][ T value ]
//
// Since Gc cannot be mutated and therefore has no need to be atomic,
// it is able to be a wide pointer.

/// Immutable Gc pointer, meaning a write barrier cannot be used to update this pointer.
///
/// It's inner value can still be mutated.
///
/// A [`Gc`] can safely dereference into
/// a `&'gc T`, but provides no option to obtain mutable references to it's
/// inner value. Due to all GC values sharing the same 'gc lifetime,
/// any number of GC values are allowed to reference each other at anytime. This
/// is beneficial in easing the creation of graphs and cyclical data structures,
/// but means any mutation of a GC value requires some form of interior mutatbility.
///
/// A [`Gc`] is itself immutable in that it's inner pointer may never be
/// changed. The [`Gc`] and [`GcOpt`] types allow for updating
/// which value it is referencing through the means of a write barrier.
/// A [`Gc`] may also point at a garbage collected array like `Gc<'gc, [T]>`. A Gc referencing an
/// array can be obtained via the mutator by using one of several array allocation methods
/// including [`Mutator::alloc_array`].

// Gc may be updated to point somewhere else which requires it to be atomic
// in order to sync with the tracing threads.

/// Mutable GC pointer, meaning a write barrier can be used to update this pointer.
///
/// See [`crate::barrier::WriteBarrier`] for how to update a [`Gc`].
pub struct Gc<'gc, T: Trace + ?Sized> {
    ptr: AtomicPtr<Thin<T>>,
    scope: PhantomData<&'gc *mut T>,
}

impl<'gc, T: Trace> Gc<'gc, T> {
    /// Provides a way to allocate a value into the GC arena, returning a `Gc<T>`.
    ///
    /// This method is equivalent to calling [`crate::mutator::Mutator::alloc`].
    ///
    /// # Example
    /// ```rust
    /// # use sandpit::{Arena, Gc, Root};
    /// # let arena: Arena<Root![()]> = Arena::new(|mu| ());
    /// arena.mutate(|mu, root| {
    ///    let new = Gc::new(mu, 69);
    /// });
    pub fn new(m: &'gc Mutator<'gc>, obj: T) -> Self {
        m.alloc(obj)
    }
}

impl<'gc, T: Trace + ?Sized> Gc<'gc, T> {
    pub(crate) fn get_layout(&self) -> Layout {
        <T as GcPointee>::get_header(self.as_thin()).get_alloc_layout()
    }
}

impl<'gc, T: Trace + ?Sized + 'gc> Deref for Gc<'gc, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        let thin_ptr = self.ptr.load(Ordering::Relaxed);

        <T as GcPointee>::deref(NonNull::new(thin_ptr).unwrap())
    }
}

impl<'gc, T: Trace + ?Sized> Clone for Gc<'gc, T> {
    fn clone(&self) -> Self {
        let ptr = self.ptr.load(Ordering::Relaxed);

        Self {
            ptr: AtomicPtr::new(ptr),
            scope: PhantomData::<&'gc *mut T>,
        }
    }
}

impl<'gc, T: Trace + ?Sized> Gc<'gc, T> {
    pub(crate) unsafe fn set(&self, value: Gc<'gc, T>) {
        let thin_ptr = value.ptr.load(Ordering::Relaxed);

        self.ptr.store(thin_ptr, Ordering::Relaxed);
    }

    // SAFETY: the pointer must have a valid GcHeader for T, and be allocated
    // within a GC Arena
    pub(crate) unsafe fn from_ptr(ptr: *const T) -> Self {
        Self {
            ptr: AtomicPtr::new(ptr as *mut T as *mut Thin<T>),
            scope: PhantomData::<&'gc *mut T>,
        }
    }

    pub(crate) fn get_header(&self) -> &<T as GcPointee>::GcHeader {
        <T as GcPointee>::get_header(self.as_thin())
    }

    // HACK: THIS EXIST FOR PROVENANCE
    pub(crate) fn get_header_ptr(&self) -> *const <T as GcPointee>::GcHeader {
        <T as GcPointee>::get_header_ptr(self.as_thin())
    }

    pub(crate)  fn as_thin(&self) -> NonNull<Thin<T>> {
        unsafe {
            NonNull::new_unchecked(self.ptr.load(Ordering::Relaxed))
        }
    }

    /// Get a reference to a garabage collected value with the lifetime of the mutation.
    ///
    /// Becuase all Gc pointers point to values valid for the entire mutation
    /// lifetime, it is fine to dereference them with that lifetime.
    ///
    /// A regular deref of a `Gc<'gc, T>` gives `&'a T` where `'a` is the lifetime
    /// of the pointer.
    ///
    /// # Example
    ///
    /// In this example scoded deref is needed to implement Foo's set_inner method.
    ///
    ///```rust
    /// # use sandpit::{Arena, Root, Gc};
    /// let arena: Arena<Root![Gc<'_, usize>]> = Arena::new(|mu| Gc::new(mu, 69));
    ///
    /// arena.mutate(|mu, root| {
    ///     struct Foo<'gc> {
    ///         inner: &'gc usize
    ///     }
    ///
    ///     impl<'gc> Foo<'gc> {
    ///         fn set_inner(&mut self, gc: Gc<'gc, usize>) {
    ///             // DOES NOT COMPILE 
    ///             // self.inner = &gc;
    ///             self.inner = &gc.scoped_deref();
    ///         }
    ///     }
    ///
    ///     let mut foo = Foo {
    ///         inner: root.scoped_deref()
    ///     };
    ///
    ///     let gc = Gc::new(mu, 2);
    ///
    ///     foo.set_inner(gc);
    /// });
    ///```
    pub fn scoped_deref(&self) -> &'gc T {
        let thin_ptr = self.ptr.load(Ordering::Relaxed);

        <T as GcPointee>::deref(NonNull::new(thin_ptr).unwrap())
    }

    /// Allows for updating internal `Gc`'s and `GcOpt`'s.
    ///
    /// Returns a reference to the pointed at value that is wrapped in a
    /// [`crate::barrier::WriteBarrier`] which allows for mutating `Gc` and
    /// `GcOpt`'s.
    ///
    /// # Example 
    ///
    /// Get a writer barrier to a index of a slice behind write barrier.
    ///
    /// ## Example
    ///
    /// See [`crate::barrier::WriteBarrier`] for more examples.
    ///
    /// ```rust
    /// use sandpit::{Arena, Gc, Root};
    ///
    /// let arena: Arena<Root![Gc<'_, Gc<'_, usize>>]> = Arena::new(|mu| {
    ///    Gc::new(mu, Gc::new(mu, 69))
    /// });
    ///
    /// arena.mutate(|mu, root| {
    ///     root.write_barrier(mu, |barrier| {
    ///         let new_value = Gc::new(mu, 420);
    ///
    ///         barrier.set(new_value);
    ///
    ///         assert!(**barrier.inner() == 420);
    ///     });
    /// });
    ///```
    pub fn write_barrier<F>(&self, mu: &'gc Mutator, f: F)
    where
        F: FnOnce(&WriteBarrier<T>),
    {
        // SAFETY: Its safe to create a writebarrier over this pointer b/c it is guaranteed
        // to be retraced after the closure ends.
        let barrier = unsafe { WriteBarrier::new(self.scoped_deref()) };

        f(&barrier);

        if mu.has_marked(&self) {
            mu.retrace(self.scoped_deref());
        }
    }
}

/// A GC pointer which is able of pointing to null.
///
/// [`GcOpt`] can be unwrapped into a Gc, and can also be updated via
/// a [`crate::barrier::WriteBarrier`].
pub struct GcOpt<'gc, T: Trace + ?Sized> {
    ptr: AtomicPtr<Thin<T>>,
    scope: PhantomData<&'gc *mut T>,
}

impl<'gc, T: Trace + ?Sized> From<Gc<'gc, T>> for GcOpt<'gc, T> {
    fn from(gc: Gc<'gc, T>) -> Self {
        Self {
            ptr: AtomicPtr::new(gc.ptr.load(Ordering::Relaxed)),
            scope: PhantomData::<&'gc *mut T>,
        }
    }
}

impl<'gc, T: Trace + ?Sized> Clone for GcOpt<'gc, T> {
    fn clone(&self) -> Self {
        Self {
            ptr: AtomicPtr::new(self.ptr.load(Ordering::Relaxed)),
            scope: PhantomData::<&'gc *mut T>,
        }
    }
}

impl<'gc, T: Trace + ?Sized> GcOpt<'gc, T> {
    /// Creates a new GcOpt which points to null.
    ///
    /// A GcOpt can also be created from a [`Gc`] or [`Gc`].
    ///
    /// # Example
    /// ```rust
    /// use sandpit::{Arena, GcOpt, Root};
    ///
    /// let arena: Arena<Root![GcOpt<'_, usize>]> = Arena::new(|mu| {
    ///    GcOpt::new_none()
    /// });
    ///```
    pub fn new_none() -> Self {
        Self {
            ptr: AtomicPtr::new(null_mut()),
            scope: PhantomData::<&'gc *mut T>,
        }
    }

    /// Check whether this [`GcOpt`] is null.
    ///
    /// # Example
    /// ```rust
    /// # use sandpit::{Arena, GcOpt, Root};
    /// # let arena: Arena<Root![()]> = Arena::new(|mu| {
    ///    let gc_opt: GcOpt<()> = GcOpt::new_none();
    ///
    ///    assert!(gc_opt.is_none());
    /// # });
    ///```
    pub fn is_none(&self) -> bool {
        self.ptr.load(Ordering::Relaxed).is_null()
    }

    /// Check whether this [`GcOpt`] contains a valid [`Gc`].
    ///
    /// # Example
    /// ```rust
    /// # use sandpit::{Arena, Gc, GcOpt, Root};
    /// # let arena: Arena<Root![()]> = Arena::new(|mu| {
    ///    let gc_opt = GcOpt::from(Gc::new(mu, 123));
    ///
    ///    assert!(gc_opt.is_some());
    /// # });
    ///```
    pub fn is_some(&self) -> bool {
        !self.is_none()
    }

    /// Mutate this [`GcOpt`] so that it is null.
    ///
    /// Normally updating a Gc pointer requires a write barrier, however,
    /// this method is an exception as the null pointer requires no tracing.
    ///
    /// # Example
    /// ```rust
    /// # use sandpit::{Arena, Gc, GcOpt, Root};
    /// # let arena: Arena<Root![()]> = Arena::new(|mu| {
    ///    let gc_opt = GcOpt::from(Gc::new(mu, 123));
    ///
    ///    assert!(gc_opt.is_some());
    ///
    ///    gc_opt.set_none();
    ///
    ///    assert!(gc_opt.is_none());
    /// # });
    ///```
    pub fn set_none(&self) {
        self.ptr.store(null_mut(), Ordering::Relaxed)
    }

    /// Convert into a Option of [`Gc`].
    ///
    /// # Example
    /// ```rust
    /// # use sandpit::{Arena, Gc, GcOpt, Root};
    /// # let arena: Arena<Root![()]> = Arena::new(|mu| {
    ///    let gc_opt = GcOpt::from(Gc::new(mu, 123));
    ///
    ///    let gc_mut = gc_opt.as_option().unwrap();
    ///
    ///    assert!(*gc_mut == 123);
    /// # });
    ///```
    pub fn as_option(&self) -> Option<Gc<'gc, T>> {
        if self.is_some() {
            Some(Gc {
                ptr: AtomicPtr::new(self.ptr.load(Ordering::Relaxed)),
                scope: PhantomData::<&'gc *mut T>,
            })
        } else {
            None
        }
    }

    /// ## Panics
    ///
    /// Panics if it is in the null state
    pub fn unwrap(&self) -> Gc<'gc, T> {
        self.as_option().unwrap()
    }

    // If the tracers have already traced this pointer, than the new pointer
    // must be retraced before the end of the mutation context.
    //
    // Use a write barrier to call this method safely.
    pub(crate) unsafe fn set(&self, new: GcOpt<'gc, T>) {
        let thin_ptr = new.ptr.load(Ordering::Relaxed);

        self.ptr.store(thin_ptr, Ordering::SeqCst);
    }
}

impl<'gc, T: Trace> GcOpt<'gc, T> {
    pub fn new(m: &'gc Mutator<'gc>, obj: T) -> Self {
        m.alloc(obj).into()
    }
}

impl<'gc, T: Trace + ?Sized> GcOpt<'gc, T> {
    pub unsafe fn from_ptr(ptr: *mut T) -> Self {
        Self {
            ptr: AtomicPtr::new(ptr as *mut Thin<T>),
            scope: PhantomData::<&'gc *mut T>,
        }
    }
}

/*
pub trait GcPointer: Trace + Clone {
    const POINTEE_ALIGNMENT: usize;

    unsafe fn set(&self, value: Self);
}

impl<'gc, T: Trace> GcPointer for Gc<'gc, T> {
    const POINTEE_ALIGNMENT: usize = core::mem::align_of::<T>();

    unsafe fn set(&self, value: Self) {
        self.set(value)
    }
}

impl<'gc, T: Trace> GcPointer for GcOpt<'gc, T> {
    const POINTEE_ALIGNMENT: usize = core::mem::align_of::<T>();

    unsafe fn set(&self, value: Self) {
        self.set(value)
    }
}
*/

#[cfg(test)]
mod tests {
    use super::*;
    use crate::header::GcMark;
    use crate::{Arena, Root};

    #[test]
    fn set_header_marker() {
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
    fn gc_from_gcopt() {
        let _: Arena<Root![_]> = Arena::new(|mu| {
            let gc = Gc::new(mu, 69);
            let gc_opt = GcOpt::from(gc);
            let gc = Gc::from(gc_opt.unwrap());
            let header = gc.get_header();

            assert!(*gc == 69);
            assert_eq!(header.get_mark(), GcMark::Blue);
        });
    }

    #[test]
    fn test_gc_sizes() {
        let _: Arena<Root![_]> = Arena::new(|_| {
            assert!(size_of::<Gc<()>>() == size_of::<*const ()>());
            assert!(size_of::<GcOpt<()>>() == size_of::<*const ()>());
        });
    }
}
