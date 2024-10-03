use super::tracer::Tracer;
use crate::gc::{Gc, GcMut, GcOpt};
use crate::pointee::{GcPointee, Thin};
use std::cell::*;
use std::ptr::NonNull;
use std::sync::atomic::*;

/// Indicates a type contains no Gc references internally.
///
/// Unsafe to impl b/c if the trait does have internal GcPtr's
/// then referenced memory may be freed.
///
/// Allows for this type to be put into the std `Cell` types, as `Cell<T>` only impls
/// `Trace` if its inner type if TraceLeaf. This is because interior mutability requires
/// careful consideration so that the tracers correctly mark all GC references reachable from
/// the root. If a type
pub unsafe trait TraceLeaf: Trace {
    // used by the traceleaf derive to statically assert that all inner types also impl TraceLeaf
    #[doc(hidden)]
    fn __assert_trace_leaf() {}
}

#[doc(hidden)]
pub trait __MustNotDrop {}
#[doc(hidden)]
#[allow(drop_bounds)]
impl<T: Drop> __MustNotDrop for T {}

/// Allows tracer to find all Gc references stored in a type.
///
/// Types allocated in a Gc are required to implement this trait so that tracing reaches all
/// objects. It is unsafe to implement b/c if a Gc reference is not traced, it could result
/// in a Gc value being freed with the reference still existing.
///
/// Can safely be implemented using `#[derive(Trace)]`.
pub unsafe trait Trace: GcPointee {
    #[doc(hidden)]
    const IS_LEAF: bool;

    #[doc(hidden)]
    fn trace(&self, _tracer: &mut Tracer);

    #[doc(hidden)]
    fn dyn_trace(ptr: NonNull<Thin<()>>, tracer: &mut Tracer) {
        <Self as GcPointee>::deref(ptr.cast()).trace(tracer)
    }
}

// trait __TraceTypeMustNotImplDrop {}
// impl<T: Drop> __TraceTypeMustNotImplDrop for T {}

unsafe impl<'gc, T: Trace + ?Sized> Trace for Gc<'gc, T> {
    const IS_LEAF: bool = false;

    fn trace(&self, tracer: &mut Tracer) {
        tracer.mark_and_trace(self.clone());
    }
}

unsafe impl<'gc, T: Trace + ?Sized> Trace for GcMut<'gc, T> {
    const IS_LEAF: bool = false;

    fn trace(&self, tracer: &mut Tracer) {
        Gc::from(self.clone()).trace(tracer)
    }
}

unsafe impl<'gc, T: Trace + ?Sized> Trace for GcOpt<'gc, T> {
    const IS_LEAF: bool = false;

    fn trace(&self, tracer: &mut Tracer) {
        match self.as_option() {
            Some(gc_mut) => gc_mut.trace(tracer),
            None => {}
        }
    }
}

// ****************************************************************************
// TRACE LEAF IMPLS
// ****************************************************************************

macro_rules! impl_trace_leaf {
    ($($t:ty),*) => {
        $(unsafe impl Trace for $t {
            const IS_LEAF: bool = true;

            fn trace(&self, _: &mut Tracer) {}
        })*
        $(unsafe impl TraceLeaf for $t {})*
    };
}

impl_trace_leaf!(
    (),
    bool,
    char,
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
unsafe impl<T: TraceLeaf> Trace for UnsafeCell<T> {
    const IS_LEAF: bool = true;

    fn trace(&self, _: &mut Tracer) {}
}

unsafe impl<T: TraceLeaf> TraceLeaf for Cell<T> {}
unsafe impl<T: TraceLeaf> Trace for Cell<T> {
    const IS_LEAF: bool = true;

    fn trace(&self, _: &mut Tracer) {}
}

unsafe impl<T: TraceLeaf> TraceLeaf for RefCell<T> {}
unsafe impl<T: TraceLeaf> Trace for RefCell<T> {
    const IS_LEAF: bool = true;

    fn trace(&self, _: &mut Tracer) {}
}

unsafe impl<T: TraceLeaf> TraceLeaf for OnceCell<T> {}
unsafe impl<T: TraceLeaf> Trace for OnceCell<T> {
    const IS_LEAF: bool = true;

    fn trace(&self, _: &mut Tracer) {}
}

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

unsafe impl<T: TraceLeaf> TraceLeaf for [T] {}
unsafe impl<T: Trace> Trace for [T] {
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

unsafe impl<A: TraceLeaf, B: TraceLeaf, C: TraceLeaf> TraceLeaf for (A, B, C) {}
unsafe impl<A: Trace, B: Trace, C: Trace> Trace for (A, B, C) {
    const IS_LEAF: bool = A::IS_LEAF && B::IS_LEAF && C::IS_LEAF;

    fn trace(&self, tracer: &mut Tracer) {
        self.0.trace(tracer);
        self.1.trace(tracer);
        self.2.trace(tracer);
    }
}
