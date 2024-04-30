//! A generational, concurrent, and parallel garbage collected arena that is
//! still under development.
//!
//! A GcArena holds a single generic Root object which must implement the Trace trait.
//! When the GcArena peforms a collection, all memory that is unreachable from the root
//! will be freed.
//!
//! To build a GcArena, you must pass a callback to the Gc::build method which must return the arena's root object. The Gc::build method also provides a mutator as an argument to allow for the option of allocating the root object within the GcArena.
//! ```rust
//! use gc::{Gc, Mutator, GcPtr};
//!
//! // This creates an arena with a usize as the root.
//! let gc: Gc<GcPtr<usize>> = Gc::build(|mutator| {
//!     mutator.alloc(123).unwrap()
//! });
//!
//! gc.mutate(|root, mutator| {
//!     assert_eq!(**root, 123)
//! });
//! ```
//!
//! To allocate a type in a GcArena it must meet a few guarantees. First, the type must
//! not impl Drop. This is because the trace and sweep collector by design doesn't keep track of
//! what has been freed and more so what is still reachable by the root.
//! ```compile_fail
//! use gc::{Gc, gc_derive::Trace, GcPtr};
//!
//! #[derive(Trace)]
//! struct Foo;
//!
//! impl Drop for Foo {
//!     fn drop(&mut self) {}
//! }
//!
//! let gc: Gc<GcPtr<Foo>> = Gc::build(|mutator| {
//!     mutator.alloc(Foo).unwrap()
//! });
//! ```
//!
//! TODO:
//! - Fixing Bugs and adding tests is the #1 priority!
//! - Right now the monitor is essentially just a placeholder. Actuall experiementing
//!   and work needs to be done in order to make an actual acceptable monitor.
//! - Organize the code into modules
//!     - make the modules more loosley coupled
//! - the exposed Gc type does not need to be generic 
//! - add a way to pass in config to the gc when building
//! - static assert that gc types cannot implement Drop
//! - make gc_ptr send only if its pointed type is send
//! - multi threading tests
#![feature(inline_const)]

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

pub mod collections {
    pub use crate::gc_array::{GcArray, GcArrayIter};
}

pub use error::GcError;
pub use gc_cell::GcCell;
pub use gc_ptr::GcPtr;
pub use mutator::Mutator;
pub use trace::{Trace, TraceLeaf, Tracer};
pub use gc_derive;

pub type Gc<T> = gc::Gc<collector::Controller<allocator::Allocator, T>, monitor::MonitorController>;

#[cfg(test)]
mod test;
