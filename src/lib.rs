//! A generational, concurrent, and parallel garbage collected arena that is
//! still under development.
//!
//! A GcArena holds a single generic Root object which must implement the Trace trait.
//! When the GcArena peforms a collection, all memory that is unreachable from the root
//! will be freed.
//! To build a GcArena, you must pass a callback to the GcArena::build method which must return the arena's root object. The GcArena::build method also provides a mutator as an argument to allow for the option of allocating the root object within the GcArena.
//! ```rust
//! use sandpit::{GcArena, Mutator, Gc};
//! use higher_kinded_types::ForLt;
//!
//! // This creates an arena with a usize as the root.
//! let gc: GcArena<ForLt![Gc<'_, usize>]> = GcArena::new(|mu| {
//!     Gc::new(mu, 123)
//! });
//!
//! gc.mutate(|mu, root| {
//!     assert_eq!(**root, 123)
//! });
//! ```
//!
//! To allocate a type in a GcArena it must meet a few guarantees. First, the type must
//! not impl Drop. This is because the trace and sweep collector by design only keeps track of what
//! is still reachable from the root, and implicitly frees what is not.
//! ```
//! use sandpit::{GcArena, Trace, Gc};
//! use higher_kinded_types::ForLt;
//!
//! #[derive(Trace)]
//! struct Foo {
//!     foo: usize
//! }
//!
//! let gc: GcArena<ForLt![Gc<'_, Foo>]> = GcArena::new(|mu| {
//!     Gc::new(mu, Foo { foo: 69 })
//! });
//!
//! gc.mutate(|mu, root| {
//!     assert_eq!(root.foo, 69)
//! });
//! ```
extern crate self as sandpit;

mod allocator;
mod collector;
mod config;
mod error;
mod gc_arena;
mod arena;
mod gc;
mod metrics;
mod monitor;
mod mutator;
mod trace;

pub use config::GcConfig;
pub use error::GcError;
pub use gc_arena::GcArena;
pub use gc::Gc;
pub use metrics::GcMetrics;
pub use mutator::Mutator;
pub use sandpit_derive::{Trace, TraceLeaf};
pub use trace::{AssertTraceLeaf, Trace, TraceLeaf};

#[doc(hidden)]
pub use trace::Tracer;

#[cfg(test)]
mod test;
