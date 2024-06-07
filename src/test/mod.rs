use crate::{Gc, GcPtr, Mutator, Trace};
use std::alloc::Layout;
use std::cell::Cell;
use std::mem::{align_of, size_of};

#[test]
fn create_rooted_arena() {
    let gc: Gc<usize> = Gc::build(|mutator| *mutator.alloc(69).unwrap());

    gc.mutate(|root, _| {
        assert_eq!(*root, 69);
    });
}

#[test]
fn cell_root() {
    let gc: Gc<Cell<usize>> = Gc::build(|_| Cell::new(69));

    gc.mutate(|root, _| {
        root.set(420);
        let val = root.get();
        assert_eq!(val, 420);
    });
}

#[test]
fn gc_cell_write_barrier() {
    let gc: Gc<GcPtr<GcPtr<usize>>> =
        Gc::build(|mutator| mutator.alloc(mutator.alloc(69).unwrap()).unwrap());

    gc.mutate(|root, mutator| {
        let new_val: GcPtr<usize> = mutator.alloc(420).unwrap();
        let val: usize = ***root;
        assert_eq!(val, 69);

        mutator.write_barrier(root.clone(), new_val, |root_ref| root_ref);

        let val: usize = ***root;
        assert_eq!(val, 420);
    });
}

// this is just testing that dyn trace doesn't get called on
// a TraceLeaf like usize
#[test]
fn dyn_trace_on_usize() {
    let gc: Gc<GcPtr<usize>> = Gc::build(|mutator| mutator.alloc(69).unwrap());

    gc.mutate(|root, _| {
        assert_eq!(**root, 69);
    });

    gc.major_collect();
}

#[test]
#[should_panic]
fn deref_null_prt() {
    Gc::build(|_| {
        let ptr: GcPtr<usize> = GcPtr::null();

        assert!(*ptr == 123);
    });
}

#[test]
fn alloc_into_free_blocks() {
    let gc: Gc<GcPtr<usize>> = Gc::build(|mutator| mutator.alloc(69).unwrap());

    fn alloc_medium_and_small(gc: &Gc<GcPtr<usize>>) {
        gc.mutate(|_, m| {
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

    gc.mutate(|root, _| assert!(**root == 69));
}

#[test]
fn wait_for_trace() {
    let gc: Gc<GcPtr<usize>> = Gc::build(|mutator| mutator.alloc(69).unwrap());

    for _ in 0..5 {
        gc.mutate(|_, m| loop {
            m.alloc(420).unwrap();

            if m.yield_requested() {
                break;
            }
        });
    }
}

#[test]
fn start_monitor_multiple_times() {
    let gc: Gc<GcPtr<usize>> = Gc::build(|mutator| mutator.alloc(69).unwrap());

    for _ in 0..10 {
        gc.start_monitor();
    }

    gc.mutate(|_, m| loop {
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
    let gc: Gc<GcPtr<usize>> = Gc::build(|mutator| mutator.alloc(69).unwrap());

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
    let gc = Gc::build(|_| ());

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
    let gc = Gc::build(|mutator| {
        let p1 = mutator.alloc(69).unwrap();
        let p2 = mutator.alloc(p1).unwrap();
        let p3 = mutator.alloc(p2).unwrap();
        let p4 = mutator.alloc(p3).unwrap();
        let p5 = mutator.alloc(p4).unwrap();
        p5
    });

    gc.major_collect();

    let metrics = gc.metrics();

    assert_eq!(metrics.old_objects_count, 5);

    gc.mutate(|root, _| assert_eq!(******root, 69));
}

#[test]
fn gc_ptr_size_and_align() {
    assert_eq!(size_of::<GcPtr<()>>(), size_of::<GcPtr<u128>>());
    assert_eq!(align_of::<GcPtr<()>>(), align_of::<GcPtr<u128>>());
}

#[test]
fn extract_and_insert() {
    let gc = Gc::build(|_| Cell::new(0));
    let val = gc.extract(|root| root.get());

    assert!(val == 0);

    gc.insert(69, |root, new_val| {
        root.set(new_val);
    });

    let val = gc.extract(|root| root.get());

    assert!(val == 69);
}
