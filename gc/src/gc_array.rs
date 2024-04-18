use super::gc_ptr::GcPtr;
use super::gc_cell::GcCell;
use super::trace::Trace;
use super::mutator::Mutator;
use std::ptr::write;

pub struct GcArray<T: Trace> {
    start: GcPtr<T>,
    size: GcCell<usize>,
    capacity: GcCell<usize>,
}

impl<T: Trace> GcArray<T> {
    pub fn new(start: GcPtr<T>, size: usize, capacity: usize) -> Self {
        Self {
            start,
            size: GcCell::new(size),
            capacity: GcCell::new(capacity),
        }
    }

    pub fn len(&self) -> usize {
        self.size.get()
    }

    pub fn cap(&self) -> usize {
        self.capacity.get()
    }

    pub fn at(&self, idx: usize) -> &T {
        if idx >= self.size.get() {
            panic!("Out of bounds Array access");
        }

        todo!()
    }

    pub fn push<M: Mutator>(&self, item: T, mutator: &M) {
        todo!()
        /*
        if self.size.get() == self.capacity.get() {
            // if capacity is zero
            //   alloc a new array
            //   push the new value to the new array
            //   set the pointer to that array as a strong ptr
            //   update the array field
            // else capacity is greater than 0
            //   store pointer to old array
            //   alloc new array
            //   copy values to new array
            //   push the new value to the new array
            //   if the old pointer is marked,
            //   send the new array to unscanned
        }

        match self.start.as_ptr() {
            None => {}
            Some(ptr) => {
                unsafe {
                    let offset_ptr = ptr.as_ptr().add(self.size.get());

                    write(offset_ptr, item);
                }
            }
        }

        self.size.set(self.size.get() + 1);
        */
    }

    pub fn pop(&self) -> T {
        todo!()
    }
}
