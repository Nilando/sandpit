//! A generational, concurrent, and parallel garbage collected arena that is
//! still under development.
//!
//! A Arena holds a single generic Root object which must implement the Trace trait.
//! When the Arena peforms a collection, all memory that is unreachable from the root
//! will be freed.
//! To build a Arena, you must pass a callback to the Arena::build method which must return the arena's root object. The Arena::build method also provides a mutator as an argument to allow for the option of allocating the root object within the Arena.
//! ```rust
//! use sandpit::{Arena, Mutator, Gc, Root};
//!
//! // This creates an arena with a Gc<usize> as the root.
//! let gc: Arena<Root![Gc<'_, usize>]> = Arena::new(|mu| {
//!     Gc::new(mu, 123)
//! });
//!
//! gc.mutate(|mu, root| {
//!     assert_eq!(**root, 123)
//! });
//! ```
//!
//! To allocate a type in a Arena it must meet a few guarantees. First, the type must
//! not impl Drop. This is because the trace and sweep collector by design only keeps track of what
//! is still reachable from the root, and implicitly frees what is not.
//! ```
//! use sandpit::{Arena, Trace, Gc, Root};
//!
//! #[derive(Trace)]
//! struct Foo {
//!     foo: usize
//! }
//!
//! let gc: Arena<Root![Gc<'_, Foo>]> = Arena::new(|mu| {
//!     Gc::new(mu, Foo { foo: 69 })
//! });
//!
//! gc.mutate(|mu, root| {
//!     assert_eq!(root.foo, 69)
//! });
//! ```
extern crate self as sandpit;

mod allocator;
mod arena;
mod barrier;
mod collector;
mod config;
mod gc;
mod header;
mod metrics;
mod monitor;
mod mutator;
mod raw_allocator;
mod trace;
mod time_slicer;
//mod trace_vec;

pub use arena::Arena;
pub use barrier::WriteBarrier;
pub use config::GcConfig;
pub use gc::{Gc, GcMut, GcNullMut};
pub use higher_kinded_types::ForLt as Root;
pub use metrics::GcMetrics;
pub use mutator::Mutator;
pub use sandpit_derive::{Trace, TraceLeaf};
pub use trace::{Trace, TraceLeaf};

#[doc(hidden)]
pub use trace::Tracer;
