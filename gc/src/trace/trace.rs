use std::ptr::NonNull;
use super::tracer::Tracer;

pub unsafe trait TraceLeaf: 'static {}
pub unsafe trait Trace: 'static {
    fn trace(&self, tracer: &mut Tracer);
    fn dyn_trace(ptr: NonNull<()>, tracer: &mut Tracer)
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
    fn trace(&self, _: &mut Tracer) {
        // TODO: make it so this function is never called
        // this can be done in the proc macro
    }
    fn dyn_trace(_ptr: NonNull<()>, _: &mut Tracer) {
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
unsafe impl<T: TraceLeaf> TraceLeaf for crate::gc_cell::GcCell<T> {}

// ****************************************************************************
// TRACE IMPLS
// ****************************************************************************

unsafe impl<T: Trace> Trace for Option<T> {
    fn trace(&self, tracer: &mut Tracer) {
        self.as_ref().map(|value| value.trace(tracer));
    }
}

unsafe impl<T: Trace> Trace for crate::gc_ptr::GcPtr<T> {
    fn trace(&self, tracer: &mut Tracer) {
        unsafe {
            let ptr = self.as_ptr();

            if !ptr.is_null() {
                tracer.send_unscanned(NonNull::new_unchecked(ptr))
            }
        }
    }
}
