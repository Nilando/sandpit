use crate::allocate::{Allocate, GenerationalArena};
use super::Allocator;
use super::arena::Arena;
use super::constants::BLOCK_SIZE;

#[test]
fn arena_size() {
    let arena = Arena::new();
    let allocator = Allocator::new(&arena);
    let name = "Hello Alloc";

    assert_eq!(arena.get_size(), 0);

    allocator.alloc(name).unwrap();

    assert_eq!(arena.get_size(), BLOCK_SIZE);
}
