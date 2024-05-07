use crate::{Gc, GcCell, GcPtr, Mutator};
use std::alloc::Layout;

#[test]
fn create_rooted_arena() {
    let gc: Gc<usize> = Gc::build(|mutator| *mutator.alloc(69).unwrap());

    gc.mutate(|root, _| {
        assert_eq!(*root, 69);
    });
}

#[test]
fn gc_cell_swap() {
    let gc: Gc<GcCell<usize>> = Gc::build(|_| GcCell::new(69));

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

        root.write_barrier(mutator, new_val, |root_ref| root_ref);

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

    gc.mutate(|_, m| {
        let medium_layout = unsafe { Layout::from_size_align_unchecked(200, 8) };
        for _ in 0..10_000 {
            m.alloc(420).unwrap();
            m.alloc_layout(medium_layout).unwrap();
        }
    });

    gc.major_collect();

    gc.mutate(|_, m| {
        let medium_layout = unsafe { Layout::from_size_align_unchecked(200, 8) };
        for _ in 0..10_000 {
            m.alloc(420).unwrap();
            m.alloc_layout(medium_layout).unwrap();
        }
    });
    gc.major_collect();

    gc.mutate(|root, _| {
        assert!(**root == 69);
    });
}

#[test]
fn wait_for_trace() {
    let gc: Gc<GcPtr<usize>> = Gc::build(|mutator| mutator.alloc(69).unwrap());

    gc.start_monitor();

    for _ in 0..10 {
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

    gc.start_monitor();
    gc.start_monitor();
    gc.start_monitor();
    gc.start_monitor();
    gc.start_monitor();
    gc.start_monitor();

    gc.mutate(|_, m| loop {
        m.alloc(420).unwrap();

        if m.yield_requested() {
            break;
        }
    });
}
