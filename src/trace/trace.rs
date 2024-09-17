use super::tracer::Tracer;
use crate::gc::{Gc, GcMut, GcNullMut};
use log::debug;
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

    fn trace(&self, _tracer: &mut Tracer) {}

    fn dyn_trace(ptr: NonNull<()>, tracer: &mut Tracer)
    where
        Self: Sized,
    {
        unsafe { ptr.cast::<Self>().as_ref().trace(tracer) }
    }
}

// Gc, GcMut, and GcNullMut are the 3 "core" non TraceLeaf types
// That is to say, every other type that impls Trace + !TraceLeaf
// must also contains one of these types within it.
unsafe impl<'a, T: Trace> Trace for Gc<'a, T> {
    const IS_LEAF: bool = false;

    fn trace(&self, tracer: &mut Tracer) {
        debug!(
            "(TRACER {}) GC TRACE: {:?}",
            tracer.id(),
            self as *const Gc<'a, T>
        );
        let ptr: NonNull<T> = self.as_nonnull();

        tracer.trace(ptr)
    }
}

unsafe impl<'a, T: Trace> Trace for GcMut<'a, T> {
    const IS_LEAF: bool = false;

    fn trace(&self, tracer: &mut Tracer) {
        debug!(
            "(TRACER {}) GC MUT TRACE: {:?}",
            tracer.id(),
            self as *const GcMut<'a, T>
        );
        let ptr: NonNull<T> = self.as_nonnull();

        tracer.trace(ptr)
    }
}

unsafe impl<'a, T: Trace> Trace for GcNullMut<'a, T> {
    const IS_LEAF: bool = false;

    fn trace(&self, tracer: &mut Tracer) {
        debug!(
            "(TRACER {}) GC MUT TRACE: {:?}",
            tracer.id(),
            self as *const GcNullMut<'a, T>
        );

        if let Some(gc_mut) = self.as_option() {
            gc_mut.trace(tracer);
        }
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
unsafe impl<const N: usize, T: TraceLeaf> TraceLeaf for [T; N] {}
unsafe impl<const N: usize, T: Trace> Trace for [T; N] {
    const IS_LEAF: bool = T::IS_LEAF;

    fn trace(&self, tracer: &mut Tracer) {
        for item in self.iter() {
            item.trace(tracer)
        }
    }
}

unsafe impl<T: TraceLeaf> TraceLeaf for Option<T> {}
unsafe impl<T: Trace> Trace for Option<T> {
    const IS_LEAF: bool = T::IS_LEAF;

    fn trace(&self, tracer: &mut Tracer) {
        if let Some(value) = self.as_ref() {
            value.trace(tracer)
        }
    }
}

unsafe impl<A: TraceLeaf, B: TraceLeaf> TraceLeaf for Result<A, B> {}
unsafe impl<A: Trace, B: Trace> Trace for Result<A, B> {
    const IS_LEAF: bool = A::IS_LEAF && B::IS_LEAF;

    fn trace(&self, tracer: &mut Tracer) {
        match self {
            Ok(res) => res.trace(tracer),
            Err(e) => e.trace(tracer),
        }
    }
}

unsafe impl<A: TraceLeaf, B: TraceLeaf> TraceLeaf for (A, B) {}
unsafe impl<A: Trace, B: Trace> Trace for (A, B) {
    const IS_LEAF: bool = A::IS_LEAF && B::IS_LEAF;

    fn trace(&self, tracer: &mut Tracer) {
        self.0.trace(tracer);
        self.1.trace(tracer);
    }
}
