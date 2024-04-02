use super::constants::BLOCK_SIZE;
use super::errors::BlockError;
use std::ptr::NonNull;

pub type BlockPtr = NonNull<u8>;
pub type BlockSize = usize;

pub struct Block {
    ptr: BlockPtr,
    size: BlockSize,
}

impl Block {
    pub fn new(size: BlockSize, align: usize) -> Result<Block, BlockError> {
        if !align.is_power_of_two() {
            return Err(BlockError::BadRequest);
        }

        Ok(Block {
            ptr: internal::alloc_block(size, align)?,
            size,
        })
    }

    pub fn default() -> Result<Block, BlockError> {
        Self::new(BLOCK_SIZE, BLOCK_SIZE)
    }

    pub fn into_mut_ptr(self) -> BlockPtr {
        self.ptr
    }

    pub fn size(&self) -> BlockSize {
        self.size
    }

    pub unsafe fn from_raw_parts(ptr: BlockPtr, size: BlockSize) -> Block {
        Block { ptr, size }
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
    use super::{BlockError, BlockPtr, BlockSize};
    use std::alloc::{alloc, dealloc, Layout};
    use std::mem::size_of;
    use std::ptr::NonNull;
    const LAYOUT: usize = size_of::<usize>();

    pub fn alloc_block(size: BlockSize, align: usize) -> Result<BlockPtr, BlockError> {
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

    pub fn dealloc_block(ptr: BlockPtr, size: BlockSize) {
        unsafe {
            let layout = Layout::from_size_align_unchecked(size, LAYOUT);

            dealloc(ptr.as_ptr(), layout);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Block, BlockError, BlockSize};
    use std::mem::size_of;

    fn alloc_dealloc(size: BlockSize) -> Result<(), BlockError> {
        let block = Block::new(size, 8)?;
        let mask = size_of::<usize>() - 1;

        assert!((block.ptr.as_ptr() as usize & mask) ^ mask == mask);

        Ok(())
    }

    #[test]
    fn test_bad_sizealign() {
        assert!(Block::new(333, 3).is_err())
    }

    #[test]
    fn test_4k() {
        assert!(alloc_dealloc(4096).is_ok())
    }

    #[test]
    fn test_32k() {
        assert!(alloc_dealloc(32768).is_ok())
    }

    #[test]
    fn test_16m() {
        assert!(alloc_dealloc(16 * 1024 * 1024).is_ok())
    }

    #[test]
    fn test_oom() {
        assert_eq!(
            alloc_dealloc(1024 * 1024 * 1024 * 1024),
            Err(BlockError::OOM)
        )
    }

    #[test]
    fn test_from_raw_parts() {
        let block = Block::new(1024, 1024).unwrap();
        let ptr = block.as_ptr();
        let new_block = unsafe { Block::from_raw_parts(block.into_mut_ptr(), 1024) };
        assert_eq!(ptr, new_block.as_ptr());
        std::mem::forget(new_block); // don't dealloc 'new_block' since it already exists, and will be dropped, as 'block'
    }
}
