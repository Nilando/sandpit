use crate::{Arena, Gc, Mutator, Root};
use higher_kinded_types::ForLt;
use std::mem::{align_of, size_of};

#[test]
fn new_arena() {
    let arena: Arena<Root![Gc<'_, usize>]> = Arena::new(|mu| Gc::new(mu, 69));

    arena.mutate(|_mu, root| {
        assert_eq!(**root, 69);
    });
}

/*
#[test]
fn cell_root() {
    let gc: GcArena<Cell<usize>> = GcArena::build((), |_, _| Cell::new(69));

    gc.mutate((), |root, _, _| {
        root.set(420);
        let val = root.get();
        assert_eq!(val, 420);
    });
}

#[test]
fn gc_ptr_swap() {
    let gc: GcArena<Gc<Gc<usize>>> =
        GcArena::build((), |mu, _| mu.alloc(mu.alloc(69).unwrap()).unwrap());

    gc.mutate((), |root, mu, _| {
        let new_val: Gc<usize> = mu.alloc(420).unwrap();
        let val: usize = ***root;
        assert_eq!(val, 69);

        unsafe { (**root).swap(new_val); }

        let val: usize = ***root;
        assert_eq!(val, 420);
    });
}

// this is just testing that dyn trace doesn't get called on
// a TraceLeaf like usize
#[test]
fn dyn_trace_on_usize() {
    let gc: GcArena<Gc<usize>> = GcArena::build((), |mu, _| mu.alloc(69).unwrap());

    gc.mutate((), |root, _, _| {
        assert_eq!(**root, 69);
    });

    gc.major_collect();
}

#[test]
#[should_panic]
fn deref_null_prt() {
    GcArena::build((), |_, _| {
        let ptr: Gc<usize> = Gc::null();

        assert!(*ptr == 123);
    });
}

#[test]
fn alloc_into_free_blocks() {
    let gc: GcArena<Gc<usize>> = GcArena::build((), |mu, _| mu.alloc(69).unwrap());

    fn alloc_medium_and_small(gc: &GcArena<Gc<usize>>) {
        gc.mutate((), |_, m, _| {
            for _ in 0..10_000 {
                m.alloc(420).unwrap();
                let data: [u8; 1000] = [0; 1000];

                m.alloc(data).unwrap();
            }
        });
    }

    alloc_medium_and_small(&gc); // this should leave us with a bunch of free blocks to alloc into
    gc.major_collect();
    alloc_medium_and_small(&gc);
    gc.major_collect(); // now only the root should be left

    gc.mutate((), |root, _, _| assert!(**root == 69));
}

#[test]
fn wait_for_trace() {
    let gc: GcArena<Gc<usize>> = GcArena::build((), |mu, _| mu.alloc(69).unwrap());

    for _ in 0..5 {
        gc.mutate((), |_, m, _| loop {
            m.alloc(420).unwrap();

            if m.yield_requested() {
                break;
            }
        });
    }
}

#[test]
fn start_monitor_multiple_times() {
    let gc: GcArena<Gc<usize>> = GcArena::build((), |mu, _| mu.alloc(69).unwrap());

    for _ in 0..10 {
        gc.start_monitor();
    }

    gc.mutate((), |_, m, _| loop {
        m.alloc(420).unwrap();

        if m.yield_requested() {
            break;
        }
    });

    gc.major_collect();
    assert_eq!(gc.metrics().old_objects_count, 1);
}

#[test]
fn counts_collections() {
    let gc: GcArena<Gc<usize>> = GcArena::build((), |mu, _| mu.alloc(69).unwrap());

    for _ in 0..100 {
        gc.major_collect();
        gc.minor_collect();
    }

    let metrics = gc.metrics();

    assert_eq!(metrics.major_collections, 100);
    assert_eq!(metrics.minor_collections, 100);
    assert_eq!(metrics.old_objects_count, 1);
}

#[test]
fn empty_gc_metrics() {
    let gc = GcArena::build((), |_, _| ());

    gc.major_collect();

    let metrics = gc.metrics();

    assert_eq!(metrics.major_collections, 1);
    assert_eq!(metrics.minor_collections, 0);
    assert_eq!(metrics.old_objects_count, 0);
    assert_eq!(metrics.max_old_objects, 0);
    assert_eq!(metrics.arena_size, 0);
    assert_eq!(metrics.prev_arena_size, 0);
}

#[test]
fn nested_gc_ptr_root() {
    let gc = GcArena::build((), |mu, _| {
        let p1 = mu.alloc(69).unwrap();
        let p2 = mu.alloc(p1).unwrap();
        let p3 = mu.alloc(p2).unwrap();
        let p4 = mu.alloc(p3).unwrap();
        let p5 = mu.alloc(p4).unwrap();
        p5
    });

    gc.major_collect();

    let metrics = gc.metrics();

    assert_eq!(metrics.old_objects_count, 5);

    gc.mutate((), |root, _, _| assert_eq!(******root, 69));
}

#[test]
fn gc_ptr_size_and_align() {
    assert_eq!(size_of::<Gc<()>>(), size_of::<Gc<u128>>());
    assert_eq!(align_of::<Gc<()>>(), align_of::<Gc<u128>>());
}

#[derive(Trace)]
struct MyDroppable;

impl Drop for MyDroppable {
    fn drop(&mut self) { 
        println!("DROPPING");
    }
}
#[test]
fn disable_alloc_drop_types() {

    let arena = GcArena::build(|_| ());

    arena.mutate(|mu, _| {
        let drop_obj = MyDroppable;

        mu.alloc(drop_obj).unwrap();
    });
}
*/
