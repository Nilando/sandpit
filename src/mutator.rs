use super::allocator::{Allocate, GenerationalArena, Marker};
use super::error::GcError;
use super::gc_ptr::GcPtr;
use super::trace::{Trace, TraceMarker, TracePacket, TracerController};

use std::alloc::Layout;
use std::ptr::write;
use std::sync::Mutex;
use std::sync::RwLockReadGuard;

/// An interface for the mutator type which allows for interaction with the
/// Gc inside a `gc.mutate(...)` context.
pub trait Mutator {
    fn alloc<T: Trace>(&self, obj: T) -> Result<GcPtr<T>, GcError>;

    // TODO: remove this method! or make it private somehow.
    fn alloc_layout(&self, layout: Layout) -> Result<GcPtr<()>, GcError>;

    fn write_barrier<A: Trace, B: Trace>(
        &self,
        update: GcPtr<A>,
        new: GcPtr<B>,
        callback: fn(&A) -> &GcPtr<B>,
    );
    fn rescan<T: Trace>(&self, ptr: GcPtr<T>);
    fn yield_requested(&self) -> bool;
}

pub struct MutatorScope<'scope, A: Allocate> {
    allocator: A,
    tracer_controller: &'scope TracerController<TraceMarker<A>>,
    trace_packet: Mutex<TracePacket<TraceMarker<A>>>,
    _lock: RwLockReadGuard<'scope, ()>,
}

impl<'scope, A: Allocate> MutatorScope<'scope, A> {
    pub fn new(
        arena: &A::Arena,
        tracer_controller: &'scope TracerController<TraceMarker<A>>,
        _lock: RwLockReadGuard<'scope, ()>,
    ) -> Self {
        let allocator = A::new(arena);

        Self {
            allocator,
            tracer_controller,
            // TODO: this could probably be something other than a mutex
            trace_packet: Mutex::new(TracePacket::new()),
            _lock,
        }
    }
}

impl<'scope, A: Allocate> Drop for MutatorScope<'scope, A> {
    fn drop(&mut self) {
        let packet_ref = &mut *self.trace_packet.lock().unwrap();
        self.tracer_controller.push_packet(packet_ref.clone());
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

    fn write_barrier<X: Trace, Y: Trace>(
        &self,
        update_ptr: GcPtr<X>,
        new_ptr: GcPtr<Y>,
        callback: fn(&X) -> &GcPtr<Y>,
    ) {
        unsafe {
            let ptr = update_ptr.as_nonnull();
            let old_ptr = callback(ptr.as_ref());
            // TODO: this might work but new_ptr could be null!
            // let need_rescan = !self.allocator.is_old(new_ptr.as_nonnull());

            old_ptr.unsafe_set(new_ptr);

            //if need_rescan {
            self.rescan(update_ptr);
            //}
        }
    }

    fn rescan<T: Trace>(&self, gc_ptr: GcPtr<T>) {
        let ptr = unsafe { gc_ptr.as_nonnull() };

        if !self.allocator.is_old(ptr) {
            return;
        }

        let new = <<A as Allocate>::Arena as GenerationalArena>::Mark::new();
        A::set_mark(ptr, new);

        let packet_ref = &mut *self.trace_packet.lock().unwrap();

        if packet_ref.is_full() {
            self.tracer_controller.push_packet(packet_ref.clone());
            packet_ref.drain();
        }

        packet_ref.push(ptr);
    }
}