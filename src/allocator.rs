use super::header::GcMark;
use nimix::Allocator as Nimix;
use std::alloc::Layout;

#[derive(Clone)]
pub struct Allocator {
    allocator: Nimix,
}

impl Allocator {
    pub fn new() -> Self {
        Self {
            allocator: Nimix::new(),
        }
    }

    pub unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.allocator.alloc(layout).expect("GC Failed Alloc") as *mut u8
    }

    // needs the layout in the case of large objects
    pub unsafe fn mark(ptr: *mut u8, layout: Layout, mark: GcMark) {
        Nimix::mark(ptr, layout, mark.into()).expect("GC Failed Marking Obj")
    }

    // Anything not marked with the live mark will be freed
    pub unsafe fn sweep<F: FnOnce()>(&self, live_mark: GcMark, cb: F) {
        self.allocator.sweep(live_mark.into(), cb)
    }

    pub fn get_size(&self) -> usize {
        self.allocator.get_size()
    }
}
