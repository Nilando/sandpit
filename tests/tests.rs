use sandpit::{
    gc::{Gc, GcMut, GcNullMut},
    field, 
    Mutator,
    Arena, 
    Root,
    Trace,
    TraceLeaf
};
use rand::prelude::*;

fn alloc_rand_garbage(mu: &Mutator) {
    let mut rng = rand::thread_rng();
    for _ in 0..rng.gen_range(1..100) {
        for k in 0..rng.gen_range(1..100) {
            Gc::new(mu, k);
        }

        for _ in 0..rng.gen_range(1..10) {
            let array_size = rng.gen_range(0..u16::MAX);
            mu.alloc_array(0u8, array_size as usize);
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

    arena.mutate(|_mu, root| {
        assert_eq!(**root, 69);
    });
}

#[test]
fn arena_allocating_and_collecting() {
    let arena: Arena<Root![Gc<'_, Gc<'_, usize>>]> = Arena::new(|mu| {
        Gc::new(mu, Gc::new(mu, 123))
    });

    arena.major_collect();

    arena.mutate(|_, root| assert!(***root == 123));
}

// TODO find a way to write this test so that it doesn't use a crazy amount of memory
#[ignore]
#[test]
fn yield_requested_after_allocating() {
    let arena: Arena<Root![Gc<'_, usize>]> = Arena::new(|mu| Gc::new(mu, 69));

    arena.mutate(|mu, _| loop {
        Gc::new(mu, 42);

        if mu.gc_yield() { break; }
    });
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

    assert_eq!(arena.metrics().old_objects_count, 1);
}

#[test]
fn counts_collections() {
    let arena: Arena<Root![Gc<'_, usize>]> = Arena::new(|mu| Gc::new(mu, 69));

    for _ in 0..100 {
        arena.major_collect();
        arena.minor_collect();
    }

    let metrics = arena.metrics();

    assert_eq!(metrics.major_collections, 100);
    assert_eq!(metrics.minor_collections, 100);
    assert_eq!(metrics.old_objects_count, 1);
}

#[test]
fn empty_gc_metrics() {
    let arena: Arena<Root![()]> = Arena::new(|_| ());
    let metrics = arena.metrics();

    assert_eq!(metrics.major_collections, 0);
    assert_eq!(metrics.minor_collections, 0);
    assert_eq!(metrics.old_objects_count, 0);
    assert_eq!(metrics.max_old_objects, 0);
    assert_eq!(metrics.arena_size, 0);
    assert_eq!(metrics.prev_arena_size, 0);
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

    assert_eq!(metrics.old_objects_count, 3);

    arena.mutate(|_, root| assert_eq!(****root, 69));
}

#[test]
fn trace_gc_null_mut() {
    let arena: Arena<Root![GcNullMut<'_, Gc<'_, usize>>]> = Arena::new(|mu| {
        GcNullMut::new(mu, Gc::new(mu, 69))
    });

    arena.major_collect();

    assert_eq!(arena.metrics().old_objects_count, 2);

    arena.mutate(|_, root| {
        root.set_null();
    });

    arena.major_collect();

    assert_eq!(arena.metrics().old_objects_count, 0);
}

#[test]
fn old_objects_count_stays_constant() {
    let arena: Arena<Root![Gc<'_, usize>]> = Arena::new(|mu| Gc::new(mu, 69));

    for _ in 0..5 {
        arena.major_collect();
        assert_eq!(arena.metrics().old_objects_count, 1);
    }
}

#[test]
fn write_barrier() {
    #[derive(Trace)]
    struct Foo<'gc> {
        a: GcNullMut<'gc, usize>,
        b: GcNullMut<'gc, usize>,
        c: GcNullMut<'gc, usize>,
    }

    let arena: Arena<Root![Gc<'_, Foo<'_>>]> = Arena::new(|mu| {
        let foo = Foo {
            a: GcNullMut::new_null(mu),
            b: GcNullMut::new_null(mu),
            c: GcNullMut::new_null(mu),
        };

        Gc::new(mu, foo)
    });

    arena.major_collect();

    assert_eq!(arena.metrics().old_objects_count, 1);

    arena.mutate(|mu, root| {
        let new = Gc::new(mu, 420);

        root.write_barrier(mu, |write_barrier| {
            field!(write_barrier, Foo, a).set(new.into());
            field!(write_barrier, Foo, b).set(new.into());
            field!(write_barrier, Foo, c).set(new.into());
        });
    });

    arena.major_collect();

    assert_eq!(arena.metrics().old_objects_count, 2);
}

#[test]
fn yield_is_not_requested() {
    let arena: Arena<Root![Gc<'_, usize>]> = Arena::new(|mu| Gc::new(mu, 3));

    arena.major_collect();

    arena.mutate(|mu, _root| {
        for _ in 0..1000 {
            assert!(mu.gc_yield() == false);
        }
    });
}

#[test]
fn resets_old_object_count() {
    let arena: Arena<Root![GcNullMut<'_, Gc<'_, usize>>]> = Arena::new(|mu| GcNullMut::new(mu, Gc::new(mu, 3)));

    arena.major_collect();

    assert_eq!(arena.metrics().old_objects_count, 2);

    arena.mutate(|_mu, root| root.set_null() );

    arena.major_collect();

    assert_eq!(arena.metrics().old_objects_count, 0);
}

#[test]
fn alloc_array() {
    let arena: Arena<Root![Gc<'_, [usize]>]> = Arena::new(|mu| {
        mu.alloc_array(69, 420)
    });

    arena.major_collect();

    assert_eq!(arena.metrics().old_objects_count, 1);

    arena.mutate(|_mu, root| {
        for x in root.iter() {
            assert!(*x == 69);
        }

        assert!(root.len() == 420);
    });

    arena.major_collect();

    assert_eq!(arena.metrics().old_objects_count, 1);
}

#[test]
fn alloc_array_from_fn() {
    let arena: Arena<Root![Gc<'_, [usize]>]> = Arena::new(|mu| {
        mu.alloc_array_from_fn(100, |idx| {
            idx
        })
    });

    arena.major_collect();

    assert_eq!(arena.metrics().old_objects_count, 1);

    arena.mutate(|_mu, root| {
        for (i, x) in root.iter().enumerate() {
            assert!(*x == i);
        }

        assert!(root.len() == 100);
    });
}

#[test]
fn alloc_array_from_slice() {
    let arena: Arena<Root![Gc<'_, [usize]>]> = Arena::new(|mu| {
        mu.alloc_array_from_fn(100, |idx| {
            idx
        })
    });

    arena.major_collect();

    assert_eq!(arena.metrics().old_objects_count, 1);

    arena.mutate(|mu, root| {
        let root_copy = mu.alloc_array_from_slice(root);

        assert!(*root_copy == **root);
    });
}

#[test]
fn alloc_array_of_gc() {
    let arena: Arena<Root![Gc<'_, [Gc<'_, usize>]>]> = Arena::new(|mu| {
        mu.alloc_array_from_fn(100, |idx| {
            Gc::new(mu, idx)
        })
    });

    arena.major_collect();

    assert_eq!(arena.metrics().old_objects_count, 101);

    arena.mutate(|_mu, root| {
        for (idx, gc) in root.iter().enumerate() {
            assert!(idx == **gc);
        }
    });
}

#[test]
fn alloc_array_of_gc_mut() {
    let arena: Arena<Root![Gc<'_, [GcMut<'_, usize>]>]> = Arena::new(|mu| {
        mu.alloc_array_from_fn(100, |idx| {
            GcMut::new(mu, idx)
        })
    });

    arena.major_collect();
    assert_eq!(arena.metrics().old_objects_count, 101);

    arena.mutate(|mu, root| {
        for i in 0..100 {
            let new = GcMut::new(mu, i + 100);

            root.write_barrier(mu, |barrier| {
                barrier.at(i).set(new);
            });
        }
    });

    arena.major_collect();
    arena.major_collect();
    assert_eq!(arena.metrics().old_objects_count, 101);

    arena.mutate(|_mu, root| {
        for (idx, gc) in root.iter().enumerate() {
            assert!((idx + 100) == **gc);
        }
    });
}

