use super::arena::Arena;
use super::constants::{BLOCK_CAPACITY, BLOCK_SIZE};
use super::size_class::SizeClass;
use super::Allocator;
use crate::allocate::{Allocate, GenerationalArena};

#[test]
fn hello_alloc() {
    let arena = Arena::new();
    let allocator = Allocator::new(&arena);
    let name = "Hello Alloc";

    assert_eq!(arena.get_size(), 0);

    let ptr: std::ptr::NonNull<&str> = allocator.alloc(name).unwrap();

    unsafe {
        let name_ref = ptr.as_ref();

        assert_eq!(name_ref, &name);
    }

    assert_eq!(arena.get_size(), BLOCK_SIZE);
}

/*
#[test]
fn alloc_large() {
    let arena = Arena::new();
    let allocator = Allocator::new(&arena);
    let data: [usize; BLOCK_SIZE] = [0; BLOCK_SIZE];

    assert_eq!(arena.get_size(), 0);

    allocator.alloc(data).unwrap();

    assert_eq!(
        arena.get_size(),
        (BLOCK_SIZE * aligned_size::<usize>()) + aligned_size::<Header>()
    );
}
*/

#[test]
fn alloc_many_single_bytes() {
    let arena = Arena::new();
    let allocator = Allocator::new(&arena);

    for _ in 0..100_000 {
        let ptr = allocator.alloc(3 as u8).unwrap();
        unsafe {
            let val = ptr.as_ref();
            assert_eq!(*val, 3);
        }
    }
}

#[test]
fn alloc_too_big() {
    let arena = Arena::new();
    let allocator = Allocator::new(&arena);
    let result = allocator.alloc_sized(std::u32::MAX);
    assert!(result.is_err());
}

#[test]
fn alloc_two_large_arrays() {
    let arena = Arena::new();
    let allocator = Allocator::new(&arena);
    allocator.alloc_sized((BLOCK_CAPACITY / 2) as u32).unwrap();
    assert_eq!(arena.get_size(), BLOCK_SIZE);
    allocator.alloc_sized((BLOCK_CAPACITY / 2) as u32).unwrap();
    assert_eq!(arena.get_size(), BLOCK_SIZE * 2);
}

#[test]
fn refresh_arena() {
    let arena = Arena::new();
    let allocator = Allocator::new(&arena);
    for i in 0..20 {
        allocator.alloc_sized((BLOCK_CAPACITY / 2) as u32).unwrap();
    }
    assert!(arena.get_size() > 10 * BLOCK_SIZE);
    arena.refresh();
    assert_eq!(arena.get_size(), 10 * BLOCK_SIZE);
}

#[test]
fn clone_size_class() {
    // this is just for test coverage
    let foo = SizeClass::get_for_size(69);
    let clone = foo.clone();

    assert!(foo == clone);
}
