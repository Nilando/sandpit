use super::constants::{ALIGN, BLOCK_SIZE};
use super::errors::BlockError;
use std::ptr::NonNull;
use std::alloc::Layout;

pub type BlockPtr = NonNull<u8>;

pub struct Block {
    ptr: BlockPtr,
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
            ptr: internal::alloc_block(layout)?,
            layout,
        })
    }

    pub fn as_ptr(&self) -> *const u8 {
        self.ptr.as_ptr()
    }

    pub fn get_size(&self) -> usize {
        self.layout.size()
    }
}

impl Drop for Block {
    fn drop(&mut self) {
        internal::dealloc_block(self.ptr, self.layout);
    }
}

mod internal {
    use super::{BlockError, BlockPtr, ALIGN, BLOCK_SIZE};
    use std::alloc::{alloc, dealloc, Layout};
    use std::ptr::NonNull;

    pub fn alloc_block(layout: Layout) -> Result<BlockPtr, BlockError> {
        unsafe {
            let ptr = alloc(layout);

            if ptr.is_null() {
                Err(BlockError::OOM)
            } else {
                Ok(NonNull::new_unchecked(ptr))
            }
        }
    }

    pub fn dealloc_block(ptr: BlockPtr, layout: Layout) {
        unsafe { dealloc(ptr.as_ptr(), layout) }
    }
}