#[test]
fn two_dimensional_array() {
    let arena: Arena<Root![Gc<'_, [Gc<'_, [Gc<'_, usize>]>]>]> = Arena::new(|mu| {
        mu.alloc_array_from_fn(100, |i| {
            mu.alloc_array_from_fn(100, |k| {
                Gc::new(mu, i + k)
            })
        })
    });

    arena.mutate(|_mu, root| {
        for (i, gc) in root.iter().enumerate() {
            for (k, gc) in gc.iter().enumerate() {
                assert!(i + k == **gc);
            }
        }
    });
}

#[test]
fn change_array_size() {
    let arena: Arena<Root![Gc<'_, GcMut<'_, [usize]>>]> = Arena::new(|mu| {
        Gc::new(mu, mu.alloc_array_from_fn(100, |i| i).into())
    });

    arena.mutate(|mu, root| {
        let new = mu.alloc_array_from_fn(10, |_| 69);

        root.write_barrier(mu, |barrier| {
            barrier.set(new.into())
        });

        assert!(root.len() == 10);
    });
}

#[test]
fn derive_trace_unit_struct() {
    #[derive(Trace)]
    struct Foo;

    let arena: Arena<Root![Gc<'_, Foo>]> = Arena::new(|mu| {
        Gc::new(mu, Foo)
    });

    arena.major_collect();
}

