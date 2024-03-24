use super::size_class::SizeClass;
use super::constants::aligned_size;

use std::sync::atomic::AtomicBool;

pub struct Header {
    marked: AtomicBool,
    size_class: SizeClass, // includes header size
    size: u16, // includes header size
}

impl Header {
    pub const ALIGNED_SIZE: usize = aligned_size::<Self>();

    pub fn new(size_class: SizeClass, size: u16) -> Self {
        Header {
            marked: AtomicBool::new(false),
            size_class,
            size,
        }
    }
}
