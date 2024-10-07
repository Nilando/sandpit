//! A concurrent, generational, trace and sweep garbage collected arena.
//!
//! ## High Level Example
//!
//! ## Creating An Arena
//!
//! All garbage collection in Sandpit happens within an arena. Therefore, to
//! be begin we can start with creating a new arena.
//!
//! This can be done like so..
//! ```rust
//! use sandpit::{Arena, Root};
//! # use sandpit::{Trace, Mutator};
//! # #[derive(Trace)]
//! # struct MyRoot;
//! # impl MyRoot {
//! #   fn new(mutator: &Mutator) -> Self { Self }
//! # }
//!
//! // This creates an arena with a `MyRoot` as the root.
//! let gc: Arena<Root![MyRoot]> = Arena::new(|mutator| {
//!     MyRoot::new(mutator)
//! });
//! ```
//! There are two big things to unpack here:
//! * An `Arena` is generic on its single Root value which it holds.
//! * The Root of an `Arena` must be a Higher Kinded Type(HKT).
//!
//! This is explained in much further depth in [`sandpit::Arena`].
//!
//! ## Trace Trait
//! For a type to be GC'ed it is required to impl [`Trace`]
//! which can be safely derived as long as all inner types also impl [`Trace`].
//!
//! ```rust
//! use sandpit::{Trace, gc::{Gc, GcMut, GcOpt}};
//! # #[derive(Trace)]
//! # struct A;
//! # #[derive(Trace)]
//! # struct B;
//! # #[derive(Trace)]
//! # struct C;
//!
//! #[derive(Trace)]
//! enum Value<'gc> {
//!     // GC values must be branded with a mutation lifetime
//!     // to ensure freeing memory can happen safely.
//!     A(Gc<'gc, A>), // Immutable pointer, essentially a &'gc T.
//!     B(GcMut<'gc, B>), // Mutable pointer, can be updated to point at something else via a write barrier.
//!     C(GcOpt<'gc, C>), // Optionally null pointer that is also mutable. Can be unwrapped into a GcMut.
//! }
//! // All inner values must be trace, therefore types A, B, and T must impl Trace as well!
//! ```
//! Essentially when a value is traced the tracer will mark the value as live,
//! and call trace on all its inner pointers to GC values.
//!
//! There are 3 types of GC pointers:
//! * [`gc::Gc`]
//! * [`gc::GcMut`]
//! * [`gc::GcOpt`]
//!
//! A type may also derive [`TraceLeaf`], if it contains no GC pointers.
//! [`TraceLeaf`] allows for easier interior mutability.
//!
//! ## Mutating the Arena
//! Once you have your arena and your traceable types, you can begin allocating
//! them in the arena by calling [`Arena::mutate`]. Within a mutation
//! we can essentially do 3 important things:
//! * Access all data reaachable from the root.
//! * Create new garbage collected values.
//! * Update Gc pointers to point to new values via a [`WriteBarrier`].
//! ```rust
//! use sandpit::{Trace, gc::GcMut};
//!
//! # use sandpit::{Arena, Root, Mutator, WriteBarrier};
//! # let arena: Arena<Root![GcMut<'_, usize>]> = Arena::new(|mutator| {
//! #     GcMut::new(mutator, 0usize)
//! # });
//! # fn traverse(root: &usize) {}
//! arena.mutate(|mutator, root| {
//!     // We can access everything reachable from the root.
//!     traverse(root);
//!
//!     // We can allocate new Gc values.
//!     // Here is a pointer, to a pointer, to a bool!
//!     let gc_mut = GcMut::new(mutator,
//!         GcMut::new(mutator, true)
//!     );
//!
//!     // We can mutate existing inner GcMut and GcOpt pointers.
//!     gc_mut.write_barrier(mutator, |barrier| {
//!         barrier.set(GcMut::new(mutator, false));
//!     })
//! });
//! ```
//!
//! ## Collection and Yielding
//!
//! In order for the Gc to free memory, and do so safely, all mutations must
//! exit. Therefore, if a mutation involves a continuous loop of instructions,
//! it must exit it's mutation every so often to allow the GC to free memory.
//!
//! The mutator exposes a signal([`Mutator::gc_yield`]) which indicates if it is ready to free memory,
//! and that the mutation should end.
//!
//! ```rust
//! # use sandpit::{Arena, Root, Mutator, WriteBarrier, gc::GcMut};
//! # let arena: Arena<Root![GcMut<'_, usize>]> = Arena::new(|mutator| {
//! #     GcMut::new(mutator, 0usize)
//! # });
//! # fn allocate_stuff(mutator: &Mutator, root: &usize) {
//! #   for i in 0..100 {
//! #       GcMut::new(mutator, 0);
//! #   }
//! # }
//! arena.mutate(|mutator, root| loop {
//!     // during this function it is likely the the GC will concurrently begin tracing!
//!     allocate_stuff(mutator, root);
//!
//!     if mutator.gc_yield() {
//!         // the mutator is signaling to us that memory is ready to be freed so we should leave the mutation context
//!         break;
//!     } else {
//!         // if the mutator isn't signaling for us to yield then we
//!         // are fine to go on allocating more garbage
//!     }
//! });
//! ```
//!
//! ***WARNING:*** If a mutation continously runs without occasionally checking
//! the yield signal, memory cannot be freed!
//!
extern crate self as sandpit;

pub mod gc;

mod allocator;
mod arena;
mod barrier;
mod collector;
mod config;
mod header;
mod metrics;
mod monitor;
mod mutator;
mod pointee;
mod time_slicer;
mod trace;

pub use arena::Arena;
pub use barrier::WriteBarrier;
pub use config::Config;

/// Re-exported from ForLt. Used in making the root of an arena.
pub use higher_kinded_types::ForLt as Root;

pub use metrics::Metrics;
pub use mutator::Mutator;
pub use sandpit_derive::{Trace, TraceLeaf};
pub use trace::{Trace, TraceLeaf};

#[doc(hidden)]
pub use trace::{Tracer, __MustNotDrop};
