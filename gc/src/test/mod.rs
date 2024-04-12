use crate::{Gc, GcCell, GcCellPtr, GcPtr, Mutator};

#[test]
fn create_rooted_arena() {
    let gc: Gc<usize> = Gc::build(|mutator| *mutator.alloc(69).unwrap());

    gc.mutate(|root, _| {
        assert_eq!(*root, 69);
    });
}

#[test]
fn gc_cell_swap() {
    let gc: Gc<GcCell<usize>> = Gc::build(|_| GcCell::new(69) );

    gc.mutate(|root, _| {
        root.set(420);
        let val = root.get();
        assert_eq!(val, 420);
    });
}

#[test]
fn gc_cell_write_barrier() {
    let gc: Gc<GcPtr<GcCellPtr<usize>>> = Gc::build(|mutator| {
        let gc_cell_ptr = GcCellPtr::from(mutator.alloc(69).unwrap());

        mutator.alloc(gc_cell_ptr).unwrap()
    });

    gc.mutate(|root, mutator| {
        let new_val: GcPtr<usize> = mutator.alloc(420).unwrap();
        let val: usize = **(root.as_ref().unwrap());
        assert_eq!(val, 69);

        root.write_barrier(mutator, new_val, |root_ref| root_ref);

        let val: usize = **(root.as_ref().unwrap());
        assert_eq!(val, 420);
    });
}
