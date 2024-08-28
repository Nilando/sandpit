use crate::{Arena, Root, Gc, Trace, Mutator};
use super::mutator::MutatorScope;
use super::allocator::Allocator;

#[repr(transparent)]
pub struct WriteBarrier<'gc, T: Trace> {
    pub inner: &'gc T,
}

impl<'barrier, T: Trace> WriteBarrier<'barrier, T> {
    pub fn new(gc: &'barrier T) -> Self {
        WriteBarrier {
            inner: &gc
        }
    }
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
        match $value {
            $crate::WriteBarrier {
                inner: $type { ref $field, .. },
                ..
            } => unsafe { $crate::WriteBarrier::__from_ref_and_ptr($field, $field as *const _) },
        }
    };
}
