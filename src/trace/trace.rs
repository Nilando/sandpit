use super::tracer::Tracer;
use std::cell::*;
use std::ptr::NonNull;
use std::sync::atomic::*;

/// TraceLeaf is a sub-trait of Trace which ensures its implementor does not
/// contain any GcPtr's.
/// It would be written like TraceLeaf: Trace but unfortunately due to negative
/// trait bounds being unstable it makes it infeasible to do;
pub unsafe trait TraceLeaf {
    fn __assert_trace_leaf() {}
}

/// Types allocated in a Gc are required to implement this trait.
/// The default impl if for a type that impls TraceLeaf
pub unsafe trait Trace {
    const IS_LEAF: bool = true;

    fn trace<T: Tracer>(&self, _tracer: &mut T) {}

    fn dyn_trace<T: Tracer>(ptr: NonNull<()>, tracer: &mut T)
    where
        Self: Sized,
    {
        unsafe { ptr.cast::<Self>().as_ref().trace(tracer) }
    }
}

// ****************************************************************************
// TRACE LEAF IMPLS
// ****************************************************************************

macro_rules! impl_trace_leaf {
    ($($t:ty),*) => {
        $(unsafe impl Trace for $t {})*
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
    f32,
    f64,
    AtomicBool,
    AtomicI8,
    AtomicI16,
    AtomicI32,
    AtomicI64,
    AtomicIsize,
    AtomicU8,
    AtomicU16,
    AtomicU32,
    AtomicU64,
    AtomicUsize
);

unsafe impl<T: TraceLeaf> TraceLeaf for UnsafeCell<T> {}
unsafe impl<T: TraceLeaf> Trace for UnsafeCell<T> {}

unsafe impl<T: TraceLeaf> TraceLeaf for Cell<T> {}
unsafe impl<T: TraceLeaf> Trace for Cell<T> {}

unsafe impl<T: TraceLeaf> TraceLeaf for RefCell<T> {}
unsafe impl<T: TraceLeaf> Trace for RefCell<T> {}

unsafe impl<T: TraceLeaf> TraceLeaf for OnceCell<T> {}
unsafe impl<T: TraceLeaf> Trace for OnceCell<T> {}

// ****************************************************************************
// TRACE IMPLS
// ****************************************************************************
unsafe impl<const N: usize, L: TraceLeaf> TraceLeaf for [L; N] {}
unsafe impl<const N: usize, L: Trace> Trace for [L; N] {
    const IS_LEAF: bool = L::IS_LEAF;

    fn trace<T: Tracer>(&self, tracer: &mut T) {
        for item in self.iter() {
            item.trace(tracer)
        }
    }
}

unsafe impl<T: TraceLeaf> TraceLeaf for Option<T> {}
unsafe impl<T: Trace> Trace for Option<T> {
    const IS_LEAF: bool = T::IS_LEAF;

    fn trace<R: Tracer>(&self, tracer: &mut R) {
        if let Some(value) = self.as_ref() {
            value.trace(tracer)
        }
    }
}

unsafe impl<A: TraceLeaf, B: TraceLeaf> TraceLeaf for Result<A, B> {}
unsafe impl<A: Trace, B: Trace> Trace for Result<A, B> {
    const IS_LEAF: bool = A::IS_LEAF && B::IS_LEAF;

    fn trace<R: Tracer>(&self, tracer: &mut R) {
        match self {
            Ok(res) => res.trace(tracer),
            Err(e) => e.trace(tracer),
        }
    }
}

// Gc is not TraceLeaf!
unsafe impl<'a, T: Trace> Trace for crate::gc::Gc<'a, T> {
    const IS_LEAF: bool = false;

    fn trace<R: Tracer>(&self, tracer: &mut R) {
        unsafe {
            let ptr = self.as_ptr();

            if !ptr.is_null() {
                tracer.trace(NonNull::new_unchecked(ptr))
            }
        }
    }
}

unsafe impl<A: TraceLeaf, B: TraceLeaf> TraceLeaf for (A, B) {}
unsafe impl<A: Trace, B: Trace> Trace for (A, B) {
    const IS_LEAF: bool = A::IS_LEAF && B::IS_LEAF;

    fn trace<R: Tracer>(&self, tracer: &mut R) {
        self.0.trace(tracer);
        self.1.trace(tracer);
    }
}
