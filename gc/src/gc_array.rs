use super::gc_ptr::GcCellPtr;
use super::trace::Trace;

pub struct GcArray<T: Trace> {
    start: GcCellPtr<T>,
    size: usize,
    capacity: usize,
}

impl<T: Trace> GcArray<T> {
    pub fn new(start: GcCellPtr<T>, size: usize, capacity: usize) -> Self {
        Self {
            start,
            size,
            capacity,
        }
    }

    pub fn len(&self) -> usize {
        self.size
    }

    pub fn cap(&self) -> usize {
        self.capacity
    }

    pub fn at(&self, idx: usize) -> &T {
        if idx >= self.size {
            panic!("Out of bounds Array access");
        }

        unsafe {
            let start = self.start.unwrap().as_ptr().as_ptr();
            let idx_ptr = start.add(idx);

            &*idx_ptr
        }
    }

    pub fn push(&self, item: T) {
        // if len is less than capacity
        // simply push

        // if len == cap
        // we need to grow/realloc the array
        // this will change the start pointer and update the capacity
        todo!()
    }

    pub fn pop(&self) -> T {
        todo!()
    }
}
