use super::gc::{GcMut, GcOpt};
use super::trace::Trace;

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

impl<'gc, T: Trace> WriteBarrier<'gc, Option<T>> {
    pub fn into(&self) -> Option<WriteBarrier<T>> {
        match self.inner {
            Some(ref inner) => Some(WriteBarrier { inner }),
            None => None,
        }
    }
}

impl<'gc, T: Trace + ?Sized> WriteBarrier<'gc, GcMut<'gc, T>> {
    // SAFETY: A write barrier can only be safely obtained through
    // the callback passed to `fn write_barrier` in which the object
    // containing this pointer will be retraced
    pub fn set(&self, gc: GcMut<'gc, T>) {
        unsafe {
            self.inner.set(gc);
        }
    }
}

impl<'gc, T: Trace + ?Sized> WriteBarrier<'gc, GcOpt<'gc, T>> {
    // SAFETY: A write barrier can only be safely obtained through
    // the callback passed to `fn write_barrier` in which the object
    // containing this pointer will be retraced
    pub fn set(&self, gc: GcOpt<'gc, T>) {
        unsafe {
            self.inner.set(gc);
        }
    }
}

impl<'gc, T: Trace> WriteBarrier<'gc, [T]> {
    // SAFETY: A write barrier can only be safely obtained through
    // the callback passed to `fn write_barrier` in which the object
    // containing this pointer will be retraced
    pub fn at(&self, idx: usize) -> WriteBarrier<'gc, T> {
        WriteBarrier {
            inner: &self.inner[idx],
        }
    }
}

#[macro_export]
#[doc(hidden)]
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
