use super::*;
use std::ptr::NonNull;

pub unsafe trait TraceLeaf {}
pub unsafe trait Trace {
    fn trace<T: Tracer>(&self, tracer: &mut T);
    fn dyn_trace<T: Tracer>(ptr: NonNull<()>, tracer: &mut T);
}

unsafe impl<O: Trace> Trace for Option<O> {
    fn trace<T: Tracer>(&self, tracer: &mut T) {
        match self {
            Some(value) => value.trace(tracer),
            None => {}
        }
    }

    fn dyn_trace<T: Tracer>(ptr: NonNull<()>, tracer: &mut T) {
        unsafe { ptr.cast::<Self>().as_ref().trace(tracer) }
    }
}

unsafe impl TraceLeaf for usize {}
unsafe impl Trace for usize {
    fn trace<T: Tracer>(&self, tracer: &mut T) {}
    fn dyn_trace<T: Tracer>(ptr: NonNull<()>, tracer: &mut T) {}
}

unsafe impl<T: TraceLeaf> TraceLeaf for gc_cell::GcCell<T> {}
unsafe impl<T: TraceLeaf> Trace for gc_cell::GcCell<T> {
    fn trace<B: Tracer>(&self, tracer: &mut B) {}
    fn dyn_trace<B: Tracer>(ptr: NonNull<()>, tracer: &mut B) {}
}

unsafe impl<O: Trace> Trace for gc_ptr::GcPtr<O> {
    fn trace<T: Tracer>(&self, tracer: &mut T) {
        tracer.send_unscanned(self.as_ptr())
    }
    fn dyn_trace<T: Tracer>(ptr: NonNull<()>, tracer: &mut T) {
        unsafe { ptr.cast::<Self>().as_ref().trace(tracer) }
    }
}
unsafe impl<T: Trace> Trace for gc_ptr::GcCellPtr<T> {
    fn trace<A: Tracer>(&self, tracer: &mut A) {
        self.as_ptr().map(|ptr| tracer.send_unscanned(ptr));
    }
    fn dyn_trace<A: Tracer>(ptr: NonNull<()>, tracer: &mut A) {
        unsafe { ptr.cast::<Self>().as_ref().trace(tracer) }
    }
}

// We need a type to allow for interior mutability of gc values, yet also helps
// maintain the writer barrier when mutating nested gc refs.
//
// To do this we must implement our our interior mutability pattern while excluding,
// the existing cell types from being allocated into the gc arena, essentially
// enforcing our new interior mutability pattern.
//
// The two main types in interation here that allow interior mutatbility inside the
// gc are the GcCell<T> and GcPtr<T>.
//
// As well as the two traits TraceNode, TraceLeaf. Both traits are unsafe to impl
// by hand but can safely be implemented via the macro trace.
//
// Now if we ever need to mutate a traceleaf type, we don't need to worry about
// the write barrier as that type contains no references.
//
// However, if a type is of type TraceNode, than we must make sure that updating
//
//
//  struct A {
//      inner: Struct B { // B contains a nonleaf, meaning it can't go into a gccell
//          ptr: GcPtr
//          cell_ptr: GcCellPtr
//      }
//      num: GcCell<usize> // a usize is a leaf so it can go in a gccell
//  }
//
// If a type contains GcPtr's or a type that implment TraceNode, than that type
// will implement TraceNode, else the type is TraceLeaf.
//
// With a GcCell<T>, T must impl TraceLeaf, and updating a GcCell will therefore,
// never need to trigger a write barrier.
//
// Updating a GcPtr will always need to trigger a write barrier
