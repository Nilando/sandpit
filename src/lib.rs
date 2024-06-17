//! A generational, concurrent, and parallel garbage collected arena that is
//! still under development.
//!
//! A GcArena holds a single generic Root object which must implement the Trace trait.
//! When the GcArena peforms a collection, all memory that is unreachable from the root
//! will be freed.
//
// To build a GcArena, you must pass a callback to the Gc::build method which must return the arena's root object. The Gc::build method also provides a mutator as an argument to allow for the option of allocating the root object within the GcArena.
// ```rust
// use gc::{Gc, Mutator, GcPtr};
//
// // This creates an arena with a usize as the root.
// let gc = Gc::build(|mutator| {
//     mutator.alloc(123).unwrap()
// });
//
// gc.mutate(|root, mutator| {
//     assert_eq!(**root, 123)
// });
// ```
//
// To allocate a type in a GcArena it must meet a few guarantees. First, the type must
// not impl Drop. This is because the trace and sweep collector by design only keeps track of what
// is still reachable from the root, and implicitly frees what is not.
// ```compile_fail
// use gc::{Gc, gc_derive::Trace, GcPtr};
//
// #[derive(Trace)]
// struct Foo;
//
// impl Drop for Foo {
//     fn drop(&mut self) {}
// }
//
// let gc: Gc<GcPtr<Foo>> = Gc::build(|mutator| {
//     mutator.alloc(Foo).unwrap()
// });
// ```
//
// A Gc is Send/Sync only if its root type T is also Send/Sync.
// ```compile_fail
// use gc::{Gc, gc_derive::Trace, GcPtr};
//
// #[derive(Trace)]
// struct Foo;
//
// let gc: Gc<GcPtr<Foo>> = Gc::build(|mutator| {
//     mutator.alloc(Foo).unwrap()
// });
//
// // GcPtr is not send, so gc cannot be send
// std::thread::spawn(|| {
//     gc.mutate(|_, _| {});
// });
// ```
extern crate self as sandpit;

mod allocator;
mod collector;
mod config;
mod error;
mod gc;
mod gc_ptr;
mod metrics;
mod monitor;
mod mutator;
mod trace;

pub use config::GcConfig;
pub use error::GcError;
pub use gc::Gc;
pub use gc_ptr::GcPtr;
pub use metrics::GcMetrics;
pub use mutator::Mutator;
pub use trace::{AssertTraceLeaf, Trace, TraceLeaf};

pub extern crate derive;
#[macro_use]
pub use derive::*;

#[doc(hidden)]
pub use trace::Tracer;

#[cfg(test)]
mod test;
