mod allocate;
mod allocator;
mod error;
mod gc;
mod gc_cell;
mod gc_ptr;
mod mutator; 
mod trace;
mod tracer;
mod tracer_handle;

pub use mutator::Mutator;
pub use gc_cell::GcCell;
pub use gc_ptr::{GcPtr, StrongGcPtr, GcCellPtr};
pub use trace::Trace;

pub type MutatorScope = mutator::MutatorScope<allocator::Allocator>;
pub type Gc<T> = gc::Gc<allocator::Allocator, T>;

#[cfg(test)]
mod test;
