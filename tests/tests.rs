use rand::prelude::*;
use sandpit::{field, Arena, Gc, GcOpt, GcSync, InnerBarrier, Mutator, Root, Tag, Trace, TraceLeaf};

fn alloc_rand_garbage(mu: &Mutator) {
    let mut rng = rand::thread_rng();
    for _ in 0..rng.gen_range(1..100) {
        for k in 0..rng.gen_range(1..100) {
            Gc::new(mu, k);
        }

        for _ in 0..rng.gen_range(1..10) {
            let array_size = rng.gen_range(0..u16::MAX);
            mu.alloc_array_from_fn(array_size.into(), |i| i);
        }
    }
}

#[test]
fn new_arena() {
    let arena: Arena<Root![Gc<'_, usize>]> = Arena::new(|mu| {
        let root = Gc::new(mu, 69);
        let _foo = Gc::new(mu, 42); // foo will automatically be freed by the GC!

        root
    });

    arena.mutate(|_mu, root| assert_eq!(**root, 69));
}

#[test]
fn arena_allocating_and_collecting() {
    let arena: Arena<Root![Gc<'_, Gc<'_, usize>>]> = Arena::new(|mu| Gc::new(mu, Gc::new(mu, 123)));

    arena.major_collect();

    arena.mutate(|_, root| assert!(***root == 123));
}

/*
 Got rid of the start monitor option for now
#[test]
fn calling_start_monitor_repeatedly_is_okay() {
    let arena: Arena<Root![Gc<'_, usize>]> = Arena::new(|mu| Gc::new(mu, 69));

    for _ in 0..100 {
        arena.start_monitor();
    }

    arena.mutate(|_, root| assert!(**root == 69));
}
*/

