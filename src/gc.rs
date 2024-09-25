use super::trace::Trace;
use crate::mutator::Mutator;
use crate::header::{GcHeader, SliceHeader, SizedHeader};

use std::alloc::Layout;
use std::marker::PhantomData;
use std::ops::Deref;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::ptr::null_mut;

// Gc pointer types are capable of pointing to [T]. However, you can not
// have an AtomicPtr<T> where T: ?Sized b/c atomic operations can only operate
// on the word size of the machine, which is smaller than the size of a fat pointer.
//
// So in order to support the atomic operations on a pointer to [T],
// you can hold a `*const Thin<[T]>` (which is a thin pointer).
// However, due to being a thin pointer, the length of the [T] must be stored
// somewhere else. So the length of a Gc<[T]> is stored in the header to the [T].
pub struct Thin<T: ?Sized> {
    kind: PhantomData<T>,
}

// The two basic kinda of GcPointee's are T and [T] where T: Sized.
// Due to the usage of thin pointers, the length of [T], needs
// to be stored in the header.
pub trait GcPointee {
    type GcHeader: GcHeader;

    fn deref<'a>(ptr: *mut Thin<Self>) -> &'a Self;
    fn get_header<'a>(ptr: *mut Thin<Self>) -> &'a Self::GcHeader;
}

impl<T: Trace> GcPointee for T {
    type GcHeader = SizedHeader;

    fn deref<'a>(ptr: *mut Thin<Self>) -> &'a Self {
        unsafe { &*(ptr as *mut Self) }
    }

    fn get_header<'a>(ptr: *mut Thin<Self>) -> &'a Self::GcHeader {
        let header_layout = Layout::new::<SizedHeader>();
        let item_layout = Layout::new::<T>();

        // Unwrap safe b/c layout has already been validated during alloc.
        let (_, item_offset) = header_layout.extend(item_layout).unwrap();

        unsafe {
            let header_ptr = ptr.byte_sub(item_offset) as *mut Self::GcHeader;

            &*header_ptr
        }
    }
}

impl<T: Trace> GcPointee for [T] {
    type GcHeader = SliceHeader;

    fn deref<'a>(ptr: *mut Thin<Self>) -> &'a Self {
        let header: &SliceHeader = Self::get_header(ptr);
        let len = header.len();

        // SAFETY: the length of the slice is stored in the SliceHeader.
        unsafe { &*std::ptr::slice_from_raw_parts(ptr as *mut T, len) }
    }

    fn get_header<'a>(ptr: *mut Thin<Self>) -> &'a Self::GcHeader {
        let header_layout = Layout::new::<SliceHeader>();
        // note: item_layout might not be the same layout used to alloc [T], but should 
        // still be fine in calculating the offset needed to get to the header.
        let item_layout = Layout::new::<T>();

        // Unwrap safe b/c layout has already been validated during alloc.
        let (_, item_offset) = header_layout.extend(item_layout).unwrap();

        unsafe {
            let header_ptr = ptr.byte_sub(item_offset) as *mut Self::GcHeader;

            &*header_ptr
        }
    }
}

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
pub struct Gc<'gc, T: Trace + ?Sized> {
    ptr: &'gc T,
}

impl<'gc, T: Trace + ?Sized> Copy for Gc<'gc, T> {}

impl<'gc, T: Trace + ?Sized> Clone for Gc<'gc, T> {
    fn clone(&self) -> Self {
        Self { ptr: self.ptr }
    }
}

impl<'gc, T: Trace + ?Sized> From<GcMut<'gc, T>> for Gc<'gc, T> {
    fn from(gc_mut: GcMut<'gc, T>) -> Self {
        let thin = gc_mut.ptr.load(Ordering::SeqCst);
        
        Self {
            ptr: <T as GcPointee>::deref(thin)
        }
    }
}

impl<'gc, T: Trace + ?Sized> Deref for Gc<'gc, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.ptr
    }
}

impl<'gc, T: Trace> Gc<'gc, T> {
    pub(crate) fn get_layout(&self) -> Layout {
        <T as GcPointee>::get_header(self.as_thin()).get_alloc_layout::<T>()
    }
}

impl<'gc, T: Trace> Gc<'gc, [T]> {
    pub(crate) fn get_layout(&self) -> Layout {
        <[T] as GcPointee>::get_header(self.as_thin()).get_alloc_layout::<T>()
    }
}

impl<'gc, T: Trace + ?Sized> Gc<'gc, T> {
    // SAFETY: the pointer must have a valid GcHeader for T, and be allocated
    // within a GC Arena
    pub(crate) unsafe fn from_ptr(ptr: *const T) -> Self {
        Self { ptr: &*ptr }
    }

    pub(crate) fn get_header(&self) -> &<T as GcPointee>::GcHeader {
        <T as GcPointee>::get_header(self.as_thin())
    }


    pub fn as_ptr(&self) -> *mut T {
        self.ptr as *const T as *mut T
    }

    fn as_thin(&self) -> *mut Thin<T> {
        self.ptr as *const T as *const Thin<T> as *mut Thin<T>
    }

    pub fn scoped_deref(&self) -> &'gc T {
        self.ptr
    }
}

impl<'gc, T: Trace> Gc<'gc, T> {
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

impl<'gc, T: Trace> Deref for GcMut<'gc, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        let thin_ptr = self.ptr.load(Ordering::Acquire);

        <T as GcPointee>::deref(thin_ptr)
    }
}

impl<'gc, T: Trace> Deref for GcMut<'gc, [T]> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        let thin_ptr = self.ptr.load(Ordering::Acquire);

        <[T] as GcPointee>::deref(thin_ptr)
    }
}

impl<'gc, T: Trace + ?Sized> From<Gc<'gc, T>> for GcMut<'gc, T> {
    fn from(gc: Gc<'gc, T>) -> Self {
        Self {
            ptr: AtomicPtr::new(gc.as_thin()),
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
        let thin_ptr = new_gc.into().as_thin();

        self.ptr.store(thin_ptr, Ordering::Release);
    }

    pub fn scoped_deref(&self) -> &'gc T {
        let thin_ptr = self.ptr.load(Ordering::Acquire);

        <T as GcPointee>::deref(thin_ptr)
    }
}

pub struct GcNullMut<'gc, T: Trace + ?Sized> {
    ptr: AtomicPtr<Thin<T>>,
    scope: PhantomData<&'gc *mut T>,
}

impl<'gc, T: Trace + ?Sized> From<Gc<'gc, T>> for GcNullMut<'gc, T> {
    fn from(gc: Gc<'gc, T>) -> Self {
        Self {
            ptr: AtomicPtr::new(gc.as_thin()),
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

        self.ptr.store(thin_ptr, Ordering::Release);
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
                    ptr: AtomicPtr::new(self.ptr.load(Ordering::SeqCst)),
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
    use crate::{field, Arena, Root, Trace};
    use crate::header::GcMark;

    #[test]
    fn valid_sized_header() {
        let _: Arena<Root![_]> = Arena::new(|mu| {
            let gc = Gc::new(mu, 69); 
            let header = gc.get_header();

            assert!(*gc == 69);
            assert_eq!(header.get_mark(), GcMark::New);
            header.set_mark(GcMark::Red);
            assert_eq!(header.get_mark(), GcMark::Red);
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
            assert_eq!(header.get_mark(), GcMark::New);
        });
    }
}
