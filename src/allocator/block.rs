use super::constants::BLOCK_SIZE;
use super::errors::BlockError;
use std::alloc::{alloc, dealloc, Layout};
use std::ptr::NonNull;

pub struct Block {
    ptr: NonNull<u8>,
    layout: Layout,
}

impl Block {
    pub fn default() -> Result<Block, BlockError> {
        unsafe {
            let layout = Layout::from_size_align_unchecked(BLOCK_SIZE, BLOCK_SIZE);

            Self::new(layout)
        }
    }

    pub fn new(layout: Layout) -> Result<Block, BlockError> {
        Ok(Block {
            ptr: Self::alloc_block(layout)?,
            layout,
        })
    }

    pub fn as_ptr(&self) -> *const u8 {
        self.ptr.as_ptr()
    }

    pub fn get_size(&self) -> usize {
        self.layout.size()
    }

    fn alloc_block(layout: Layout) -> Result<NonNull<u8>, BlockError> {
        unsafe {
            let ptr = alloc(layout);

            if ptr.is_null() {
                Err(BlockError::OOM)
            } else {
                Ok(NonNull::new_unchecked(ptr))
            }
        }
    }
}

impl Drop for Block {
    fn drop(&mut self) {
        unsafe { dealloc(self.ptr.as_ptr(), self.layout) }
    }
}
