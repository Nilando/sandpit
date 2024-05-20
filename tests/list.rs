use sandpit::{collections::GcArray, Gc, GcPtr, Mutator, Trace};

type List<T> = GcArray<ListItem<T>>;

#[derive(Trace)]
pub enum ListItem<T: Trace> {
    Val(T),
    List(List<T>),
}

#[test]
fn empty_list() {
    let gc: Gc<List<u8>> =
        Gc::build(|mutator| GcArray::alloc_with_capacity(mutator, 0).expect("root allocated"));

    gc.major_collect();

    gc.mutate(|root, _| {
        assert_eq!(root.len(), 0);
        assert_eq!(root.cap(), 0);
        assert!(root.pop().is_none());
    });
}

#[test]
fn collect_list() {
    let gc: Gc<List<u8>> = Gc::build(|mutator| GcArray::alloc(mutator).expect("root allocated"));

    gc.major_collect();

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

    gc.major_collect();

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
fn set_array() {
    let gc: Gc<List<usize>> = Gc::build(|mutator| GcArray::alloc(mutator).expect("root allocated"));

    gc.mutate(|root, mutator| {
        for _ in 0..10_000 {
            let new_list = GcArray::alloc_with_capacity(mutator, 200).unwrap();
            let item = mutator.alloc(ListItem::List(new_list)).unwrap();
            root.push(mutator, item);
        }

        assert_eq!(root.len(), 10000);

        for i in 0..10_000 {
            let item = mutator.alloc(ListItem::Val(i)).unwrap();
            root.set(mutator, i, item);
        }
    });

    gc.major_collect();

    gc.mutate(|root, _| {
        assert_eq!(root.len(), 10000);
        for (i, item) in root.iter().enumerate() {
            match *item {
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
    });

    gc.major_collect();

    gc.mutate(|root, _| {
        for (i, item) in root.iter().enumerate() {
            match *item {
                ListItem::Val(val) => assert!(val == i),
                ListItem::List(_) => assert!(false),
            }
        }
    });
}

#[test]
fn pop() {
    let gc: Gc<List<usize>> = Gc::build(|mutator| GcArray::alloc(mutator).expect("root allocated"));

    gc.mutate(|root, mutator| {
        for i in 0..100 {
            let item = mutator.alloc(ListItem::Val(i)).unwrap();
            root.push(mutator, item);
        }

        assert_eq!(root.len(), 100);

        for i in 0..100 {
            assert_eq!(root.len(), 100 - i);
            root.pop();
        }

        assert_eq!(root.len(), 0);
    });
}

#[test]
fn nested_arrays() {
    let gc: Gc<List<usize>> = Gc::build(|mutator| GcArray::alloc(mutator).expect("root allocated"));

    gc.mutate(|root, mutator| {
        let outer_len = 100;
        for _ in 0..outer_len {
            let new_list = GcArray::alloc(mutator).unwrap();
            let item = mutator.alloc(ListItem::List(new_list)).unwrap();
            root.push(mutator, item);
        }

        assert_eq!(root.len(), outer_len);
    });

    gc.major_collect();

    let num_objs = ((1 + 100) * 3) - 1; // -1 b/c root isnt marked
    assert_eq!(gc.metrics().old_objects_count, num_objs);

    gc.mutate(|root, mutator| {
        let inner_len = 100;
        for item in root.iter() {
            for k in 0..inner_len {
                let n = mutator.alloc(ListItem::Val(k)).unwrap();

                match *item {
                    ListItem::Val(_) => assert!(false),
                    ListItem::List(ref list) => {
                        list.push(mutator, n);
                    }
                }
            }
        }
    });

    gc.major_collect();

    gc.mutate(|root, _| {
        for item in root.iter() {
            match *item {
                ListItem::Val(_) => assert!(false),
                ListItem::List(ref list) => {
                    assert_eq!(list.len(), 100);

                    for (idx, nested_item) in list.iter().enumerate() {
                        match *nested_item {
                            ListItem::Val(val) => assert_eq!(val, idx),
                            ListItem::List(_) => assert!(false),
                        }
                    }
                }
            }
        }
    });
}

#[test]
fn large_array() {
    let gc: Gc<List<usize>> = Gc::build(|mutator| GcArray::alloc(mutator).expect("root allocated"));

    gc.mutate(|root, mutator| {
        for _ in 0..10_000 {
            let new_list = GcArray::alloc(mutator).unwrap();
            let item = mutator.alloc(ListItem::List(new_list)).unwrap();
            root.push(mutator, item);
        }

        assert_eq!(root.len(), 10000);
    });

    gc.major_collect();

    gc.mutate(|root, _| {
        assert_eq!(root.len(), 10000);
    });
}

#[test]
fn get_size() {
    let gc: Gc<List<usize>> = Gc::build(|mutator| GcArray::alloc(mutator).expect("root allocated"));

    gc.mutate(|root, mutator| {
        let large_list = GcArray::alloc_with_capacity(mutator, 40_000).unwrap();
        let large_item = mutator.alloc(ListItem::List(large_list)).unwrap();
        root.push(mutator, large_item);

        let medium_list = GcArray::alloc_with_capacity(mutator, 100).unwrap();
        let medium_item = mutator.alloc(ListItem::List(medium_list)).unwrap();
        root.push(mutator, medium_item);

        let small = mutator.alloc(ListItem::Val(3)).unwrap();
        root.push(mutator, small);
    });

    gc.major_collect();
    //assert_eq!(*gc.metrics().get("arena_size").unwrap(), block_size + large_size);
}

#[test]
#[should_panic]
fn out_of_bounds_at() {
    let gc: Gc<List<usize>> = Gc::build(|mutator| GcArray::alloc(mutator).expect("root allocated"));

    gc.mutate(|root, _| {
        root.at(0);
    });
}

#[test]
#[should_panic]
fn out_of_bounds_set() {
    let gc: Gc<List<usize>> = Gc::build(|mutator| GcArray::alloc(mutator).expect("root allocated"));

    gc.mutate(|root, mutator| {
        root.set(mutator, 0, GcPtr::null());
    });
}

#[test]
fn push_zero_cap() {
    let gc: Gc<List<u8>> =
        Gc::build(|mutator| GcArray::alloc_with_capacity(mutator, 0).expect("root allocated"));

    gc.mutate(|root, mx| {
        let val = mx.alloc(ListItem::Val(9)).unwrap();

        for _ in 0..10 {
            root.push(mx, val.clone());
        }
    });

    gc.major_collect();

    gc.mutate(|root, _mx| {
        assert!(root.len() == 10);
        assert!(root.cap() == 12);
        for i in 0..10 {
            match *root.at(i) {
                ListItem::Val(val) => assert_eq!(val, 9),
                ListItem::List(_) => assert!(false),
            }
        }
    });
}

#[test]
fn cyclic_list() {
    let gc: Gc<List<u8>> = Gc::build(|mutator| GcArray::alloc(mutator).expect("root allocated"));

    gc.mutate(|root, mx| {
        let list = GcArray::alloc(mx).unwrap();
        let list_item = mx.alloc(ListItem::List(list.clone())).unwrap();

        list.push(mx, list_item.clone());
        root.push(mx, list_item);
    });

    gc.major_collect();
}