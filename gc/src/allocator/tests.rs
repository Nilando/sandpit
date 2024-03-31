use crate::allocate::{Allocate, GenerationalArena};
use super::Allocator;
use super::arena::Arena;
use super::constants::BLOCK_SIZE;
use std::mem::size_of;

#[test]
fn arena_size() {
    let arena = Arena::new();
    let allocator = Allocator::new(&arena);
    let name = "Hello Alloc";

    assert_eq!(arena.get_size(), 0);

    allocator.alloc(name).unwrap();

    assert_eq!(arena.get_size(), BLOCK_SIZE);
}

#[test]
fn alloc_large() {
    let arena = Arena::new();
    let allocator = Allocator::new(&arena);
    let data: [usize; BLOCK_SIZE] = [0; BLOCK_SIZE];

    assert_eq!(arena.get_size(), 0);

    allocator.alloc(data).unwrap();

    assert_eq!(arena.get_size(), BLOCK_SIZE * size_of::<usize>());
}
