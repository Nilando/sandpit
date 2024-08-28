use crate::{Gc, Trace};

#[repr(transparent)]
pub struct WriteBarrier<'gc, T: Trace> {
    inner: &'gc T,
}

impl<'barrier, T: Trace> WriteBarrier<'barrier, T> {
    pub(crate) fn new(gc: &'barrier T) -> Self {
        WriteBarrier {
            inner: &gc
        }
    }

    pub fn inner(&self) -> &'barrier T {
        self.inner
    }

    #[inline(always)]
    #[doc(hidden)]
    pub fn __type_check(_: &WriteBarrier<'barrier, T>) {}

    /// Implementation detail of `write_field!`; same safety requirements as `assume`.
    #[inline(always)]
    #[doc(hidden)]
    pub unsafe fn __from_ref_and_ptr(v: &T, _: *const T) -> &Self {
        // SAFETY: `Self` is `repr(transparent)`.
        std::mem::transmute(v)
    }
}

impl<'barrier, T: Trace> WriteBarrier<'barrier, Option<T>> {
    pub fn into(&self) -> Option<WriteBarrier<T>> {
        match self.inner {
            Some(ref inner) => Some(WriteBarrier { inner }),
            None => None,
        }
    }
}

impl<'barrier, T: Trace> WriteBarrier<'barrier, Gc<'barrier, T>> {
    pub fn set(&mut self, new_ptr: Gc<'barrier, T>) {
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
            WriteBarrier::__type_check($value);

            match $value.inner() {
                $type { ref $field, .. } => unsafe { $crate::WriteBarrier::__from_ref_and_ptr($field, $field as *const _) },
            }
        }
    };
}
