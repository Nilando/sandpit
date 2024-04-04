use super::constants::BLOCK_SIZE;
use super::errors::BlockError;
use std::ptr::NonNull;

pub type BlockPtr = NonNull<u8>;

pub struct Block {
    ptr: BlockPtr,
}

impl Block {
    pub fn new() -> Result<Block, BlockError> {
        Ok(Block {
            ptr: internal::alloc_block()?,
        })
    }

    pub fn as_ptr(&self) -> *const u8 {
        self.ptr.as_ptr()
    }
}

impl Drop for Block {
    fn drop(&mut self) {
        internal::dealloc_block(self.ptr);
    }
}

mod internal {
    use super::{BlockError, BlockPtr, BLOCK_SIZE};
    use std::alloc::{alloc, dealloc, Layout};
    use std::ptr::NonNull;

    pub fn alloc_block() -> Result<BlockPtr, BlockError> {
        unsafe {
            let layout = Layout::from_size_align_unchecked(BLOCK_SIZE, BLOCK_SIZE);

            let ptr = alloc(layout);
            if ptr.is_null() {
                Err(BlockError::OOM)
            } else {
                Ok(NonNull::new_unchecked(ptr))
            }
        }
    }

    pub fn dealloc_block(ptr: BlockPtr) {
        unsafe {
            let layout = Layout::from_size_align_unchecked(BLOCK_SIZE, BLOCK_SIZE);

            dealloc(ptr.as_ptr(), layout);
        }
    }
}
