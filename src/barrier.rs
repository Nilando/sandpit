use crate::{Gc, GcMut, GcNullMut, Trace};

#[repr(transparent)]
pub struct WriteBarrier<'gc, T: Trace> {
    inner: &'gc T,
}

impl<'gc, T: Trace> WriteBarrier<'gc, T> {
    pub(crate) fn new(gc: &'gc T) -> Self {
        WriteBarrier {
            inner: &gc
        }
    }

    pub fn inner(&self) -> &'gc T {
        self.inner
    }

    #[doc(hidden)]

    // SAFETY: this can only be safely called via the field! macro
    // which ensures that the inner value is within an existing write barrier
    pub unsafe fn __from_field(inner: &'gc T, _: *const T) -> Self {
        Self {
            inner
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

impl<'gc, T: Trace> WriteBarrier<'gc, GcMut<'gc, T>> {
    pub fn set(&self, gc: Gc<'gc, T>) {
        println!("setting: {:?}", gc.as_nonnull());

        // transmute self into a gcmut


        unsafe {
            self.inner.set(gc);
        }
    }
}

impl<'gc, T: Trace> WriteBarrier<'gc, GcNullMut<'gc, T>> {
    pub fn set(&self, gc: Gc<'gc, T>) {
        println!("setting: {:?}", gc.as_nonnull());
        println!("inner: {:?}", self.inner as *const GcNullMut<T>);

        unsafe {
            self.inner.set(gc);
        }
    }
}

#[macro_export]
#[doc(hidden)]
macro_rules! field {
    ($value:expr, $type:path, $field:ident) => {
        {
            let _: &$crate::WriteBarrier<_> = $value;

            match $value.inner() {
                $type { ref $field, .. } => unsafe { $crate::WriteBarrier::__from_field($field, $field as *const _) },
            }
        }
    };
}
