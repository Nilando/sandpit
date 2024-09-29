use super::trace::Trace;
use crate::header::{GcHeader, SliceHeader, SizedHeader};

use std::alloc::Layout;
use std::marker::PhantomData;
use std::ptr::NonNull;

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

// The two basic kinds of GcPointee's are T and [T] where T: Sized.
// Due to the usage of thin pointers, the length of [T], needs
// to be stored in the header.
pub trait GcPointee {
    type GcHeader: GcHeader;

    fn as_fat<'a>(thin_ptr: NonNull<Thin<Self>>) -> *const Self;
    fn deref<'a>(thin_ptr: NonNull<Thin<Self>>) -> &'a Self {
        unsafe { &*Self::as_fat(thin_ptr) }
    }
    fn get_header<'a>(thin_ptr: NonNull<Thin<Self>>) -> &'a Self::GcHeader {
        unsafe { &*Self::get_header_ptr(thin_ptr) }
    }
    fn get_header_ptr(thin_ptr: NonNull<Thin<Self>>) -> *const Self::GcHeader;
}

impl<T: Trace> GcPointee for T {
    type GcHeader = SizedHeader<T>;

    fn as_fat<'a>(thin_ptr: NonNull<Thin<Self>>) -> *const Self {
        thin_ptr.cast().as_ptr()
    }

    fn get_header_ptr(thin_ptr: NonNull<Thin<Self>>) -> *const Self::GcHeader {
        let header_layout = Layout::new::<SizedHeader<T>>();
        let item_layout = Layout::new::<T>();

        // Unwrap safe b/c layout has already been validated during alloc.
        let (_, item_offset) = header_layout.extend(item_layout).unwrap();

        unsafe { thin_ptr.as_ptr().byte_sub(item_offset) as *const Self::GcHeader }
    }
}

impl<T: Trace> GcPointee for [T] {
    type GcHeader = SliceHeader<T>;

    fn as_fat<'a>(thin_ptr: NonNull<Thin<Self>>) -> *const Self {
        let header: &SliceHeader<T> = Self::get_header(thin_ptr);
        let len = header.len();

        std::ptr::slice_from_raw_parts(thin_ptr.cast().as_ptr(), len)
    }

    fn get_header_ptr(thin_ptr: NonNull<Thin<Self>>) -> *const Self::GcHeader {
        let header_layout = Layout::new::<SliceHeader<T>>();
        // note: item_layout might not be the same layout used to alloc [T], but should 
        // still be fine in calculating the offset needed to get to the header.
        let item_layout = Layout::new::<T>();

        // Unwrap safe b/c layout has already been validated during alloc.
        let (_, item_offset) = header_layout.extend(item_layout).unwrap();
        let ptr: *mut Self::GcHeader = thin_ptr.cast().as_ptr();

        unsafe { ptr.byte_sub(item_offset) as *const Self::GcHeader }
    }
}

