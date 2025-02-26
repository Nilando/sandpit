use super::mutator::Mutator;
use super::trace::Trace;
use super::gc::{Gc, GcOpt, GcPointer};
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

impl<'gc, T: GcPointer + 'gc, B: Tag + 'gc> GcSync<'gc> for Tagged<T, B> {
    fn update_array(mu: &'gc Mutator, array: Gc<'gc, [Self]>, index: usize, value: Self) {
        match value.get_ptr() {
            Some(ptr) => array.write_barrier(mu, |barrier| barrier.at(index).set(ptr)),
            None => array[index].set_tagged_raw(value.get_raw().unwrap(), value.get_tag().unwrap()),
        }
    }
}
