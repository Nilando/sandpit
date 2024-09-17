use super::header::GcMark;
use crate::raw_allocator::{AllocError as RawAllocError, Allocator as RawAllocator};
use std::alloc::Layout;
use std::ptr::NonNull;

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

    pub fn alloc(&self, layout: Layout) -> Result<NonNull<u8>, AllocError> {
        let ptr: *mut u8 = self.allocator.alloc(layout)?;

        Ok(NonNull::new(ptr).unwrap().cast())
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
