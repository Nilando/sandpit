use super::header::GcMark;
use nimix::Heap as NimixHeap;
use std::alloc::Layout;

#[derive(Clone)]
pub struct Heap {
    heap: NimixHeap,
}

impl Heap {
    pub fn new() -> Self {
        Self {
            heap: NimixHeap::new(),
        }
    }

    pub unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.heap.alloc(layout).expect("GC Failed Alloc") as *mut u8
    }

    pub unsafe fn mark(ptr: *mut u8, layout: Layout, mark: GcMark) {
        NimixHeap::mark(ptr, layout, mark.into()).expect("GC Failed Marking Obj")
    }

    pub unsafe fn sweep<F: FnOnce()>(&self, live_mark: GcMark, cb: F) {
        self.heap.sweep(live_mark.into(), cb)
    }

    pub fn get_size(&self) -> u64 {
        self.heap.size() as u64
    }
}
