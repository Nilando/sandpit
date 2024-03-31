use crate::{Gc, Mutator, GcPtr, GcCell, GcCellPtr};

struct Node {
    left: GcCellPtr<Node>,
    right: GcCellPtr<Node>,
    val: GcCell<usize>,
}

#[test]
fn create_rooted_arena() {
    let mut gc: Gc<usize> = Gc::build(|scope| {
        scope.alloc(69).unwrap()
    });

    gc.mutate(|root, scope| {
        let root = root.as_ref(scope);

        assert_eq!(*root, 69);
    });
}

#[test]
fn gc_cell_swap() {
    let mut gc: Gc<GcCell<usize>> = Gc::build(|scope| {
        scope.alloc(GcCell::new(69)).unwrap()
    });

    gc.mutate(|root, scope| {
        let root = root.as_ref(scope);
        root.set(scope, 420);
        let val = root.replace(scope, 0);
        assert_eq!(val, 420);
    });
}

#[test]
fn gc_cell_write_barrier() {
    let mut gc: Gc<GcCellPtr<usize>> = Gc::build(|scope| {
        let gc_cell_ptr = GcCellPtr::from(scope.alloc(69).unwrap());

        scope.alloc(gc_cell_ptr).unwrap()
    });

    gc.mutate(|root, scope| {
        let new_val: GcPtr<usize> = scope.alloc(420).unwrap();
        let root_ref: &GcCellPtr<usize> = root.as_ref(scope);
        let val: &usize = root_ref.as_ref(scope).unwrap();

        assert_eq!(*val, 69);

        root.write_barrier(scope, new_val, |root_ref| { root_ref });

        let root_ref: &GcCellPtr<usize> = root.as_ref(scope);
        let val: &usize = root_ref.as_ref(scope).unwrap();

        assert_eq!(*val, 420);
    });
}