#[test]
fn objects_counted_should_be_one() {
    let arena: Arena<Root![Gc<'_, usize>]> = Arena::new(|mu| Gc::new(mu, 69));

    arena.major_collect();

    assert_eq!(arena.metrics().get_old_objects_count(), 1);
}

#[test]
fn counts_collections() {
    let arena: Arena<Root![Gc<'_, usize>]> = Arena::new(|mu| Gc::new(mu, 69));

    for _ in 0..100 {
        arena.major_collect();
        arena.minor_collect();
    }

    let metrics = arena.metrics();

    assert_eq!(metrics.get_major_collections(), 100);
    assert_eq!(metrics.get_minor_collections(), 100);
    assert_eq!(metrics.get_old_objects_count(), 1);
}

#[test]
fn empty_gc_metrics() {
    let arena: Arena<Root![()]> = Arena::new(|_| ());
    let metrics = arena.metrics();

    assert_eq!(metrics.get_major_collections(), 0);
    assert_eq!(metrics.get_minor_collections(), 0);
    assert_eq!(metrics.get_old_objects_count(), 0);
    assert_eq!(metrics.get_max_old_objects(), 0);
    assert_eq!(metrics.get_arena_size(), 0);
    assert_eq!(metrics.get_prev_arena_size(), 0);
}

#[test]
fn nested_root() {
    let arena: Arena<Root![Gc<'_, Gc<'_, Gc<'_, usize>>>]> = Arena::new(|mu| {
        let p1 = Gc::new(mu, 69);
        let p2 = Gc::new(mu, p1);
        Gc::new(mu, p2)
    });

    arena.major_collect();

    let metrics = arena.metrics();

    assert_eq!(metrics.get_old_objects_count(), 3);

    arena.mutate(|_, root| assert_eq!(****root, 69));
}

#[test]
fn trace_gc_null_mut() {
    let arena: Arena<Root![GcOpt<'_, Gc<'_, usize>>]> =
        Arena::new(|mu| GcOpt::new(mu, Gc::new(mu, 69)));

    arena.major_collect();

    assert_eq!(arena.metrics().get_old_objects_count(), 2);

    arena.mutate(|_, root| {
        root.set_none();
    });

    arena.major_collect();

    assert_eq!(arena.metrics().get_old_objects_count(), 0);
}

#[test]
fn old_objects_count_stays_constant() {
    let arena: Arena<Root![Gc<'_, usize>]> = Arena::new(|mu| Gc::new(mu, 69));

    for _ in 0..5 {
        arena.major_collect();
        assert_eq!(arena.metrics().get_old_objects_count(), 1);
    }
}

#[test]
fn write_barrier() {
    #[derive(Trace)]
    struct Foo<'gc> {
        a: GcOpt<'gc, usize>,
        b: GcOpt<'gc, usize>,
        c: GcOpt<'gc, usize>,
    }

    let arena: Arena<Root![Gc<'_, Foo<'_>>]> = Arena::new(|mu| {
        let foo = Foo {
            a: GcOpt::new_none(),
            b: GcOpt::new_none(),
            c: GcOpt::new_none(),
        };

        Gc::new(mu, foo)
    });

    arena.major_collect();

    assert_eq!(arena.metrics().get_old_objects_count(), 1);

    arena.mutate(|mu, root| {
        let new = Gc::new(mu, 420);

        root.write_barrier(mu, |write_barrier| {
            field!(write_barrier, Foo, a).set(new.clone());
            field!(write_barrier, Foo, b).set(new.clone());
            field!(write_barrier, Foo, c).set(new.clone());
        });
    });

    arena.major_collect();

    assert_eq!(arena.metrics().get_old_objects_count(), 2);
}

#[test]
fn yield_is_not_requested() {
    let arena: Arena<Root![Gc<'_, usize>]> = Arena::new(|mu| Gc::new(mu, 3));

    arena.major_collect();

    arena.mutate(|mu, _root| {
        for _ in 0..1000 {
            assert!(!mu.gc_yield());
        }
    });
}

#[test]
fn resets_old_object_count() {
    let arena: Arena<Root![GcOpt<'_, Gc<'_, usize>>]> =
        Arena::new(|mu| GcOpt::new(mu, Gc::new(mu, 3)));

    arena.major_collect();

    assert_eq!(arena.metrics().get_old_objects_count(), 2);

    arena.mutate(|_mu, root| root.set_none());

    arena.major_collect();

    assert_eq!(arena.metrics().get_old_objects_count(), 0);
}

#[test]
fn alloc_option() {
    let arena: Arena<Root![Gc<'_, Gc<'_, Option<Gc<'_, usize>>>>]> =
        Arena::new(|mu| Gc::new(mu, Gc::new(mu, None)));

    arena.major_collect();

    assert_eq!(arena.metrics().get_old_objects_count(), 2);

    arena.mutate(|mu, root| {
        assert!(root.is_none());

        root.write_barrier(mu, |barrier| {
            barrier.set(Gc::new(mu, Some(Gc::new(mu, 69))));
        });
    });

    arena.major_collect();

    assert_eq!(arena.metrics().get_old_objects_count(), 3);
}

#[test]
fn alloc_result() {
    let arena: Arena<Root![Gc<'_, Gc<'_, Result<Gc<'_, usize>, ()>>>]> =
        Arena::new(|mu| Gc::new(mu, Gc::new(mu, Ok(Gc::new(mu, 3)))));

    arena.major_collect();

    assert_eq!(arena.metrics().get_old_objects_count(), 3);

    arena.mutate(|mu, root| {
        let n = root.as_ref().unwrap();
        assert!(**n == 3);

        root.write_barrier(mu, |barrier| {
            barrier.set(Gc::new(mu, Err(())));
        });
    });

    arena.major_collect();

    assert_eq!(arena.metrics().get_old_objects_count(), 2);
}

#[test]
fn alloc_sized_array() {
    let arena: Arena<Root![Gc<'_, [Gc<'_, usize>; 100]>]> = Arena::new(|mu| {
        let arr = core::array::from_fn(|idx| Gc::new(mu, idx));

        Gc::new(mu, arr)
    });

    arena.major_collect();

    assert_eq!(arena.metrics().get_old_objects_count(), 101);

    arena.mutate(|_mu, root| {
        for (idx, gc) in root.iter().enumerate() {
            assert!(**gc == idx);
        }
    });
}

#[test]
fn alloc_tuple() {
    let arena: Arena<Root![(Gc<'_, usize>, Gc<'_, usize>)]> =
        Arena::new(|mu| (Gc::new(mu, 0), Gc::new(mu, 1)));

    arena.major_collect();

    arena.mutate(|_mu, root| {
        assert!(*root.0 == 0);
        assert!(*root.1 == 1);
    });
}

#[test]
fn alloc_array() {
    let arena: Arena<Root![Gc<'_, [usize]>]> = Arena::new(|mu| mu.alloc_array(69, 420));

    arena.major_collect();

    assert_eq!(arena.metrics().get_old_objects_count(), 1);

    arena.mutate(|_mu, root| {
        for x in root.iter() {
            assert!(*x == 69);
        }

        assert!(root.len() == 420);
    });

    arena.major_collect();

    assert_eq!(arena.metrics().get_old_objects_count(), 1);
}

#[test]
fn alloc_array_from_fn() {
    let arena: Arena<Root![Gc<'_, [usize]>]> =
        Arena::new(|mu| mu.alloc_array_from_fn(100, |idx| idx));

    arena.major_collect();

    assert_eq!(arena.metrics().get_old_objects_count(), 1);

    arena.mutate(|_mu, root| {
        for (i, x) in root.iter().enumerate() {
            assert!(*x == i);
        }

        assert!(root.len() == 100);
    });
}

#[test]
fn collect_empty_arena() {
    let arena: Arena<Root![()]> = Arena::new(|_| {});

    arena.major_collect();
}

#[test]
fn collect_singular_void_gc() {
    let arena: Arena<Root![Gc<'_, ()>]> = Arena::new(|mu| Gc::new(mu, ()));

    arena.major_collect();
}

#[test]
fn alloc_array_from_slice() {
    let arena: Arena<Root![Gc<'_, [usize]>]> =
        Arena::new(|mu| mu.alloc_array_from_fn(100, |idx| idx));

    arena.major_collect();

    assert_eq!(arena.metrics().get_old_objects_count(), 1);

    arena.mutate(|mu, root| {
        let root_copy = mu.alloc_array_from_slice(root);

        assert!(*root_copy == **root);
    });
}

#[test]
fn alloc_array_of_static_gc() {
    let arena: Arena<Root![Gc<'_, [Gc<'_, usize>]>]> =
        Arena::new(|mu| mu.alloc_array_from_fn(100, |idx| Gc::new(mu, idx)));

    arena.major_collect();

    assert_eq!(arena.metrics().get_old_objects_count(), 101);

    arena.mutate(|_mu, root| {
        for (idx, gc) in root.iter().enumerate() {
            assert!(idx == **gc);
        }
    });
}

#[test]
fn alloc_array_of_updated_gc() {
    let arena: Arena<Root![Gc<'_, [Gc<'_, usize>]>]> =
        Arena::new(|mu| mu.alloc_array_from_fn(100, |idx| Gc::new(mu, idx)).into());

    arena.major_collect();
    assert_eq!(arena.metrics().get_old_objects_count(), 101);

    arena.mutate(|mu, root| {
        for i in 0..100 {
            let new = Gc::new(mu, i + 100);

            root.write_barrier(mu, |barrier| {
                barrier.at(i).set(new);
            });
        }
    });

    arena.major_collect();
    arena.major_collect();
    assert_eq!(arena.metrics().get_old_objects_count(), 101);

    arena.mutate(|_mu, root| {
        for (idx, gc) in root.iter().enumerate() {
            assert!((idx + 100) == **gc);
        }
    });
}

#[test]
fn two_dimensional_array() {
    let arena: Arena<Root![Gc<'_, [Gc<'_, [Gc<'_, usize>]>]>]> = Arena::new(|mu| {
        mu.alloc_array_from_fn(1000, |i| {
            mu.alloc_array_from_fn(1000, |k| Gc::new(mu, i + k))
        })
    });

    arena.mutate(|_mu, root| {
        for (i, gc) in root.iter().enumerate() {
            for (k, gc) in gc.iter().enumerate() {
                assert!(i + k == **gc);
            }
        }
    });

    arena.major_collect();
}

#[test]
fn change_array_size() {
    let arena: Arena<Root![Gc<'_, Gc<'_, [usize]>>]> =
        Arena::new(|mu| Gc::new(mu, mu.alloc_array_from_fn(100, |i| i).into()));

    arena.mutate(|mu, root| {
        let new = mu.alloc_array_from_fn(10, |_| 69);

        root.write_barrier(mu, |barrier| barrier.set(new));

        assert!(root.len() == 10);
    });
}

#[test]
fn derive_trace_unit_struct() {
    #[derive(Trace)]
    struct Foo;

    let arena: Arena<Root![Gc<'_, Foo>]> = Arena::new(|mu| Gc::new(mu, Foo));

    arena.major_collect();
}

#[test]
fn trace_complex_enum() {
    #[derive(Trace)]
    enum Foo {
        A,
        _B(u8, u8),
        _C { a: u8, b: u8, c: u8 },
    }

    let arena: Arena<Root![Gc<'_, Foo>]> = Arena::new(|mu| Gc::new(mu, Foo::A));

    arena.major_collect();
}

#[test]
fn derive_empty_enums() {
    #[derive(Trace)]
    enum _Foo {}

    #[derive(TraceLeaf)]
    enum _Bar {}
    // can't actually instantiate Foo so this test is just
    // making sure Trace derive works
}

#[test]
fn traceleaf_tuple_struct() {
    use core::cell::Cell;

    #[derive(TraceLeaf, Copy, Clone)]
    struct Foo(u8, u8);

    let arena: Arena<Root![Gc<'_, Cell<Foo>>]> = Arena::new(|mu| Gc::new(mu, Cell::new(Foo(0, 1))));

    arena.major_collect();

    // just to avoid dead code warning
    arena.mutate(|_mu, root| {
        assert!(root.get().0 == 0);
        assert!(root.get().1 == 1);
    });
}

#[test]
fn trace_tuple_struct() {
    #[derive(Trace)]
    struct Foo<'gc>(Gc<'gc, u8>, Gc<'gc, u8>);

    let arena: Arena<Root![Gc<'_, Foo<'_>>]> =
        Arena::new(|mu| Gc::new(mu, Foo(Gc::new(mu, 0), Gc::new(mu, 1))));

    arena.major_collect();

    // just to avoid dead code warning
    arena.mutate(|_mu, root| {
        assert!(*root.0 == 0);
        assert!(*root.1 == 1);
    });
}

/*
#[test]
fn multi_threaded_allocating() {
    use std::sync::Arc;
    let arena: Arc<Arena<Root![usize]>> = Arc::new(Arena::new(|_| 42));
    let arena_copy = Arc::clone(&arena);

    std::thread::spawn(move || {
        arena.mutate(|mu, _| loop {
            Gc::new(mu, 42);

            if mu.gc_yield() {
                break;
            }
        });
    });

    std::thread::spawn(move || {
        arena_copy.mutate(|mu, _| loop {
            Gc::new(mu, 42);

            if mu.gc_yield() {
                break;
            }
        });
    });
}
*/

#[test]
fn cyclic_graph() {
    #[derive(Trace)]
    struct Node<'gc> {
        ptr: GcOpt<'gc, Node<'gc>>,
    }

    impl<'gc> Node<'gc> {
        fn new(_: &'gc Mutator) -> Self {
            Self {
                ptr: GcOpt::new_none(),
            }
        }
    }

    let arena: Arena<Root![Gc<'_, Node<'_>>]> = Arena::new(|mu| Gc::new(mu, Node::new(mu)));

    arena.mutate(|mu, root| {
        let a = Gc::new(mu, Node::new(mu));
        let b = Gc::new(mu, Node::new(mu));
        let c = Gc::new(mu, Node::new(mu));
        let d = Gc::new(mu, Node::new(mu));

        a.write_barrier(mu, |barrier| {
            field!(barrier, Node, ptr).set(b.clone());
        });
        b.write_barrier(mu, |barrier| {
            field!(barrier, Node, ptr).set(c.clone());
        });
        c.write_barrier(mu, |barrier| {
            field!(barrier, Node, ptr).set(d.clone());
        });
        d.write_barrier(mu, |barrier| {
            field!(barrier, Node, ptr).set(a.clone());
        });
        root.write_barrier(mu, |barrier| {
            field!(barrier, Node, ptr).set(a.clone());
        });
    });

    arena.major_collect();
    arena.major_collect();

    assert_eq!(arena.metrics().get_old_objects_count(), 5);
}

// test idea
// for n times
//  1. insert a node into the list
//  2. perform a collection
//  3. allocate random garbage
//
// assert the list still holds all correct values
#[test]
fn alloc_after_collect_test() {
    const LIST_SIZE: usize = 10;
    // increasing list size makes this test run a long time
    #[derive(Trace)]
    struct Node<'gc> {
        ptr: GcOpt<'gc, Node<'gc>>,
        idx: usize,
    }

    let arena: Arena<Root![Gc<'_, Node<'_>>]> = Arena::new(|mu| {
        Gc::new(
            mu,
            Node {
                ptr: GcOpt::new_none(),
                idx: 0,
            },
        )
    });

    for i in (1..LIST_SIZE).rev() {
        arena.mutate(|mu, root| {
            let new_node = Gc::new(
                mu,
                Node {
                    ptr: root.ptr.clone(),
                    idx: i,
                },
            );

            root.write_barrier(mu, |barrier| field!(barrier, Node, ptr).set(new_node));
        });

        arena.major_collect();

        arena.mutate(|mu, _root| {
            alloc_rand_garbage(mu);
        });
    }

    arena.mutate(|_mu, root| {
        let mut node: &Node = root;
        assert!(node.idx == 0);

        for i in 1..LIST_SIZE {
            let next = node.ptr.clone().as_option().unwrap();
            node = next.scoped_deref();
            assert!(node.idx == i);
        }
    });
}

#[test]
fn allocating_triggers_gc_yield() {
    let arena: Arena<Root![Gc<'_, usize>]> = Arena::new(|mu| Gc::new(mu, 69));

    arena.mutate(|mu, _| loop {
        Gc::new(mu, 42);

        if mu.gc_yield() {
            break;
        }
    });
}

#[test]
fn gc_opt_from_gc() {
    let arena: Arena<Root![GcOpt<'_, usize>]> = Arena::new(|mu| Gc::new(mu, 69).into());

    arena.major_collect();

    assert_eq!(arena.metrics().get_old_objects_count(), 1);
}

#[test]
fn gc_scoped_deref() {
    let arena: Arena<Root![Gc<'_, usize>]> = Arena::new(|mu| Gc::new(mu, 69));

    arena.mutate(|mu, root| {
        struct Foo<'gc> {
            inner: &'gc usize,
        }

        impl<'gc> Foo<'gc> {
            fn set_inner(&mut self, gc: Gc<'gc, usize>) {
                // DOES NOT COMPILE
                // self.inner = &gc;
                self.inner = &gc.scoped_deref();
            }
        }

        let mut foo = Foo {
            inner: root.scoped_deref(),
        };

        let gc = Gc::new(mu, 2);

        foo.set_inner(gc);
    });

    arena.major_collect();

    assert_eq!(arena.metrics().get_old_objects_count(), 1);
}

#[test]
fn barrier_as_option() {
    let arena: Arena<Root![Gc<'_, Option<Gc<'_, bool>>>]> =
        Arena::new(|mu| Gc::new(mu, Some(Gc::new(mu, false))));

    arena.mutate(|mu, root| {
        root.write_barrier(mu, |barrier| {
            barrier.into().unwrap().set(Gc::new(mu, true));
        });
    });
}

#[test]
fn gc_clone() {
    let arena: Arena<Root![Gc<'_, bool>]> = Arena::new(|mu| Gc::new(mu, false).clone());

    arena.mutate(|_mu, root| {
        assert!(!**root);
    });
}

#[test]
fn change_array_size_with_inner_barrier() {
    let arena: Arena<Root![InnerBarrier<Gc<'_, [usize]>>]> =
        Arena::new(|mu| InnerBarrier::new(mu, mu.alloc_array_from_fn(100, |i| i).into()));

    arena.mutate(|mu, root| {
        let new = mu.alloc_array_from_fn(10, |_| 69);

        root.write_barrier(mu, |barrier| barrier.set(new));

        assert!(root.inner().len() == 10);
    });
}

use sandpit::GcVec;
use sandpit::Tagged;

#[derive(Tag)]
enum TestTag {
    Raw,
    #[ptr(usize)]
    Ptr,
}

#[test]
fn gc_vec_of_tagged_pointers() {
    let arena: Arena<Root![GcVec<'_, Tagged<'_, TestTag>>]> = Arena::new(|mu| GcVec::new(mu));

    fn push_to_vec<'gc>(mu: &'gc Mutator, vec: &GcVec<'gc, Tagged<'gc, TestTag>>) {
        for i in 0..1000 {
            if i % 2 == 0 {
                let gc_ptr = Gc::new(mu, 123);
                let tag_ptr = TestTag::from_ptr(gc_ptr);

                vec.push(mu, tag_ptr);
            } else {
                let tag_ptr = Tagged::from_raw(1024, TestTag::Raw);

                vec.push(mu, tag_ptr);
            }
        }

        for i in 0..vec.len() {
            let tag_ptr = vec.get_idx(i).unwrap();
            if i % 2 == 0 {
                let gc_ptr = TestTag::get_ptr(tag_ptr).unwrap();

                assert!(*gc_ptr == 123);
            } else {
                let raw = tag_ptr.get_stripped_raw();

                assert_eq!(raw, 1024);
            }
        }
    }

    arena.mutate(|mu, vec| push_to_vec(mu, vec));
    arena.major_collect();
    arena.mutate(|mu, vec| push_to_vec(mu, vec));
    arena.major_collect();
    arena.mutate(|mu, vec| push_to_vec(mu, vec));
    arena.major_collect();
}

#[test]
fn retracing_tagged_ptrs() {
    let arena: Arena<Root![()]> = Arena::new(|_| ());

    fn mutate<'gc>(mu: &'gc Mutator) {
        let gc_ptr = Gc::new(mu, 123);
        let tag_ptr = TestTag::from_ptr(gc_ptr);

        mu.retrace(&*Gc::new(mu, tag_ptr));
    }

    arena.mutate(|mu, ()| mutate(mu));
    arena.major_collect();
}

#[test]
fn gc_str() {
    let arena: Arena<Root![Gc<'_, str>]> = Arena::new(|mu| {
        let root = mu.alloc_str("test");

        root
    });

    arena.major_collect();

    arena.mutate(|_mu, root| {
        let s: &str = root;

        assert_eq!(s, "test")
    });
}

#[test]
fn gc_empty_str() {
    let arena: Arena<Root![Gc<'_, str>]> = Arena::new(|mu| {
        let root = mu.alloc_str("");

        root
    });

    arena.major_collect();

    arena.mutate(|_mu, root| {
        let s: &str = root;

        assert_eq!(s, "")
    });
}

#[test]
fn gc_str_unicode() {
    let arena: Arena<Root![Gc<'_, str>]> = Arena::new(|mu| mu.alloc_str("Hello 世界 🦀"));

    arena.major_collect();

    arena.mutate(|_mu, root| {
        let s: &str = root;
        assert_eq!(s, "Hello 世界 🦀");
        assert_eq!(s.len(), 17); // byte length
        assert_eq!(s.chars().count(), 10); // char count
    });
}

#[test]
fn gc_str_large() {
    let arena: Arena<Root![Gc<'_, str>]> = Arena::new(|mu| {
        let large_string = "x".repeat(10_000);
        mu.alloc_str(&large_string)
    });

    arena.major_collect();

    arena.mutate(|_mu, root| {
        assert_eq!(root.len(), 10_000);
        assert!(root.chars().all(|c| c == 'x'));
    });
}

#[test]
fn gc_str_object_count() {
    let arena: Arena<Root![Gc<'_, str>]> = Arena::new(|mu| mu.alloc_str("test string"));

    arena.major_collect();

    assert_eq!(arena.metrics().get_old_objects_count(), 1);
}

#[test]
fn gc_multiple_strings() {
    let arena: Arena<Root![(Gc<'_, str>, Gc<'_, str>, Gc<'_, str>)]> =
        Arena::new(|mu| (mu.alloc_str("first"), mu.alloc_str("second"), mu.alloc_str("third")));

    arena.major_collect();

    assert_eq!(arena.metrics().get_old_objects_count(), 3);

    arena.mutate(|_mu, root| {
        assert_eq!(&*root.0, "first");
        assert_eq!(&*root.1, "second");
        assert_eq!(&*root.2, "third");
    });
}

#[test]
fn gc_array_of_strings() {
    let arena: Arena<Root![Gc<'_, [Gc<'_, str>]>]> =
        Arena::new(|mu| mu.alloc_array_from_fn(5, |i| mu.alloc_str(&format!("string_{}", i))));

    arena.major_collect();

    assert_eq!(arena.metrics().get_old_objects_count(), 6); // 1 array + 5 strings

    arena.mutate(|_mu, root| {
        for (i, s) in root.iter().enumerate() {
            assert_eq!(&**s, format!("string_{}", i));
        }
    });
}

#[test]
fn gc_str_in_struct() {
    #[derive(Trace)]
    struct Person<'gc> {
        name: Gc<'gc, str>,
        age: usize,
    }

    let arena: Arena<Root![Gc<'_, Person<'_>>]> = Arena::new(|mu| {
        Gc::new(
            mu,
            Person {
                name: mu.alloc_str("Alice"),
                age: 30,
            },
        )
    });

    arena.major_collect();

    assert_eq!(arena.metrics().get_old_objects_count(), 2); // Person + str

    arena.mutate(|_mu, root| {
        assert_eq!(&*root.name, "Alice");
        assert_eq!(root.age, 30);
    });
}

#[test]
fn gc_str_write_barrier() {
    let arena: Arena<Root![Gc<'_, Gc<'_, str>>]> =
        Arena::new(|mu| Gc::new(mu, mu.alloc_str("initial")));

    arena.major_collect();

    assert_eq!(arena.metrics().get_old_objects_count(), 2);

    arena.mutate(|mu, root| {
        let new_str = mu.alloc_str("updated");
        root.write_barrier(mu, |barrier| {
            barrier.set(new_str);
        });

        assert_eq!(&***root, "updated");
    });

    arena.major_collect();

    assert_eq!(arena.metrics().get_old_objects_count(), 2);
}

#[test]
fn gc_opt_str() {
    let arena: Arena<Root![GcOpt<'_, str>]> = Arena::new(|mu| mu.alloc_str("test").into());

    arena.major_collect();

    assert_eq!(arena.metrics().get_old_objects_count(), 1);

    arena.mutate(|_mu, root| {
        assert!(root.as_option().is_some());
        assert_eq!(&*root.as_option().unwrap(), "test");
        root.set_none();
    });

    arena.major_collect();

    assert_eq!(arena.metrics().get_old_objects_count(), 0);
}

#[test]
fn gc_str_survives_collection() {
    let arena: Arena<Root![Gc<'_, str>]> = Arena::new(|mu| mu.alloc_str("persistent"));

    for _ in 0..10 {
        arena.mutate(|mu, _root| {
            alloc_rand_garbage(mu);
        });

        arena.major_collect();
    }

    arena.mutate(|_mu, root| {
        assert_eq!(&**root, "persistent");
    });
}

#[test]
fn gc_str_on_stack() {
    let arena: Arena<Root![Gc<'_, str>]> = Arena::new(|mu| mu.alloc_str("persistent"));


    arena.mutate(|_mu, root| {
        let s: Gc<'_, str> = root.clone();

        println!("{}", &*s);
    });
}

// ===== GcSync Derive Tests =====

#[test]
fn derive_gcsync_basic() {
    #[derive(Trace, Clone, GcSync)]
    struct Point<'gc> {
        x: Gc<'gc, i32>,
        y: Gc<'gc, i32>,
    }

    let arena: Arena<Root![GcVec<'_, Point<'_>>]> = Arena::new(|mu| {
        let vec = GcVec::new(mu);
        vec.push(
            mu,
            Point {
                x: Gc::new(mu, 10),
                y: Gc::new(mu, 20),
            },
        );
        vec
    });

    arena.mutate(|_mu, vec| {
        let p = vec.get_idx(0).unwrap();
        assert_eq!(*p.x, 10);
        assert_eq!(*p.y, 20);
    });
}

#[test]
fn derive_gcsync_with_set() {
    #[derive(Trace, Clone, GcSync)]
    struct Point<'gc> {
        x: Gc<'gc, i32>,
        y: Gc<'gc, i32>,
    }

    let arena: Arena<Root![GcVec<'_, Point<'_>>]> = Arena::new(|mu| {
        let vec = GcVec::new(mu);
        vec.push(
            mu,
            Point {
                x: Gc::new(mu, 10),
                y: Gc::new(mu, 20),
            },
        );
        vec
    });

    arena.mutate(|mu, vec| {
        vec.set(
            mu,
            Point {
                x: Gc::new(mu, 30),
                y: Gc::new(mu, 40),
            },
            0,
        );
    });

    arena.major_collect();

    arena.mutate(|_mu, vec| {
        let p = vec.get_idx(0).unwrap();
        assert_eq!(*p.x, 30);
        assert_eq!(*p.y, 40);
    });
}

#[test]
fn derive_gcsync_generic() {
    #[derive(Trace, Clone, GcSync)]
    struct Wrapper<'gc, T: GcSync<'gc>> {
        value: T,
        marker: Gc<'gc, bool>,
    }

    let arena: Arena<Root![GcVec<'_, Wrapper<'_, Gc<'_, usize>>>]> = Arena::new(|mu| {
        let vec = GcVec::new(mu);
        vec.push(
            mu,
            Wrapper {
                value: Gc::new(mu, 42),
                marker: Gc::new(mu, true),
            },
        );
        vec
    });

    arena.major_collect();

    arena.mutate(|_mu, vec| {
        let w = vec.get_idx(0).unwrap();
        assert_eq!(*w.value, 42);
        assert_eq!(*w.marker, true);
    });
}

#[test]
fn derive_gcsync_mixed_fields() {
    use core::cell::Cell;

    #[derive(Trace, Clone, GcSync)]
    struct Mixed<'gc> {
        gc_ptr: Gc<'gc, str>,
        opt_ptr: GcOpt<'gc, bool>,
        leaf_data: Cell<usize>,
    }

    let arena: Arena<Root![GcVec<'_, Mixed<'_>>]> = Arena::new(|mu| {
        let vec = GcVec::new(mu);
        vec.push(
            mu,
            Mixed {
                gc_ptr: mu.alloc_str("test"),
                opt_ptr: GcOpt::new(mu, true),
                leaf_data: Cell::new(100),
            },
        );
        vec
    });

    arena.major_collect();

    arena.mutate(|_mu, vec| {
        let m = vec.get_idx(0).unwrap();
        assert_eq!(&*m.gc_ptr, "test");
        assert_eq!(*m.opt_ptr.as_option().unwrap(), true);
        assert_eq!(m.leaf_data.get(), 100);
    });
}

#[test]
fn derive_gcsync_tuple_struct() {
    #[derive(Trace, Clone, GcSync)]
    struct Tuple<'gc>(Gc<'gc, i32>, Gc<'gc, bool>);

    let arena: Arena<Root![GcVec<'_, Tuple<'_>>]> = Arena::new(|mu| {
        let vec = GcVec::new(mu);
        vec.push(mu, Tuple(Gc::new(mu, 123), Gc::new(mu, false)));
        vec
    });

    arena.mutate(|_mu, vec| {
        let t = vec.get_idx(0).unwrap();
        assert_eq!(*t.0, 123);
        assert_eq!(*t.1, false);
    });
}

#[test]
fn derive_gcsync_nested_generic() {
    #[derive(Trace, Clone, GcSync)]
    struct Inner<'gc> {
        value: Gc<'gc, usize>,
    }

    #[derive(Trace, Clone, GcSync)]
    struct Outer<'gc, T: GcSync<'gc>> {
        inner: T,
        extra: Gc<'gc, bool>,
    }

    let arena: Arena<Root![GcVec<'_, Outer<'_, Inner<'_>>>]> = Arena::new(|mu| {
        let vec = GcVec::new(mu);
        vec.push(
            mu,
            Outer {
                inner: Inner {
                    value: Gc::new(mu, 999),
                },
                extra: Gc::new(mu, true),
            },
        );
        vec
    });

    arena.major_collect();

    arena.mutate(|_mu, vec| {
        let o = vec.get_idx(0).unwrap();
        assert_eq!(*o.inner.value, 999);
        assert_eq!(*o.extra, true);
    });
}

#[test]
fn gcvec_push_pop_with_derived_type() {
    #[derive(Trace, Clone, GcSync)]
    struct Point<'gc> {
        x: Gc<'gc, i32>,
        y: Gc<'gc, i32>,
    }

    let arena: Arena<Root![GcVec<'_, Point<'_>>]> = Arena::new(|mu| {
        let vec = GcVec::new(mu);
        for i in 0..100 {
            vec.push(
                mu,
                Point {
                    x: Gc::new(mu, i),
                    y: Gc::new(mu, i * 2),
                },
            );
        }
        vec
    });

    arena.major_collect();

    arena.mutate(|_mu, vec| {
        for i in (0..100).rev() {
            let point = vec.pop().unwrap();
            assert_eq!(*point.x, i);
            assert_eq!(*point.y, i * 2);
        }
        assert_eq!(vec.len(), 0);
    });
}

#[test]
fn gcvec_set_with_gc_collection() {
    #[derive(Trace, Clone, GcSync)]
    struct Data<'gc> {
        value: Gc<'gc, usize>,
        flag: GcOpt<'gc, bool>,
    }

    let arena: Arena<Root![GcVec<'_, Data<'_>>]> = Arena::new(|mu| {
        let vec = GcVec::new(mu);
        for i in 0..50 {
            vec.push(
                mu,
                Data {
                    value: Gc::new(mu, i),
                    flag: GcOpt::new(mu, i % 2 == 0),
                },
            );
        }
        vec
    });

    arena.major_collect();

    arena.mutate(|mu, vec| {
        for i in 0..50 {
            vec.set(
                mu,
                Data {
                    value: Gc::new(mu, i + 1000),
                    flag: GcOpt::new_none(),
                },
                i,
            );
        }
    });

    arena.major_collect();
    arena.major_collect();

    arena.mutate(|_mu, vec| {
        for i in 0..50 {
            let d = vec.get_idx(i).unwrap();
            assert_eq!(*d.value, i + 1000);
            assert!(d.flag.is_none());
        }
    });
}

#[test]
fn gcvec_derived_type_with_random_garbage() {
    #[derive(Trace, Clone, GcSync)]
    struct Container<'gc> {
        data: Gc<'gc, usize>,
    }

    let arena: Arena<Root![GcVec<'_, Container<'_>>]> = Arena::new(|mu| {
        let vec = GcVec::new(mu);
        for i in 0..10 {
            vec.push(mu, Container { data: Gc::new(mu, i) });
            alloc_rand_garbage(mu);
        }
        vec
    });

    for _ in 0..5 {
        arena.major_collect();
        arena.mutate(|mu, _vec| {
            alloc_rand_garbage(mu);
        });
    }

    arena.mutate(|_mu, vec| {
        for i in 0..10 {
            let c = vec.get_idx(i).unwrap();
            assert_eq!(*c.data, i);
        }
    });
}
