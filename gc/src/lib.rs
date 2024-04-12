mod allocate;
mod allocator;
mod error;
mod gc;
mod gc_cell;
mod gc_ptr;
mod gc_array;
mod mutator;
mod trace;
mod trace_packet;
mod tracer;
mod tracer_controller;
mod monitor;
mod collector;

pub use error::GcError;
pub use gc_cell::GcCell;
pub use gc_ptr::{GcCellPtr, GcPtr, StrongGcPtr};
pub use gc_array::GcArray;
pub use mutator::Mutator;
pub use trace::Trace;
pub use tracer::Tracer;

pub type Gc<T> = gc::Gc<collector::Controller<allocator::Allocator, T>, monitor::MonitorController>;

#[cfg(test)]
mod test;
