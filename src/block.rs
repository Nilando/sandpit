use crate::constants::{
    BLOCK_CAPACITY, EFFECTIVE_LINES, BLOCK_MARK_OFFSET, META_OFFSET, BLOCK_SIZE
};

pub struct Block {
    ptr: *const u8
}

impl Block {
    pub fn from_obj_ptr(ptr: *const u8) -> Self {
        let aligned_ptr = unsafe { ptr.sub(ptr as usize % BLOCK_SIZE) };

        Self::from_ptr(aligned_ptr)
    }

    pub fn from_ptr(ptr: *const u8) -> Self {
        debug_assert!(ptr as usize % BLOCK_SIZE == 0);

        Self { ptr }
    }

    pub fn mark_block(&mut self) {
        unsafe { *(self.ptr.add(BLOCK_MARK_OFFSET) as *mut u8) = 1 }
    }

    pub fn is_block_marked(&mut self) -> bool {
        unsafe { *(self.ptr.add(BLOCK_MARK_OFFSET)) != 0 }
    }

    pub fn mark_line(&mut self, line: usize) {
        debug_assert!(line <= EFFECTIVE_LINES);

        unsafe { *(self.ptr.add(META_OFFSET + line) as *mut u8) = 1 }
    }

    pub fn is_line_marked(&mut self, line: usize) -> bool {
        debug_assert!(line <= EFFECTIVE_LINES);

        unsafe { *(self.ptr.add(META_OFFSET + line)) != 0 }
    }

    pub fn as_ptr(self) -> *const u8 {
        self.ptr
    }
}
