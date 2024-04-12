use super::size_class::SizeClass;
use crate::allocate::Marker;
use std::cell::UnsafeCell;

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
            Mark::New => panic!("The new mark cannot be rotated"),
            Mark::Red => Mark::Green,
            Mark::Green => Mark::Blue,
            Mark::Blue => Mark::Red,
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
    mark: UnsafeCell<Mark>,
    size_class: SizeClass,
    size: u16,
}

impl Header {
    pub fn new(size_class: SizeClass, size: u16) -> Self {
        Header {
            mark: UnsafeCell::new(Mark::New),
            size_class,
            size,
        }
    }

    pub fn get_mark(&self) -> Mark {
        unsafe { *self.mark.get() }
    }

    pub fn set_mark(&self, mark: Mark) {
        unsafe { *self.mark.get() = mark }
    }

    pub fn get_size_class(&self) -> SizeClass {
        self.size_class
    }

    pub fn get_size(&self) -> u16 {
        self.size
    }
}
