use super::header::GcMark;
use alloc::alloc::Layout;
use nimix::{mark as nimix_mark, Allocator as NimixAllocator, Heap as NimixHeap};

pub struct Allocator {
    allocator: NimixAllocator,
}

impl From<&Heap> for Allocator {
    fn from(value: &Heap) -> Self {
        Allocator {
            allocator: NimixAllocator::from(&value.heap),
        }
    }
}

impl Allocator {
    pub fn alloc(&self, layout: Layout) -> *const u8 {
        unsafe { self.allocator.alloc(layout).expect("Failed to allocate") }
    }
}

pub struct Heap {
    heap: NimixHeap,
}

impl Heap {
    pub fn new() -> Self {
        Self {
            heap: NimixHeap::new(),
        }
    }

    pub unsafe fn sweep(&self, live_mark: GcMark) {
        self.heap.sweep(live_mark.into())
    }

    pub fn get_size(&self) -> u64 {
        self.heap.size() as u64
    }
}

pub unsafe fn mark(ptr: *mut u8, layout: Layout, mark: GcMark) {
    nimix_mark(ptr, layout, mark.into()).expect("GC Failed Marking Obj")
}
