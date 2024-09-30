use super::header::GcMark;
use nimix::{AllocError as RawAllocError, Allocator as RawAllocator};
use std::alloc::Layout;

#[derive(Debug)]
pub enum AllocError {
    OOM,
    AllocOverflow,
}

impl From<RawAllocError> for AllocError {
    fn from(value: RawAllocError) -> Self {
        match value {
            RawAllocError::OOM => AllocError::OOM,
            RawAllocError::AllocOverflow => AllocError::AllocOverflow,
        }
    }
}

#[derive(Clone)]
pub struct Allocator {
    allocator: RawAllocator,
}

impl Allocator {
    pub fn new() -> Self {
        Self {
            allocator: RawAllocator::new(),
        }
    }

    pub unsafe fn alloc(&self, layout: Layout) -> Result<*mut u8, AllocError> {
        let ptr: *mut u8 = self.allocator.alloc(layout)?;

        Ok(ptr)
    }

    // needs the layout in the case of large objects
    pub unsafe fn mark(ptr: *mut u8, layout: Layout, mark: GcMark) -> Result<(), AllocError> {
        Ok(RawAllocator::mark(ptr, layout, mark.into())?.into())
    }

    // Anything not marked with the live mark will be freed
    pub unsafe fn sweep<F: FnOnce()>(&self, live_mark: GcMark, cb: F) {
        self.allocator.sweep(live_mark.into(), cb)
    }

    pub fn get_size(&self) -> usize {
        self.allocator.get_size()
    }
}
