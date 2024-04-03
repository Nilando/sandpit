mod allocate;
mod allocator;
mod error;
mod gc;
mod gc_cell;
mod gc_ptr;
mod mutator;
mod trace;
mod trace_packet;
mod tracer;
mod tracer_controller;
mod tracer_handle;

pub use error::GcError;
pub use gc_cell::GcCell;
pub use gc_ptr::{GcCellPtr, GcPtr, StrongGcPtr};
pub use mutator::Mutator;
pub use trace::Trace;
pub use tracer::Tracer;

pub type Gc<T> = gc::Gc<allocator::Allocator, T>;

#[cfg(test)]
mod test;
