use super::size_class::SizeClass;
use std::cell::Cell;

#[repr(u8)]
#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub enum Mark {
    New,
    Red,
    Green,
    Blue,
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
    mark: Cell<Mark>,
    size_class: SizeClass, // includes header size
    size: u16,             // includes header size
}

impl Header {
    pub fn new(size_class: SizeClass, size: u16) -> Self {
        Header {
            mark: Cell::new(Mark::New),
            size_class,
            size,
        }
    }

    pub fn get_mark(&self) -> Mark {
        self.mark.get()
    }

    pub fn set_mark(&self, mark: Mark) {
        self.mark.set(mark)
    }

    pub fn get_size_class(&self) -> SizeClass {
        self.size_class
    }

    pub fn get_size(&self) -> u16 {
        self.size
    }
}
