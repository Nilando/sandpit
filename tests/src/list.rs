use gc::{Trace, Gc, GcArray, GcCell, GcError, GcPtr, Mutator};
use gc_derive::Trace;

type List<T> = GcArray<ListItem<T>>;

#[derive(Trace)]
pub enum ListItem<T: Trace> {
    Val(T),
    List(List<T>),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alloc_and_collect_empty_list() {
        let gc: Gc<List<u8>> = Gc::build(|mutator| mutator.alloc_array(0).expect("root allocated"));

        gc.collect();

        gc.mutate(|root, _| {
            assert_eq!(root.len(), 0);
            assert_eq!(root.cap(), 0);
        });
    }

    #[test]
    fn alloc_and_collect_list_with_capacity() {
        let gc: Gc<List<u8>> = Gc::build(|mutator| mutator.alloc_array(8).expect("root allocated"));

        gc.collect();

        gc.mutate(|root, _| {
            assert_eq!(root.len(), 0);
            assert_eq!(root.cap(), 8);
        });
    }

    #[test]
    fn push_zero_capacity_list() {
        let gc: Gc<List<u8>> = Gc::build(|mutator| mutator.alloc_array(0).expect("root allocated"));

        gc.mutate(|root, mutator| {
            for i in 0..8 {
                root.push(ListItem::Val(i));
            }

            assert_eq!(root.len(), 8);
            assert_eq!(root.cap(), 8);
        });
    }

    #[test]
    fn push_list() {
        let gc: Gc<List<u8>> = Gc::build(|mutator| mutator.alloc_array(8).expect("root allocated"));

        gc.mutate(|root, mutator| {
            for i in 0..8 {
                root.push(ListItem::Val(i));
            }

            assert_eq!(root.len(), 8);
            assert_eq!(root.cap(), 8);
        });
    }
}
