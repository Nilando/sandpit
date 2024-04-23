//! A generational, concurrent, and parallel garbage collected arena that is
//! still under development.
//!
//! ```rust
//! use gc::{Gc, Mutator};
//!
//! let gc = Gc::build(|mutator| {
//!     *mutator.alloc(123).unwrap()
//! });
//!
//! gc.mutate(|root, mutator| {
//!     assert_eq!(*root, 123)
//! });
//! ```
//!
//! TODO:
//! - Fixing Bugs is the #1 priority!
//!     - Miri is detecting a few concurrency issues
//! - Right now the monitor is essentially just a placeholder. Actuall experiementing
//!   and work needs to be done in order to make an actual acceptable monitor.
//! - Organize the code into modules
//! - the exposed Gc type does not need to be generic 
//! - add a way to pass in config to the gc when building

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

pub mod collections {
    pub use crate::gc_array::{GcArray, GcArrayIter};
}

pub use error::GcError;
pub use gc_cell::GcCell;
pub use gc_ptr::GcPtr;
pub use mutator::Mutator;
pub use trace::Trace;
pub use tracer::Tracer;

pub type Gc<T> = gc::Gc<collector::Controller<allocator::Allocator, T>, monitor::MonitorController>;

#[cfg(test)]
mod test;
