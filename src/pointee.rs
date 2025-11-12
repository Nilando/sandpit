use super::trace::Trace;
use crate::header::{GcHeader, SizedHeader, SliceHeader};

use alloc::alloc::Layout;
use core::marker::PhantomData;
use core::ptr::{slice_from_raw_parts, NonNull};

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
        let fat_ptr = Self::as_fat(thin_ptr);
        debug_assert!(!fat_ptr.is_null(), "Attempting to dereference null pointer");
        unsafe { &*fat_ptr }
    }
    fn get_header<'a>(thin_ptr: NonNull<Thin<Self>>) -> &'a Self::GcHeader {
        unsafe { &*Self::get_header_ptr(thin_ptr) }
    }
    fn get_header_ptr(thin_ptr: NonNull<Thin<Self>>) -> *const Self::GcHeader;
}

impl<T: Trace> GcPointee for T {
    type GcHeader = SizedHeader<T>;

    fn as_fat<'a>(thin_ptr: NonNull<Thin<Self>>) -> *const Self {
        let ptr = thin_ptr.cast().as_ptr();
        debug_assert!(
            ptr as usize % core::mem::align_of::<T>() == 0,
            "Pointer {:p} is not aligned to {} bytes (required for type {})",
            ptr,
            core::mem::align_of::<T>(),
            core::any::type_name::<T>()
        );
        ptr
    }

    fn get_header_ptr(thin_ptr: NonNull<Thin<Self>>) -> *const Self::GcHeader {
        let (_, item_offset) = sized_alloc_layout::<T>();

        let header_ptr =
            unsafe { thin_ptr.as_ptr().byte_sub(item_offset) as *const Self::GcHeader };
        debug_assert!(
            header_ptr as usize % core::mem::align_of::<Self::GcHeader>() == 0,
            "Header pointer {:p} is not aligned to {} bytes (required for SizedHeader<{}>)",
            header_ptr,
            core::mem::align_of::<Self::GcHeader>(),
            core::any::type_name::<T>()
        );
        header_ptr
    }
}

impl<T: Trace> GcPointee for [T] {
    type GcHeader = SliceHeader<T>;

    fn as_fat<'a>(thin_ptr: NonNull<Thin<Self>>) -> *const Self {
        let slice_ptr: *const T = thin_ptr.cast().as_ptr();
        debug_assert!(
            slice_ptr as usize % core::mem::align_of::<T>() == 0,
            "Slice pointer {:p} is not aligned to {} bytes (required for [{}])",
            slice_ptr,
            core::mem::align_of::<T>(),
            core::any::type_name::<T>()
        );

        let header: &SliceHeader<T> = Self::get_header(thin_ptr);
        let len = header.len();

        slice_from_raw_parts(slice_ptr, len)
    }

    fn get_header_ptr(thin_ptr: NonNull<Thin<Self>>) -> *const Self::GcHeader {
        // we can just pretend the array has a length of 1 here, doesn't effect the offset
        let (_, item_offset) = slice_alloc_layout::<T>(1);

        let ptr: *mut Self::GcHeader = thin_ptr.cast().as_ptr();
        let header_ptr = unsafe { ptr.byte_sub(item_offset) as *const Self::GcHeader };

        debug_assert!(
            header_ptr as usize % core::mem::align_of::<Self::GcHeader>() == 0,
            "Header pointer {:p} is not aligned to {} bytes (required for SliceHeader<{}>)",
            header_ptr,
            core::mem::align_of::<Self::GcHeader>(),
            core::any::type_name::<T>()
        );
        header_ptr
    }
}

pub fn sized_alloc_layout<T>() -> (Layout, usize) {
    let header_layout = Layout::new::<SizedHeader<T>>();
    let val_layout = Layout::new::<T>();
    let (unpadded_layout, offset) = header_layout.extend(val_layout).unwrap();
    let layout = unpadded_layout.pad_to_align();

    (layout, offset)
}

pub fn slice_alloc_layout<T>(len: usize) -> (Layout, usize) {
    let header_layout = Layout::new::<SliceHeader<T>>();
    let slice_layout = Layout::array::<T>(len).unwrap();
    let (unpadded_layout, offset) = header_layout.extend(slice_layout).unwrap();
    let layout = unpadded_layout.pad_to_align();

    (layout, offset)
}
