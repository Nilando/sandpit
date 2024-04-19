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
    fn empty_list() {
        let gc: Gc<List<u8>> = Gc::build(|mutator| mutator.alloc_array(0).expect("root allocated"));

        gc.collect();

        gc.mutate(|root, _| {
            assert_eq!(root.len(), 0);
            assert_eq!(root.cap(), 0);
            assert!(root.pop().is_none());
        });
    }

    #[test]
    fn collect_list() {
        let gc: Gc<List<u8>> = Gc::build(|mutator| mutator.alloc_array(8).expect("root allocated"));

        gc.collect();

        gc.mutate(|root, _| {
            assert_eq!(root.len(), 0);
            assert_eq!(root.cap(), 8);
        });
    }

    #[test]
    fn fill_list() {
        let gc: Gc<List<u8>> = Gc::build(|mutator| mutator.alloc_array(8).expect("root allocated"));

        gc.mutate(|root, mutator| {
            for i in 0..8 {
                let item = mutator.alloc(ListItem::Val(i)).unwrap();
                root.push(item);
            }

            assert_eq!(root.len(), 8);
            assert_eq!(root.cap(), 8);
        });
    }

}
