use super::gc::{Gc, GcOpt};
use super::mutator::Mutator;
use super::tagged::{Tag, Tagged};
use super::trace::{Trace, TraceLeaf};
use core::cell::Cell;

pub trait GcSync<'gc>: Trace + Clone + 'gc {
    fn update_array(mu: &'gc Mutator, array: Gc<'gc, [Self]>, index: usize, value: Self);
}

impl<'gc, T: Trace> GcSync<'gc> for Gc<'gc, T> {
    fn update_array(mu: &'gc Mutator, array: Gc<'gc, [Self]>, index: usize, value: Self) {
        unsafe {
            array[index].set(value.clone());
        }

        if mu.has_marked(&array) {
            mu.retrace(&array[index]);
        }
    }
}

impl<'gc, T: Trace> GcSync<'gc> for GcOpt<'gc, T> {
    fn update_array(mu: &'gc Mutator, array: Gc<'gc, [Self]>, index: usize, value: Self) {
        unsafe {
            array[index].set(value.clone());
        }

        if mu.has_marked(&array) {
            if value.is_some() {
                mu.retrace(&array[index]);
            }
        }
    }
}

impl<'gc, B: Tag + 'gc> GcSync<'gc> for Tagged<'gc, B> {
    fn update_array(mu: &'gc Mutator, array: Gc<'gc, [Self]>, index: usize, value: Self) {
        unsafe {
            array[index].set(value.get_raw());
        }

        if mu.has_marked(&array) {
            if value.is_ptr() {
                mu.retrace(&array[index]);
            }
        }
    }
}

impl<'gc, T: TraceLeaf + Copy + 'gc> GcSync<'gc> for Cell<T> {
    fn update_array(_: &'gc Mutator, array: Gc<'gc, [Self]>, index: usize, value: Self) {
        array[index].swap(&value);
    }
}
