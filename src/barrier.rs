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

    /// Implementation detail of `write_field!`; same safety requirements as `assume`.
    #[doc(hidden)]
    pub unsafe fn __from_ref_and_ptr(v: &T, _: *const T) -> &Self {
        // SAFETY: `Self` is `repr(transparent)`.
        std::mem::transmute(v)
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
    pub fn set(&self, new_ptr: Gc<'gc, T>) {
        unsafe {
            self.inner.set(new_ptr);
        }
    }
}

impl<'gc, T: Trace> WriteBarrier<'gc, GcNullMut<'gc, T>> {
    pub fn set(&self, new_ptr: Gc<'gc, T>) {
        unsafe {
            self.inner.set(new_ptr);
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
                $type { ref $field, .. } => unsafe { $crate::WriteBarrier::__from_ref_and_ptr($field, $field as *const _) },
            }
        }
    };
}
