use gc::{Trace, Gc, GcArray, GcCell, GcCellPtr, GcError, GcPtr, Mutator};
use gc_derive::Trace;

#[derive(Trace)]
struct List<T: Trace> {
    array: GcArray<ListItem<T>>,
}

#[derive(Trace)]
enum ListItem<T: Trace> {
    Val(T),
    List(List<T>),
}

impl<T: Trace> List<T> {
    pub fn alloc<M: Mutator>(mutator: &mut M) -> Result<Self, GcError> {
        let array = mutator.alloc_array::<ListItem<T>>(0)?;

        Ok(Self { array })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root_node() {
        let gc: Gc<List<u8>> = Gc::build(|mutator| List::alloc(mutator).expect("root allocated"));

        gc.collect();

        assert!(true);
    }
}
