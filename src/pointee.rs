use super::trace::Trace;
use crate::header::{GcHeader, SizedHeader, SliceHeader};

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
        let (_, item_offset) = sized_alloc_layout::<T>();

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
        // we can just pretend the array has a length of 1 here, doesn't effect the offset
        let (_, item_offset) = slice_alloc_layout::<T>(1);

        let ptr: *mut Self::GcHeader = thin_ptr.cast().as_ptr();

        unsafe { ptr.byte_sub(item_offset) as *const Self::GcHeader }
    }
}

pub fn sized_alloc_layout<T>() -> (Layout, usize) {
    let header_layout = Layout::new::<SizedHeader<T>>();
    let val_layout = Layout::new::<T>();
    let (unpadded_layout, offset) = header_layout.extend(val_layout).unwrap_or_else(|err| {
        println!("GC_ERROR (./pointee.rs:74): {err}");
        println!("type: {}", std::any::type_name::<T>());

        std::process::abort();
    });
    let layout = unpadded_layout.pad_to_align();

    (layout, offset)
}

pub fn slice_alloc_layout<T>(len: usize) -> (Layout, usize) {
    let header_layout = Layout::new::<SliceHeader<T>>();
    let slice_layout = Layout::array::<T>(len).unwrap_or_else(|err| {
        println!("GC_ERROR (./pointee.rs:87): {err}");
        println!("type: {}, len: {}", std::any::type_name::<T>(), len);

        std::process::abort();
    });
    let (unpadded_layout, offset) = header_layout.extend(slice_layout).unwrap_or_else(|err| {
        println!("GC_ERROR (./pointee.rs:93): {err}");
        println!("type: {}, len: {}", std::any::type_name::<T>(), len);

        std::process::abort();
    });
    let layout = unpadded_layout.pad_to_align();

    (layout, offset)
}
