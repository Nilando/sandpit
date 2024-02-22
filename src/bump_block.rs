use std::ptr::NonNull;

use crate::block::Block;
use crate::constants::BLOCK_CAPACITY;

pub struct BumpBlock {
    block: Block,
    cursor: usize,
    limit: usize
}

impl BumpBlock {
    pub fn from_ptr(ptr: *mut u8) -> Self {
        Self {
            block: Block::from_ptr(ptr),
            cursor: 0,
            limit: BLOCK_CAPACITY
        }
    }

    pub fn reserve(&mut self, size: usize) -> Option<NonNull<()>> {
        loop {
            if self.current_hole_size() >= size {
                let ptr = self.cursor;
                self.cursor += size;
                return Some(NonNull::new(ptr as *mut ()).unwrap());
            }

            self.find_next_hole();

            if 0 == self.current_hole_size() {
                return None;
            }
        }
    }

    fn find_next_hole(&mut self) {
        todo!()
    }

    fn current_hole_size(&self) -> usize {
        self.limit - self.cursor
    }
}
