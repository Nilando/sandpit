use super::allocator::{Allocate, GenerationalArena, Marker};
use super::error::GcError;
use super::gc_ptr::GcPtr;
use super::trace::{Trace, TraceMarker, TracePacket, TracerController};

use std::alloc::Layout;
use std::cell::UnsafeCell;
use std::ptr::write;
use std::ptr::NonNull;

pub trait Mutator {
    fn alloc<T: Trace>(&self, obj: T) -> Result<GcPtr<T>, GcError>;
    fn alloc_layout(&self, layout: Layout) -> Result<GcPtr<()>, GcError>;
    fn write_barrier<T: Trace>(&self, obj: NonNull<T>);
    fn yield_requested(&self) -> bool;
}

pub struct MutatorScope<'scope, A: Allocate> {
    allocator: A,
    tracer_controller: &'scope TracerController<TraceMarker<A>>,
    trace_packet: UnsafeCell<TracePacket<TraceMarker<A>>>,
}

impl<'scope, A: Allocate> MutatorScope<'scope, A> {
    pub fn new(
        arena: &A::Arena,
        tracer_controller: &'scope TracerController<TraceMarker<A>>,
    ) -> Self {
        let allocator = A::new(arena);

        Self {
            allocator,
            tracer_controller,
            trace_packet: UnsafeCell::new(TracePacket::new()),
        }
    }
}

impl<'scope, A: Allocate> Drop for MutatorScope<'scope, A> {
    fn drop(&mut self) {
        unsafe {
            let packet_ref = &mut *self.trace_packet.get();
            self.tracer_controller.push_packet(packet_ref.clone());
        }
    }
}

impl<'scope, A: Allocate> Mutator for MutatorScope<'scope, A> {
    fn yield_requested(&self) -> bool {
        // TODO: this should also check how much memory is left,
        // as well as how long the tracer has been running
        self.tracer_controller.yield_flag()
    }

    fn alloc<T: Trace>(&self, obj: T) -> Result<GcPtr<T>, GcError> {
        const {
            assert!(
                !std::mem::needs_drop::<T>(),
                "A type must not need dropping to be allocated in a GcArena"
            )
        };

        let layout = Layout::new::<T>();
        match self.allocator.alloc(layout) {
            Ok(ptr) => {
                unsafe { write(ptr.as_ptr().cast(), obj) }

                Ok(GcPtr::new(ptr.cast()))
            }
            Err(_) => todo!(),
        }
    }

    fn alloc_layout(&self, layout: Layout) -> Result<GcPtr<()>, GcError> {
        match self.allocator.alloc(layout) {
            Ok(ptr) => {
                for i in 0..layout.size() {
                    unsafe { write(ptr.as_ptr().add(i), 0) }
                }

                Ok(GcPtr::new(ptr.cast()))
            }
            Err(_) => todo!(),
        }
    }

    fn write_barrier<T: Trace>(&self, ptr: NonNull<T>) {
        if A::get_mark(ptr).is_new() {
           // return;
        }

        let rescan_mark = <<A as Allocate>::Arena as GenerationalArena>::Mark::new_rescan();
        A::set_mark(ptr, rescan_mark);

        unsafe {
            let packet_ref = &mut *self.trace_packet.get();

            if packet_ref.is_full() {
                self.tracer_controller.push_packet(packet_ref.clone());
                packet_ref.drain();
            }

            packet_ref.push(ptr);
        }
    }
}
