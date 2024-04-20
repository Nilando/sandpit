use crate::{Gc, GcCell, GcPtr, Mutator};

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
    let gc: Gc<GcPtr<GcPtr<usize>>> = Gc::build(|mutator| {
        mutator.alloc(mutator.alloc(69).unwrap()).unwrap()
    });

    gc.mutate(|root, mutator| {
        let new_val: GcPtr<usize> = mutator.alloc(420).unwrap();
        let val: usize = ***root;
        assert_eq!(val, 69);

        root.write_barrier(mutator, new_val, |root_ref| root_ref);

        let val: usize = ***root;
        assert_eq!(val, 420);
    });
}

#[test]
fn dyn_trace_on_usize() {
    let gc: Gc<GcPtr<usize>> = Gc::build(|mutator| mutator.alloc(69).unwrap());

    gc.mutate(|root, _| {
        assert_eq!(**root, 69);
    });

    gc.collect();
}

#[test]
#[should_panic]
fn deref_null_prt() {
    Gc::build(|_| {
        let ptr: GcPtr<usize> = GcPtr::null();

        assert!(*ptr == 123);
    });
}

