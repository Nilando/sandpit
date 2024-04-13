use gc::{Gc, GcCell, GcCellPtr, GcArray, GcError, GcPtr, Mutator};
use gc_derive::Trace;

#[derive(Trace)]
struct GcString {
    array: GcArray<u8>
}

#[derive(Trace)]
struct List {
    array: GcArray<ListItem>
}

#[derive(Trace)]
enum ListItem {
    Num(isize),
    String(GcString),
    List(List),
}

impl List {
    pub fn alloc<M: Mutator>(mutator: &mut M) -> Result<Self, GcError> {
        let array = mutator.alloc_array::<ListItem>(0)?;

        Ok(Self { array })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root_node() {
        let gc: Gc<List> = Gc::build(|mutator| {
            List::alloc(mutator).expect("root allocated")
        });

        gc.collect();

        assert!(true);
    }
}
