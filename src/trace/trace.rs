use super::tracer::Tracer;
use std::cell::Cell;
use std::ptr::NonNull;
use std::sync::atomic::AtomicUsize;

/// TraceLeaf is a sub-trait of Trace which ensures its implementor does not
/// contain any GcPtr's.
pub unsafe trait TraceLeaf: Trace {}

/// Types allocated in a Gc are required to implement this trait.
pub unsafe trait Trace {
    fn trace<T: Tracer>(&self, tracer: &mut T);

    fn dyn_trace<T: Tracer>(ptr: NonNull<()>, tracer: &mut T)
    where
        Self: Sized,
    {
        unsafe { ptr.cast::<Self>().as_ref().trace(tracer) }
    }

    fn needs_trace(&self) -> bool {
        true
    }

    fn is_leaf() -> bool {
        false
    }
}

// ****************************************************************************
// TRACE LEAF IMPLS
// ****************************************************************************

unsafe impl<L: AssertTraceLeaf> TraceLeaf for L {}

pub unsafe trait AssertTraceLeaf: TraceLeaf {
    // This function should go through every field type and assert that each
    // type implements TraceLeaf.
    fn assert_leaf_fields(&self);
    fn assert_leaf<T: TraceLeaf>() {}
}

unsafe impl<L: TraceLeaf> Trace for L {
    fn trace<T: Tracer>(&self, _: &mut T) {}

    fn dyn_trace<T: Tracer>(_ptr: NonNull<()>, _: &mut T) {}

    fn needs_trace(&self) -> bool {
        false
    }

    fn is_leaf() -> bool {
        true
    }
}

macro_rules! impl_trace_leaf {
    ($($t:ty),*) => {
        $(unsafe impl TraceLeaf for $t {})*
    };
}

impl_trace_leaf!(
    (),
    bool,
    u8,
    u16,
    u32,
    u64,
    u128,
    usize,
    i8,
    i16,
    i32,
    i64,
    i128,
    isize,
    AtomicUsize
);

// ****************************************************************************
// TRACE IMPLS
// ****************************************************************************

unsafe impl<const N: usize, L: Trace> Trace for [L; N] {
    fn trace<T: Tracer>(&self, tracer: &mut T) {
        if !self.needs_trace() {
            return;
        }

        for item in self.iter() {
            item.trace(tracer)
        }
    }
}

unsafe impl<T: Trace> Trace for Option<T> {
    fn trace<R: Tracer>(&self, tracer: &mut R) {
        if let Some(value) = self.as_ref() {
            value.trace(tracer)
        }
    }
}

unsafe impl<'a, T: Trace> Trace for crate::gc::Gc<'a, T> {
    fn trace<R: Tracer>(&self, tracer: &mut R) {
        unsafe {
            let ptr = self.as_ptr();

            if !ptr.is_null() {
                tracer.trace(NonNull::new_unchecked(ptr))
            }
        }
    }
}

unsafe impl<A: Trace, B: Trace> Trace for (A, B) {
    fn trace<R: Tracer>(&self, tracer: &mut R) {
        self.0.trace(tracer);
        self.1.trace(tracer);
    }
}

unsafe impl<T: TraceLeaf> TraceLeaf for Cell<T> {}
