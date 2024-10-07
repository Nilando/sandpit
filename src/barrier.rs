use super::gc::{Gc, GcMut, GcOpt};
use super::trace::Trace;

/// Allows for the mutation of [`GcMut`] and [`GcOpt`] pointers.
///
/// A write barrier can only be obtained initially by calling [`GcMut::write_barrier`]
/// or [`Gc::write_barrier`]. The barrier is given out in a callback, in whcih afterwards,
/// the initial GC pointer will be retraced. This ensure any updates made by the
/// barrier will be caught by the tracers.
pub struct WriteBarrier<'gc, T: Trace + ?Sized> {
    inner: &'gc T,
}

impl<'gc, T: Trace + ?Sized> WriteBarrier<'gc, T> {
    // WriteBarriers are unsafe to create, as they themselves don't ensure
    // anything is retraced, they only communicate that they should have been
    // created in some context where retracing will happen after the barrier
    // goes out of context.
    pub(crate) unsafe fn new(gc: &'gc T) -> Self {
        WriteBarrier { inner: &gc }
    }

    /// Get a reference to the value behind barrier
    ///
    /// ## Example
    /// ```rust
    /// use sandpit::{Arena, gc::{Gc, GcMut}, Root};
    ///
    /// let arena: Arena<Root![Gc<'_, GcMut<'_, usize>>]> = Arena::new(|mu| {
    ///    Gc::new(mu, GcMut::new(mu, 69))
    /// });
    ///
    /// arena.mutate(|mu, root| {
    ///     root.write_barrier(mu, |barrier| {
    ///         assert!(**barrier.inner() == 69);
    ///     })
    /// });
    ///```
    pub fn inner(&self) -> &'gc T {
        self.inner
    }

    // SAFETY: this can only be safely called via the field! macro
    // which ensures that the inner value is within an existing write barrier
    // needs to be pub so that the field! macro can work
    #[doc(hidden)]
    pub unsafe fn __from_field(inner: &'gc T, _: *const T) -> Self {
        Self { inner }
    }
}

impl<'gc, T: Trace + ?Sized> WriteBarrier<'gc, GcMut<'gc, T>> {
    /// Update a [`GcMut`] that is within a write barrier to point at a new value.
    ///
    /// ## Example
    /// ```rust
    /// use sandpit::{Arena, gc::{Gc, GcMut}, Root};
    ///
    /// let arena: Arena<Root![Gc<'_, GcMut<'_, usize>>]> = Arena::new(|mu| {
    ///    Gc::new(mu, GcMut::new(mu, 69))
    /// });
    ///
    /// arena.mutate(|mu, root| {
    ///     root.write_barrier(mu, |barrier| {
    ///         let new_value = GcMut::new(mu, 420);
    ///         barrier.set(new_value);
    ///
    ///         assert!(**barrier.inner() == 420);
    ///     })
    /// });
    ///```
    // SAFETY: A write barrier can only be safely obtained through
    // the callback passed to `fn write_barrier` in which the object
    // containing this pointer will be retraced
    pub fn set(&self, gc: impl Into<GcMut<'gc, T>>) {
        unsafe {
            self.inner.set(gc.into());
        }
    }
}

impl<'gc, T: Trace + ?Sized> WriteBarrier<'gc, GcOpt<'gc, T>> {
    /// Update a [`GcOpt`] that is within a write barrier to point at a new value.
    ///
    /// ## Example
    /// ```rust
    /// use sandpit::{Arena, gc::{Gc, GcOpt}, Root};
    ///
    /// let arena: Arena<Root![Gc<'_, GcOpt<'_, usize>>]> = Arena::new(|mu| {
    ///    Gc::new(mu, GcOpt::new_none(mu))
    /// });
    ///
    /// arena.mutate(|mu, root| {
    ///     root.write_barrier(mu, |barrier| {
    ///         let new_value = GcOpt::new(mu, 420);
    ///
    ///         barrier.set(new_value);
    ///
    ///         assert!(*barrier.inner().as_option().unwrap() == 420);
    ///     })
    /// });
    ///```
    // SAFETY: A write barrier can only be safely obtained through
    // the callback passed to `fn write_barrier` in which the object
    // containing this pointer will be retraced
    pub fn set(&self, gc: impl Into<GcOpt<'gc, T>>) {
        unsafe {
            self.inner.set(gc.into());
        }
    }
}

impl<'gc, T: Trace> WriteBarrier<'gc, [T]> {
    /// Get a writer barrier to a index of a slice behind write barrier.
    ///
    /// ## Example
    /// ```rust
    /// use sandpit::{Arena, gc::{Gc, GcOpt}, Root};
    ///
    /// let arena: Arena<Root![Gc<'_, [usize]>]> = Arena::new(|mu| {
    ///    mu.alloc_array_from_fn(10, |i| i)
    /// });
    ///
    /// arena.mutate(|mu, root| {
    ///     root.write_barrier(mu, |barrier| {
    ///         let inner_barrier = barrier.at(3);
    ///
    ///         assert_eq!(*inner_barrier.inner(), 3);
    ///     })
    /// });
    ///```
    // SAFETY: A write barrier can only be safely obtained through
    // the callback passed to `fn write_barrier` in which the object
    // containing this pointer will be retraced
    pub fn at(&self, idx: usize) -> WriteBarrier<'gc, T> {
        WriteBarrier {
            inner: &self.inner[idx],
        }
    }
}

impl<'gc, T: Trace> WriteBarrier<'gc, Option<T>> {
    pub fn into(&self) -> Option<WriteBarrier<T>> {
        match self.inner {
            Some(ref inner) => Some(WriteBarrier { inner }),
            None => None,
        }
    }
}

/// Exists to allow getting a write barrier to an inner field.
///
/// It would be unsafe to allow for creating a write barrier around
#[macro_export]
macro_rules! field {
    ($value:expr, $type:path, $field:ident) => {{
        let _: &$crate::WriteBarrier<_> = $value;

        match $value.inner() {
            $type { ref $field, .. } => unsafe {
                $crate::WriteBarrier::__from_field($field, $field as *const _)
            },
        }
    }};
}
