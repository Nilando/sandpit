use super::constants::{BLOCK_SIZE, ALIGN};
use super::errors::BlockError;
use std::ptr::NonNull;

pub type BlockPtr = NonNull<u8>;

pub struct Block {
    ptr: BlockPtr,
    size: usize,
}

impl Block {
    pub fn default() -> Result<Block, BlockError> {
        Self::new(BLOCK_SIZE, BLOCK_SIZE)
    }

    pub fn new(size: usize, align: usize) -> Result<Block, BlockError> {
        Ok(Block {
            ptr: internal::alloc_block(size, align)?,
            size,
        })
    }

    pub fn as_ptr(&self) -> *const u8 {
        self.ptr.as_ptr()
    }
}

impl Drop for Block {
    fn drop(&mut self) {
        internal::dealloc_block(self.ptr, self.size);
    }
}

mod internal {
    use super::{BlockError, BlockPtr, BLOCK_SIZE, ALIGN};
    use std::alloc::{alloc, dealloc, Layout};
    use std::ptr::NonNull;

    pub fn alloc_block(size: usize, align: usize) -> Result<BlockPtr, BlockError> {
        unsafe {
            let layout = Layout::from_size_align_unchecked(size, align);
            let ptr = alloc(layout);

            if ptr.is_null() {
                Err(BlockError::OOM)
            } else {
                Ok(NonNull::new_unchecked(ptr))
            }
        }
    }

    pub fn dealloc_block(ptr: BlockPtr, size: usize) {
        unsafe {
            let layout =
                if size > BLOCK_SIZE {
                    Layout::from_size_align_unchecked(size, ALIGN)
                } else {
                    Layout::from_size_align_unchecked(BLOCK_SIZE, BLOCK_SIZE)
                };

            dealloc(ptr.as_ptr(), layout);
        }
    }
}
