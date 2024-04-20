mod allocate;
mod allocator;
mod collector;
mod error;
mod gc;
mod gc_array;
mod gc_cell;
mod gc_ptr;
mod monitor;
mod mutator;
mod trace;
mod trace_metrics;
mod trace_packet;
mod tracer;
mod tracer_controller;

pub use error::GcError;
pub use gc_array::{GcArray, GcArrayIter};
pub use gc_cell::GcCell;
pub use gc_ptr::{GcPtr};
pub use mutator::Mutator;
pub use trace::Trace;
pub use tracer::Tracer;

pub type Gc<T> = gc::Gc<collector::Controller<allocator::Allocator, T>, monitor::MonitorController>;

#[cfg(test)]
mod test;
