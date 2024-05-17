use super::allocate::Marker;
use super::block_meta::BlockMeta;
use super::size_class::SizeClass;
use std::sync::atomic::{AtomicU8, Ordering};
use std::mem::{align_of, size_of};
use std::ptr::NonNull;

#[repr(u8)]
#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub enum Mark {
    New,

    // these are the marks that rotate
    Red,
    Green,
    Blue,
}

impl Marker for Mark {
    fn new() -> Self {
        Self::New
    }

    fn is_new(&self) -> bool {
        *self == Self::New
    }
}

impl Mark {
    pub fn rotate(&self) -> Self {
        match self {
            Mark::Red => Mark::Green,
            Mark::Green => Mark::Blue,
            Mark::Blue => Mark::Red,
            _ => panic!("Attempted to rotate a mark that shouldn't be rotated"),
        }
    }
}

impl From<u8> for Mark {
    fn from(value: u8) -> Self {
        match value {
            x if x == Mark::New as u8 => Mark::New,

            x if x == Mark::Red as u8 => Mark::Red,
            x if x == Mark::Green as u8 => Mark::Green,
            x if x == Mark::Blue as u8 => Mark::Blue,
            _ => panic!("Bad GC Mark"),
        }
    }
}

#[repr(C)]
pub struct Header {
    mark: AtomicU8,
    size_class: SizeClass,
    size: u16,
}

impl Header {
    pub fn new(size_class: SizeClass, size: u16) -> Self {
        Header {
            mark: AtomicU8::new(Mark::New as u8),
            size_class,
            size,
        }
    }

    pub fn get_mark(ptr: *const Header) -> Mark {
        let mark_ptr = ptr as *const AtomicU8; // safe b/c repr C
        unsafe { (*mark_ptr).load(Ordering::SeqCst).into() }
    }

    pub fn get_size_class(&self) -> SizeClass {
        self.size_class
    }

    pub fn get_size(&self) -> u16 {
        self.size
    }

    pub fn mark_new(ptr: *const Header) {
        let mark_ptr = ptr as *const AtomicU8; // safe b/c repr C
        unsafe { (*mark_ptr).store(Mark::New as u8, Ordering::SeqCst) }
    }

    pub fn set_mark(ptr: *const Header, mark: Mark) {
        let self_ref = unsafe { &*ptr };

        self_ref.mark.store(mark as u8, Ordering::SeqCst);

        if self_ref.size_class != SizeClass::Large {
            let meta = BlockMeta::from_header(ptr);

            meta.mark(self_ref, mark);
        }
    }

    pub fn debug<T>(header: *const Header, ptr: NonNull<T>) -> bool {
        unsafe {
            let align = std::cmp::max(align_of::<Header>(), align_of::<T>());
            let header_size = size_of::<Header>();
            let padding = (align - (header_size % align)) % align;
            let alloc_size = header_size + padding + size_of::<T>();
            let size_class = SizeClass::get_for_size(alloc_size).unwrap();
            let header_ref = &*header;

            if size_class != SizeClass::Large {
                debug_assert_eq!(header_ref.get_size() as usize, alloc_size);
            }

            debug_assert_eq!(header_ref.get_size_class(), size_class);

            true
        }
    }
}
