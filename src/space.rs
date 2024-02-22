use crate::constants::{BLOCK_SIZE, SPACE_SIZE};
use crate::errors::AllocError;

pub struct Space {
    start: *const u8,
    end: *const u8,
    size: usize
}

impl Space {
    pub fn new() -> Result<Self, AllocError> {
        unsafe {
            let layout = std::alloc::Layout::from_size_align_unchecked(SPACE_SIZE, BLOCK_SIZE);
            let start = std::alloc::alloc(layout);
            let end = start.add(SPACE_SIZE);

            Ok(Space { start, end, size: SPACE_SIZE })
        }
    }

    pub fn start(&self) -> *const u8 {
        self.start
    }

    pub fn end(&self) -> *const u8 {
        self.end
    }
}