#[test]
fn trace_complex_enum() {
    #[derive(Trace)]
    enum Foo {
        A,
        B(u8, u8),
        C {
            a: u8,
            b: u8,
            c: u8
        }
    }

    let arena: Arena<Root![Gc<'_, Foo>]> = Arena::new(|mu| {
        Gc::new(mu, Foo::A)
    });

    arena.major_collect();
}

#[test]
fn derive_empty_enums() {
    #[derive(Trace)]
    enum Foo {}

    #[derive(TraceLeaf)]
    enum Bar {}
    // can't actually instantiate Foo so this test is just
    // making sure Trace derive works
}

#[test]
fn traceleaf_tuple_struct() {
    use std::cell::Cell;

    #[derive(TraceLeaf)]
    struct Foo(u8, u8);

    let arena: Arena<Root![Gc<'_, Cell<Foo>>]> = Arena::new(|mu| {
        Gc::new(mu, Cell::new(Foo(0, 1)))
    });

    arena.major_collect();
}

#[test]
fn trace_tuple_struct() {
    #[derive(Trace)]
    struct Foo(u8, u8);

    let arena: Arena<Root![Gc<'_, Foo>]> = Arena::new(|mu| {
        Gc::new(mu, Foo(0, 1))
    });

    arena.major_collect();
}

#[test]
fn multi_threaded_allocating() {
    use std::sync::Arc;
    let arena: Arc<Arena<Root![usize]>> = Arc::new(Arena::new(|_| 42));
    let arena_copy = Arc::clone(&arena);

    std::thread::spawn(move || {
        arena.mutate(|mu, _| loop {
            Gc::new(mu, 42);

            if mu.gc_yield() { break; }
        });
    });

    std::thread::spawn(move || {
        arena_copy.mutate(|mu, _| loop {
            Gc::new(mu, 42);

            if mu.gc_yield() { break; }
        });
    });
}


// test idea
// for n times
//  1. insert a node into the list
//  2. perform a collection
//  3. allocate random garbage
//
// assert the list still holds all correct values
#[test]
fn list_building_test() {
    const LIST_SIZE: usize = 10;
    // increasing list size makes this test run a  long time
    #[derive(Trace)]
    struct Node<'gc> {
        ptr: GcNullMut<'gc, Node<'gc>>,
        idx: usize
    }

    let arena: Arena<Root![Gc<'_, Node<'_>>]> = Arena::new(|mu| {
        Gc::new(mu, Node {
            ptr: GcNullMut::new_null(mu),
            idx: 0,
        })
    });

    for i in (1..LIST_SIZE).rev() {
        arena.mutate(|mu, root| {
            println!("pushing node: {i}");

            let new_node = GcNullMut::new(mu, Node {
                ptr: root.ptr.clone(),
                idx: i,
            });

            root.write_barrier(mu, |barrier| {
                field!(barrier, Node, ptr).set(new_node)
            });
        });

        arena.major_collect();

        arena.mutate(|mu, _root| {
            alloc_rand_garbage(mu);
        });
    }

    arena.mutate(|_mu, root| {
        let mut node: &Node = &**root;
        assert!(node.idx == 0);

        for i in 1..LIST_SIZE {
            let next = node.ptr.clone().as_option().unwrap();
            node = next.scoped_deref();
            assert!(node.idx == i);
        }
    });
}
