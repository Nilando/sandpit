use super::size_class::SizeClass;
use super::block_meta::BlockMeta;
use crate::allocate::Marker;
use std::sync::atomic::{Ordering, AtomicU8};

#[repr(u8)]
#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub enum Mark {
    New,
    Red,
    Green,
    Blue,
}

impl Marker for Mark {
    fn is_new(&self) -> bool {
        *self == Mark::New
    }
}

impl Mark {
    pub fn rotate(&self) -> Self {
        match self {
            Mark::Red => Mark::Green,
            Mark::Green => Mark::Blue,
            Mark::Blue => Mark::Red,
            Mark::New => panic!("The new mark cannot be rotated"),
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
            _ => panic!("Bad mark"),
        }
    }
}

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

    pub fn get_mark(&self) -> Mark {
        self.mark.load(Ordering::Relaxed).into()
    }

    pub fn get_size_class(&self) -> SizeClass {
        self.size_class
    }

    pub fn get_size(&self) -> u16 {
        self.size
    }

    pub fn set_mark(&self, mark: Mark) {
        self.mark.store(mark as u8, Ordering::Relaxed);

        if self.size_class != SizeClass::Large {
            let mut meta = BlockMeta::from_header(self);

            meta.mark(self, mark);
        }
    }
}
