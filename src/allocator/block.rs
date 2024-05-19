use super::constants::BLOCK_SIZE;
use super::errors::BlockError;
use std::alloc::{alloc, dealloc, Layout};
use std::ptr::NonNull;

pub type BlockPtr = NonNull<u8>;

// TODO: what if block was just a Box<[u8]> ?
// I think with that we could remove a ton of unsafe code!

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

    fn alloc_block(layout: Layout) -> Result<BlockPtr, BlockError> {
        unsafe {
            let ptr = alloc(layout);

            if ptr.is_null() {
                Err(BlockError::OOM)
            } else {
                Ok(NonNull::new_unchecked(ptr))
            }
        }
    }

    fn dealloc_block(ptr: BlockPtr, layout: Layout) {
        unsafe { dealloc(ptr.as_ptr(), layout) }
    }
}

impl Drop for Block {
    fn drop(&mut self) {
        Block::dealloc_block(self.ptr, self.layout);
    }
}
