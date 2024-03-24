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

use allocator::Allocator;
use gc::Gc as GenericGc;
use mutator::MutatorScope as GenericMutatorScope;

pub use mutator::{MutatorRunner, Mutator};
pub use gc_cell::GcCell;
pub use gc_ptr::GcPtr;
pub use trace::Trace;
pub type Gc = GenericGc<Allocator>;
pub type MutatorScope = GenericMutatorScope<Allocator>;

#[cfg(test)]
mod test;
