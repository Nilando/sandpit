use crate::Gc;
use core::sync::atomic::{AtomicUsize, Ordering};

use super::barrier::InnerBarrier;
use super::gc::GcOpt;
use super::gc_sync::GcSync;
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

    pub fn as_slice(&mut self) -> &[T] {
        // BAD!!!! => self.items.inner().unwrap().scoped_deref()
        todo!()
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

        self.items
            .write_barrier(mu, |barrier| barrier.set(new_array));
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
