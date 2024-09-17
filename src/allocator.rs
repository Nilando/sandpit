use crate::raw_allocator::{Allocator as RawAllocator, AllocError as RawAllocError};
use std::ptr::NonNull;
use std::alloc::Layout;
use super::header::GcMark;


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
            allocator: RawAllocator::new()
        }
    }

    pub fn alloc(&self, layout: Layout) -> Result<NonNull<u8>, AllocError> {
        Ok(self.allocator.alloc(layout)?)
    }

    // needs the layout in the case of large objects
    pub fn mark(ptr: *mut u8, layout: Layout, mark: GcMark) -> Result<(), AllocError> {
        Ok(RawAllocator::mark(ptr, layout, mark as u8)?)
    }

    pub fn sweep(&self, live_mark: GcMark) {
        self.allocator.sweep(live_mark as u8)
    }

    pub fn is_sweeping(&self) -> bool {
        self.allocator.is_sweeping()
    }

    pub fn get_size(&self) -> usize {
        self.allocator.get_size()
    }
}
