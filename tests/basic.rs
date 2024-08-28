use sandpit::{Arena, Gc, Mutator, Root};
use std::mem::{align_of, size_of};
use std::ptr::NonNull;

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
    let arena: Arena<Root![Gc<'_, usize>]> = Arena::new(|mu| Gc::new(mu, 69));

    fn alloc_medium_and_small(arena: &Arena<Root![Gc<'_, usize>]>) {
        arena.mutate(|mu, _| {
            for _ in 0..10_000 {
                Gc::new(mu, 420);
                   
                let data: [u8; 1000] = [0; 1000];

                Gc::new(mu, data);
            }
        });
    }

    alloc_medium_and_small(&arena); // this should leave us with a bunch of free blocks to alloc into
    arena.major_collect();
    alloc_medium_and_small(&arena);
    arena.major_collect(); // now only the root should be left

    arena.mutate(|_, root| assert!(**root == 69));
}


#[test]
fn yield_requested_after_allocating() {
    let arena: Arena<Root![Gc<'_, usize>]> = Arena::new(|mu| Gc::new(mu, 69));

    for _ in 0..5 {
        arena.mutate(|mu, _| loop {
            Gc::new(mu, 420);

            if mu.yield_requested() {
                break;
            }
        });
    }
}

#[test]
fn calling_start_monitor_repeatedly_is_okay() {
    let arena: Arena<Root![Gc<'_, usize>]> = Arena::new(|mu| Gc::new(mu, 69));

    for _ in 0..100 {
        arena.start_monitor();
    }

    arena.mutate(|_, root| assert!(**root == 69));
}

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
    assert_eq!(metrics.max_old_objects,   0);
    assert_eq!(metrics.arena_size,        0);
    assert_eq!(metrics.prev_arena_size,  0);
}

#[test]
fn nested_gc_ptr_root() {
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
fn gc_ptr_size_and_align_equals_nonnull() {
    assert_eq!(size_of::<Gc<()>>(), size_of::<NonNull<()>>());
    assert_eq!(align_of::<Gc<()>>(), align_of::<NonNull<()>>());
}

#[test]
fn mutate_output() {
    let arena: Arena<Root![Gc<'_, usize>]> = Arena::new(|mu| Gc::new(mu, 69));
    let output = arena.mutate(|_mu, root| **root );

    assert!(output == 69)
}
