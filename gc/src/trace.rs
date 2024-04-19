use super::*;
use std::ptr::NonNull;

pub unsafe trait TraceLeaf: 'static {}
pub unsafe trait Trace: 'static {
    fn trace<T: Tracer>(&self, tracer: &mut T);
    fn dyn_trace<T: Tracer>(ptr: NonNull<()>, tracer: &mut T)
    where
        Self: Sized,
    {
        unsafe { ptr.cast::<Self>().as_ref().trace(tracer) }
    }
    fn needs_trace() -> bool { true }
}

// ****************************************************************************
// TRACE LEAF IMPLS
// ****************************************************************************

unsafe impl<T: TraceLeaf> Trace for T {
    fn trace<U: Tracer>(&self, _tracer: &mut U) {
        // this will be called,but never
    }
    fn dyn_trace<U: Tracer>(_ptr: NonNull<()>, _tracer: &mut U) {
        unimplemented!()
    }
    fn needs_trace() -> bool { false }
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
unsafe impl<T: TraceLeaf> TraceLeaf for gc_cell::GcCell<T> {}

// ****************************************************************************
// TRACE IMPLS
// ****************************************************************************

unsafe impl<T: Trace> Trace for Option<T> {
    fn trace<U: Tracer>(&self, tracer: &mut U) {
        self.as_ref().map(|value| value.trace(tracer));
    }
}

unsafe impl<T: Trace> Trace for gc_ptr::GcPtr<T> {
    fn trace<U: Tracer>(&self, tracer: &mut U) {
        unsafe {
            let ptr = self.as_ptr();

            if !ptr.is_null() {
                tracer.send_unscanned(NonNull::new_unchecked(ptr))
            }
        }
    }
}
