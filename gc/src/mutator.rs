use super::allocate::{Allocate, Marker};
use super::error::GcError;
use super::gc_array::GcArray;
use super::gc_ptr::GcPtr;
use super::trace::Trace;
use super::trace_packet::TracePacket;
use super::tracer::TracerWorker;
use super::tracer_controller::TracerController;
use std::ptr::NonNull;
use std::sync::Arc;
use std::alloc::Layout;
use std::ptr::write;
use std::mem::{size_of, align_of};

pub trait Mutator {
    fn alloc<T: Trace>(&self, obj: T) -> Result<GcPtr<T>, GcError>;
    fn alloc_array<T: Trace>(&self, capacity: usize) -> Result<GcArray<T>, GcError>;
    fn write_barrier<T: Trace>(&mut self, obj: NonNull<T>);
    fn yield_requested(&mut self) -> bool;
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
        if let Some(packet) = self.new_packet.take() {
            self.tracer_controller.push_packet(packet)
        }
    }
}

impl<A: Allocate> Mutator for MutatorScope<A> {
    fn yield_requested(&mut self) -> bool {
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

    fn alloc_array<T: Trace>(&self, capacity: usize) -> Result<GcArray<T>, GcError> {
        let layout = unsafe { Layout::from_size_align_unchecked(size_of::<T>() * capacity, align_of::<T>()) };
        match self.allocator.alloc(layout) {
            Ok(ptr) => {
                if capacity == 0 {
                    //return Ok(GcArray::new(GcPtr::new(None), 0, capacity))
                    todo!()
                }

                let gc_ptr: GcPtr<T> = GcPtr::new(ptr.cast());

                Ok(GcArray::new(gc_ptr, 0, capacity))
            },
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
