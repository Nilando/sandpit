#[cfg(test)]
mod tests {
    use gc::{Mutator, Trace, GcArray};
    use gc_derive::Trace;

    type List<T> = GcArray<ListItem<T>>;

    #[derive(Trace)]
    pub enum ListItem<T: Trace> {
        Val(T),
        List(List<T>),
    }

    use gc::Gc;

    #[test]
    fn empty_list() {
        let gc: Gc<List<u8>> = Gc::build(|mutator| GcArray::alloc_with_capacity(mutator, 0).expect("root allocated"));

        gc.collect();

        gc.mutate(|root, _| {
            assert_eq!(root.len(), 0);
            assert_eq!(root.cap(), 0);
            assert!(root.pop().is_none());
        });
    }

    #[test]
    fn collect_list() {
        let gc: Gc<List<u8>> = Gc::build(|mutator| GcArray::alloc(mutator).expect("root allocated"));

        gc.collect();

        gc.mutate(|root, _| {
            assert_eq!(root.len(), 0);
            assert_eq!(root.cap(), 8);
        });
    }

    #[test]
    fn fill_list() {
        let gc: Gc<List<usize>> = Gc::build(|mutator| GcArray::alloc(mutator).expect("root allocated"));

        gc.mutate(|root, mutator| {
            for i in 0..8 {
                let item = mutator.alloc(ListItem::Val(i)).unwrap();
                root.push(mutator, item);
            }

            assert_eq!(root.len(), 8);
            assert_eq!(root.cap(), 8);
        });

        gc.collect();

        gc.mutate(|root, _| {
            for i in 0..8 {
                match *root.at(i) {
                    ListItem::Val(val) => assert!(val == i),
                    ListItem::List(_) => assert!(false),
                }
            }
        });
    }

    #[test]
    fn overfill_list_capacity_and_iter() {
        let gc: Gc<List<usize>> = Gc::build(|mutator| GcArray::alloc(mutator).expect("root allocated"));

        gc.mutate(|root, mutator| {
            for i in 0..100 {
                let item = mutator.alloc(ListItem::Val(i)).unwrap();
                root.push(mutator, item);
            }

            assert_eq!(root.len(), 100);
            // assert_eq!(root.cap(), 128);
        });

        gc.collect();

        gc.mutate(|root, _| {
            for (i, item) in root.iter().enumerate() {
                match *item {
                    ListItem::Val(val) => assert!(val == i),
                    ListItem::List(_) => assert!(false),
                }
            }
        });
    }
}
