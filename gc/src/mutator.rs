use super::allocate::{Allocate, Marker};
use super::error::GcError;
use super::gc_ptr::GcPtr;
use super::trace::Trace;
use super::tracer::TracerWorker;
use super::trace_packet::TracePacket;
use super::tracer_controller::TracerController;
use super::gc_array::GcArray;
use std::ptr::NonNull;
use std::sync::Arc;

pub trait Mutator {
    fn alloc<T: Trace>(&self, obj: T) -> Result<GcPtr<T>, GcError>;
    fn alloc_array<T: Trace>(&self, capacity: usize) -> Result<GcArray<T>, GcError>;
    fn write_barrier<T: Trace>(&mut self, obj: NonNull<T>);
    fn yield_requested(&mut self) -> bool;
    // fn alloc_sized(&mut self, len: u32) -> Result<NonNull<u8>, Self::Error>;
    // fn alloc_vec<T: Trace>(len, capacity, T) -> GcVec<T>;
    // fn alloc_grow
    // fn alloc_shrink
}

pub struct MutatorScope<A: Allocate> {
    allocator: A,
    tracer_controller: Arc<TracerController<A>>,
    new_packet: Option<TracePacket<TracerWorker<A>>>,
}

impl<A: Allocate> MutatorScope<A> {
    pub fn new(arena: &A::Arena, tracer_controller: Arc<TracerController<A>>) -> Self {
        let allocator = A::new(arena);

        Self {
            allocator,
            tracer_controller,
            new_packet: None,
        }
    }
}

impl<A: Allocate> Drop for MutatorScope<A> {
    fn drop(&mut self) {
        if let Some(packet) = self.new_packet.take() { self.tracer_controller.push_packet(packet) }
    }
}

impl<A: Allocate> Mutator for MutatorScope<A> {
    fn yield_requested(&mut self) -> bool {
        self.tracer_controller.get_yield_flag()
    }

    fn alloc<T: Trace>(&self, obj: T) -> Result<GcPtr<T>, GcError> {
        match self.allocator.alloc(obj) {
            Ok(ptr) => Ok(GcPtr::new(ptr)),
            Err(_) => todo!(),
        }
    }

    fn alloc_array<T: Trace>(&self, capacity: usize) -> Result<GcArray<T>, GcError> {
        let alloc_size = std::mem::size_of::<T>() * capacity;

        match self.allocator.alloc_sized(alloc_size as u32) {
            Ok(ptr) => {
                let ptr: GcPtr<T> = GcPtr::new(ptr.cast());
                Ok(GcArray::new(ptr.into(), 0, capacity))
            }
            Err(_) => todo!(),
        }
    }

    fn write_barrier<T: Trace>(&mut self, ptr: NonNull<T>) {
        if A::get_mark(ptr).is_new() {
            return;
        }

        match self.new_packet.take() {
            Some(mut packet) => {
                if packet.is_full() {
                    self.tracer_controller.push_packet(packet);
                } else {
                    packet.push(Some((ptr.cast(), T::dyn_trace)));
                    self.new_packet = Some(packet);
                }
            }
            None => {
                let mut packet = TracePacket::new();
                packet.push(Some((ptr.cast(), T::dyn_trace)));
                self.new_packet = Some(packet);
            }
        }
    }
}
