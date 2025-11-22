use super::gc::{Gc, GcOpt};
use super::mutator::Mutator;
use super::tagged::{Tag, Tagged};
use super::trace::{Trace, TraceLeaf};
use core::cell::Cell;

pub trait GcSync<'gc>: Trace + Clone + 'gc {
    /// Swap old value with new value, updating GC pointers atomically.
    ///
    /// # Safety
    /// `old` must point to valid, properly aligned memory.
    unsafe fn gc_swap(old: &Self, new: Self, mu: &'gc Mutator);

    /// Update an element in an array, handling write barriers.
    ///
    /// This default implementation calls `gc_swap` and then retraces if needed.
    fn update_array(mu: &'gc Mutator, array: Gc<'gc, [Self]>, index: usize, value: Self) {
        unsafe {
            Self::gc_swap(&array[index], value, mu);
        }

        if mu.has_marked(&array) {
            mu.retrace(&array[index]);
        }
    }
}

impl<'gc, T: Trace + ?Sized> GcSync<'gc> for Gc<'gc, T> {
    unsafe fn gc_swap(old: &Self, new: Self, _mu: &'gc Mutator) {
        old.set(new);
    }
}

impl<'gc, T: Trace + ?Sized> GcSync<'gc> for GcOpt<'gc, T> {
    unsafe fn gc_swap(old: &Self, new: Self, _mu: &'gc Mutator) {
        old.set(new);
    }
}

impl<'gc, B: Tag + 'gc> GcSync<'gc> for Tagged<'gc, B> {
    unsafe fn gc_swap(old: &Self, new: Self, _mu: &'gc Mutator) {
        old.set(new.get_raw());
    }
}

impl<'gc, T: TraceLeaf + Copy + 'gc> GcSync<'gc> for Cell<T> {
    unsafe fn gc_swap(old: &Self, new: Self, _mu: &'gc Mutator) {
        old.swap(&new);
    }
}
