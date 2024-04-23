use super::allocate::{Allocate, Marker};
use super::error::GcError;
use super::gc_ptr::GcPtr;
use super::trace::Trace;
use super::trace_packet::TracePacket;
use super::tracer::TracerWorker;
use super::tracer_controller::TracerController;
use std::ptr::NonNull;
use std::sync::Arc;
use std::alloc::Layout;
use std::ptr::write;
use std::cell::UnsafeCell;

pub trait Mutator {
    fn alloc<T: Trace>(&self, obj: T) -> Result<GcPtr<T>, GcError>;
    fn alloc_layout(&self, layout: Layout) -> Result<GcPtr<()>, GcError>;
    fn write_barrier<T: Trace>(&self, obj: NonNull<T>);
    fn yield_requested(&self) -> bool;
}

pub struct MutatorScope<A: Allocate> {
    allocator: A,
    tracer_controller: Arc<TracerController<A>>,
    new_packet: UnsafeCell<TracePacket<TracerWorker<A>>>,
}

impl<A: Allocate> MutatorScope<A> {
    pub fn new(arena: &A::Arena, tracer_controller: Arc<TracerController<A>>) -> Self {
        let allocator = A::new(arena);

        Self {
            allocator,
            tracer_controller,
            new_packet: UnsafeCell::new(TracePacket::new()),
        }
    }
}

impl<A: Allocate> Drop for MutatorScope<A> {
    fn drop(&mut self) {
        unsafe {
            let packet_ref = &mut *self.new_packet.get();
            self.tracer_controller.push_packet(packet_ref.clone());
        }
    }
}

impl<A: Allocate> Mutator for MutatorScope<A> {
    fn yield_requested(&self) -> bool {
        self.tracer_controller.get_yield_flag()
    }

    fn alloc<T: Trace>(&self, obj: T) -> Result<GcPtr<T>, GcError> {
        let layout = Layout::new::<T>();
        match self.allocator.alloc(layout) {
            Ok(ptr) => {
                unsafe { write(ptr.as_ptr().cast(), obj) }

                Ok(GcPtr::new(ptr.cast()))
            },
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
            },
            Err(_) => todo!(),
        }
    }

    fn write_barrier<T: Trace>(&self, ptr: NonNull<T>) {
        if A::get_mark(ptr).is_new() {
            return;
        }

        unsafe {
            let packet_ref = &mut *self.new_packet.get();

            if packet_ref.is_full() {
                self.tracer_controller.push_packet(packet_ref.clone());
                *packet_ref = TracePacket::new();
            } else {
                packet_ref.push(Some((ptr.cast(), T::dyn_trace))); 
            }
        }
    }
}
