use crate::Gc;
use core::sync::atomic::{AtomicUsize, Ordering};

use super::barrier::InnerBarrier;
use super::gc::GcOpt;
use super::gc_sync::GcSync;
use super::header::GcHeader;
use super::mutator::Mutator;
use super::trace::{Trace, Tracer};

unsafe impl<'gc, T: GcSync<'gc>> Trace for GcVec<'gc, T> {
    const IS_LEAF: bool = false;

    fn trace(&self, tracer: &mut Tracer) {
        if !self.items.mark(tracer) {
            return;
        }

        if let Some(ptr) = self.items.inner().as_option() {
            tracer.mark(ptr);
        }

        let mut i = 0;
        let len = self.len.load(Ordering::Acquire);

        loop {
            if len <= i {
                break;
            }

            let items_ptr: Gc<'_, [T]> = self.items.inner().unwrap();
            let item: &T = &items_ptr[i];

            item.trace(tracer);

            i += 1;
        }
    }
}

pub struct GcVec<'gc, T: GcSync<'gc>> {
    len: AtomicUsize,
    items: InnerBarrier<GcOpt<'gc, [T]>>,
}

impl<'gc, T: GcSync<'gc>> GcVec<'gc, T> {
    const INIT_CAP: usize = 8;
    const GROW_RATE: usize = 2;

    pub fn new(mu: &'gc Mutator) -> Self {
        Self {
            len: AtomicUsize::new(0),
            items: InnerBarrier::new(mu, GcOpt::new_none()),
        }
    }

    pub fn len(&self) -> usize {
        self.len.load(Ordering::Relaxed)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn cap(&self) -> usize {
        match self.items.inner().as_option() {
            Some(gc) => gc.len(),
            None => 0,
        }
    }

    pub fn get_idx(&self, idx: usize) -> Option<T> {
        if self.len() <= idx {
            return None;
        }

        let items_ptr = self.items.inner().unwrap();

        Some(items_ptr[idx].clone())
    }

    pub fn set(&self, mu: &'gc Mutator, value: T, idx: usize) {
        if idx >= self.len() {
            panic!("out of bounds access on gc vec");
        }

        let items_ptr = self.items.inner().unwrap();

        <T as GcSync<'gc>>::update_array(mu, items_ptr, idx, value);
    }

    pub fn push(&self, mu: &'gc Mutator, value: T) {
        if self.len() == self.cap() {
            self.grow_cap(mu);
        }

        let items_ptr = self.items.inner().unwrap();

        <T as GcSync<'gc>>::update_array(mu, items_ptr, self.len(), value);

        self.len.store(self.len() + 1, Ordering::Release);
    }

    pub fn pop(&self) -> Option<T> {
        if self.len() == 0 {
            return None;
        }

        let new_len = self.len() - 1;
        let item = self.get_idx(new_len);

        self.len.store(new_len, Ordering::Release);

        item
    }

    fn grow_cap(&self, mu: &'gc Mutator) {
        let old_cap = self.cap();
        let new_cap = if self.cap() == 0 {
            Self::INIT_CAP
        } else {
            old_cap * Self::GROW_RATE
        };

        let new_array = mu.alloc_array_from_fn(new_cap, |i| {
            if i < old_cap {
                self.get_idx(i).unwrap()
            } else {
                unsafe { core::mem::MaybeUninit::zeroed().assume_init() }
            }
        });

        let old_array = self.items.inner().clone();
        // Don't use write_barrier here because that would retrace the entire new array,
        // including uninitialized elements beyond old_cap. Instead, we directly set the
        // new array and manually mark it and retrace only the valid elements.
        unsafe { self.items.inner().set(GcOpt::from(new_array.clone())); }

        // If the old array has been marked by the GC, we need to mark the new array
        // (to keep it alive) and retrace the valid elements.
        if let Some(old_array_ptr) = old_array.as_option() {
            if mu.has_marked(&old_array_ptr) {
                // Mark the new array allocation itself without tracing its contents
                // FIXME: Marking the array this way bypasses the tracer's mark_count,
                // so it won't be counted towards the old objects count in metrics.
                let header = new_array.get_header();
                let mark = mu.get_mark();
                header.set_mark(mark);
                unsafe {
                    crate::heap::mark(
                        new_array.get_header_ptr() as *mut u8,
                        new_array.get_layout(),
                        mark,
                    );
                }

                // Manually retrace only the valid elements (up to self.len)
                for i in 0..self.len() {
                    mu.retrace(&new_array[i]);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{gc::Gc, Arena, Root};

    #[test]
    pub fn init_vec() {
        let _: Arena<Root![_]> = Arena::new(|mu| {
            let vec: GcVec<Gc<()>> = GcVec::new(mu);

            assert!(vec.len() == 0);
            assert!(vec.cap() == 0);
        });
    }

    #[test]
    pub fn push_and_pop_items() {
        let _: Arena<Root![_]> = Arena::new(|mu| {
            let vec: GcVec<Gc<usize>> = GcVec::new(mu);

            for i in 0..36 {
                assert!(vec.len() == i);

                let gc = Gc::new(mu, i as usize);

                vec.push(mu, gc);
            }

            for i in (0..36).rev() {
                assert_eq!(*vec.pop().unwrap(), i);
                assert!(vec.len() == i);
            }
        });
    }
}
