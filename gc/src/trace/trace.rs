use super::tracer::Tracer;
use std::ptr::NonNull;
use std::cell::Cell;

/// TraceLeaf is a sub-trait of Trace which ensures its implementor does not contain
/// any GcPtr's.
pub unsafe trait TraceLeaf: 'static {}
/// Types allocated in a Gc are required to implement this trait.
pub unsafe trait Trace: 'static {
    fn trace<T: Tracer>(&self, tracer: &mut T);
    fn dyn_trace<T: Tracer>(ptr: NonNull<()>, tracer: &mut T)
    where
        Self: Sized,
    {
        unsafe { ptr.cast::<Self>().as_ref().trace(tracer) }
    }
    fn needs_trace() -> bool {
        true
    }
}

// ****************************************************************************
// TRACE LEAF IMPLS
// ****************************************************************************

unsafe impl<L: TraceLeaf> Trace for L {
    fn trace<T: Tracer>(&self, _: &mut T) {
        // TODO: This ensure the function is never compiled
        // const { assert!(false) }
    }
    fn dyn_trace<T: Tracer>(_ptr: NonNull<()>, _: &mut T) {
        unimplemented!()
    }
    fn needs_trace() -> bool {
        false
    }
}

unsafe impl TraceLeaf for () {}
unsafe impl TraceLeaf for bool {}
unsafe impl TraceLeaf for u8 {}
unsafe impl TraceLeaf for u16 {}
unsafe impl TraceLeaf for u32 {}
unsafe impl TraceLeaf for u64 {}
unsafe impl TraceLeaf for u128 {}
unsafe impl TraceLeaf for usize {}
unsafe impl TraceLeaf for i8 {}
unsafe impl TraceLeaf for i16 {}
unsafe impl TraceLeaf for i32 {}
unsafe impl TraceLeaf for i64 {}
unsafe impl TraceLeaf for i128 {}
unsafe impl TraceLeaf for isize {}
unsafe impl TraceLeaf for std::sync::atomic::AtomicUsize {}

// ****************************************************************************
// TRACE IMPLS
// ****************************************************************************

unsafe impl<T: Trace> Trace for Option<T> {
    fn trace<R: Tracer>(&self, tracer: &mut R) {
        if let Some(value) = self.as_ref() {
            value.trace(tracer)
        }
    }
}

unsafe impl<T: Trace> Trace for crate::gc_ptr::GcPtr<T> {
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
