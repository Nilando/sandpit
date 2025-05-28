use super::gc::{Gc, GcOpt};
use super::trace::{Trace, Tracer};
use super::mutator::Mutator;
use super::tagged::{Tagged, Tag};
use super::gc::GcPointer;
use std::sync::atomic::{AtomicU8, Ordering};

/// Allows for the mutation of [`Gc`] and [`GcOpt`] pointers.
///
/// A write barrier can only be obtained initially by calling [`Gc::write_barrier`]
/// or [`crate::gc::Gc::write_barrier`]. The barrier is given out in a callback, in which afterwards,
/// the initial GC pointer will be retraced. This ensure any updates made by the
/// barrier will be caught by the tracers.
///
/// Also see the [`crate::field`] macro which is needed to safely "move" the
/// write barrier onto fields within a struct.
pub struct WriteBarrier<'gc, T: Trace + ?Sized> {
    inner: &'gc T,
}

impl<'gc, T: Trace + ?Sized> WriteBarrier<'gc, T> {
    // WriteBarriers are unsafe to create, as they themselves don't ensure
    // anything is retraced, they only communicate that they should have been
    // created in some context where retracing will happen after the barrier
    // goes out of context.
    pub(crate) unsafe fn new(gc: &'gc T) -> Self {
        WriteBarrier { inner: gc }
    }

    /// Get a reference to the value behind barrier
    ///
    /// ## Example
    /// ```rust
    /// use sandpit::{Arena, Gc, Root};
    ///
    /// let arena: Arena<Root![Gc<'_, Gc<'_, usize>>]> = Arena::new(|mu| {
    ///    Gc::new(mu, Gc::new(mu, 69))
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

impl<'gc, T: Trace + ?Sized> WriteBarrier<'gc, Gc<'gc, T>> {
    /// Update a [`Gc`] that is within a write barrier to point at a new value.
    ///
    /// ## Example
    /// ```rust
    /// use sandpit::{Arena, Gc, Root};
    ///
    /// let arena: Arena<Root![Gc<'_, Gc<'_, usize>>]> = Arena::new(|mu| {
    ///    Gc::new(mu, Gc::new(mu, 69))
    /// });
    ///
    /// arena.mutate(|mu, root| {
    ///     root.write_barrier(mu, |barrier| {
    ///         let new_value = Gc::new(mu, 420);
    ///         barrier.set(new_value);
    ///
    ///         assert!(**barrier.inner() == 420);
    ///     })
    /// });
    ///```
    // SAFETY: A write barrier can only be safely obtained through
    // the callback passed to `fn write_barrier` in which the object
    // containing this pointer will be retraced
    pub fn set(&self, gc: impl Into<Gc<'gc, T>>) {
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
    /// use sandpit::{Arena, Gc, GcOpt, Root};
    ///
    /// let arena: Arena<Root![Gc<'_, GcOpt<'_, usize>>]> = Arena::new(|mu| {
    ///    Gc::new(mu, GcOpt::new_none())
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
    /// use sandpit::{Arena, Gc, GcOpt, Root};
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
        self.inner.as_ref().map(|inner| WriteBarrier { inner })
    }
}

pub struct InnerBarrier<T: Trace> {
    mark: AtomicU8,
    inner: T,
}

impl<T: Trace> InnerBarrier<T> {
    pub fn new(mu: &Mutator, inner: T) -> Self {
        Self {
            mark: AtomicU8::new(mu.get_mark().into()),
            inner
        }
    }

    pub fn inner(&self) -> &T {
        &self.inner
    }

    pub fn mark(&self, tracer: &mut Tracer) -> bool {
        let mark = self.mark.load(Ordering::Acquire);

        if mark == tracer.get_mark().into() {
            return false;
        }

        self.mark.store(tracer.get_mark().into(), Ordering::SeqCst);

        return true;
    }

    pub fn write_barrier<'gc, F>(&self, mu: &'gc Mutator, f: F)
    where
        F: FnOnce(&WriteBarrier<T>),
    {
        // SAFETY: Its safe to create a writebarrier over this pointer b/c it is guaranteed
        // to be retraced after the closure ends.
        let barrier = unsafe { WriteBarrier::new(&self.inner) };

        f(&barrier);

        if self.mark.load(Ordering::Acquire) == mu.get_mark().into() {
            mu.retrace(&self.inner);
        }
    }
}

unsafe impl<T: Trace> Trace for InnerBarrier<T> {
    const IS_LEAF: bool = T::IS_LEAF;

    fn trace(&self, tracer: &mut Tracer) {
        if self.mark.load(Ordering::Acquire) != tracer.get_mark().into() {
            self.mark(tracer);
            self.inner.trace(tracer);
        }
    }
}

impl<'gc, B: Tag> WriteBarrier<'gc, Tagged<'gc, B>> {
    pub fn set(&self, tagged_ptr: Tagged<'gc, B>) {
        unsafe { self.inner.set(tagged_ptr) };
    }
}

/// Exists to allow getting a write barrier to an inner field.
///
/// The field macro is needed to control how a [`WriteBarrier`] can be created,
/// ensuring that from one write barrier, further barriers can only be obtained
/// to fields within the same contiguous allocation/type.
///
/// # Example 
/// ```rust
/// use sandpit::{Arena, Trace, Root, Gc, field};
///
/// #[derive(Trace)]
/// struct Foo<'gc> {
///     inner: Gc<'gc, bool>,
/// }
///
/// let arena: Arena<Root![Gc<'_, Foo<'_>>]> = Arena::new(|mu| {
///     let foo = Foo {
///         inner: Gc::new(mu, false),
///     };
///
///     Gc::new(mu, foo)
/// });
///
/// arena.mutate(|mu, root| {
///     let new = Gc::new(mu, true);
///     root.write_barrier(mu, |write_barrier| {
///         // use `field!` to get a write barrier around Foo's inner field.
///         let inner_barrier = field!(write_barrier, Foo, inner);
///
///         // Now that the write barrier is around `inner` we can update the Gc.
///         inner_barrier.set(new);
///     });
/// });
/// ```
#[macro_export]
macro_rules! field {
    ($value:expr, $type:path, $field:ident) => {{
        let _: &$crate::WriteBarrier<_> = $value;

        match $value.inner() {
            $type { ref $field, .. } => unsafe {
                $crate::WriteBarrier::__from_field($field, $field as *const _)
            },
            _ => panic!("WriteBarrier field! macro failed to match on inner field"),
        }
    }};
}
