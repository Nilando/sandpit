use super::allocate::Marker;
use super::block_meta::BlockMeta;
use super::size_class::SizeClass;
use std::sync::atomic::{AtomicU8, Ordering};

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

    pub fn get_mark(this: *const Header) -> Mark {
        unsafe { (*this).mark.load(Ordering::Acquire).into() }
    }

    pub fn get_size_class(&self) -> SizeClass {
        self.size_class
    }

    pub fn get_size(&self) -> u16 {
        self.size
    }

    /*
    pub fn mark_new(&self) {
        self.mark.store(Mark::New as u8, Ordering::SeqCst)
    }
    */

    pub fn set_mark(this: *const Header, mark: Mark) {
        unsafe {
            (*this).mark.store(mark as u8, Ordering::Release);

            if mark != Mark::New && (*this).size_class != SizeClass::Large {
                let meta = BlockMeta::from_header(this);

                meta.mark(this, mark);
            }
        }
    }
}
