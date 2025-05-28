use super::mutator::Mutator;
use super::trace::Trace;
use super::gc::{Gc, GcOpt};
use super::tagged::{Tagged, Tag};

pub trait GcSync<'gc>: Trace + Clone + 'gc {
    fn update_array(mu: &'gc Mutator, array: Gc<'gc, [Self]>, index: usize, value: Self);
}

impl<'gc, T: Trace> GcSync<'gc> for Gc<'gc, T> {
    fn update_array(mu: &'gc Mutator, array: Gc<'gc, [Self]>, index: usize, value: Self) {
        array.write_barrier(mu, |barrier| barrier.at(index).set(value));
    }
}

impl<'gc, T: Trace> GcSync<'gc> for GcOpt<'gc, T> {
    fn update_array(mu: &'gc Mutator, array: Gc<'gc, [Self]>, index: usize, value: Self) {
        array.write_barrier(mu, |barrier| barrier.at(index).set(value));
    }
}

impl<'gc, B: Tag + 'gc> GcSync<'gc> for Tagged<'gc, B> {
    fn update_array(mu: &'gc Mutator, array: Gc<'gc, [Self]>, index: usize, value: Self) {
        array.write_barrier(mu, |barrier| barrier.at(index).set(value));
    }
}
