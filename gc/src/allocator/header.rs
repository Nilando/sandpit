use super::size_class::SizeClass;
use std::sync::atomic::AtomicBool;
use super::constants::aligned_size;

pub struct Header {
    marked: AtomicBool,
    size_class: SizeClass, // includes header size
    size: u16, // includes header size
}

impl Header {
    pub const ALIGNED_SIZE: usize = aligned_size::<Self>();

    pub fn new(size: u16) -> Self {
        Header {
            marked: AtomicBool::new(false),
            size_class: todo!(),
            size: todo!(),
        }
    }
}
