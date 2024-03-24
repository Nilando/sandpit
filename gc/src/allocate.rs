use std::ptr::NonNull;

pub trait Allocate {
    type Arena;
    type Error;

    fn new_arena() -> Self::Arena;
    fn new_allocator(arena: &Self::Arena) -> Self;
    fn alloc<T>(&self, object: T) -> Result<NonNull<T>, Self::Error>;
    fn alloc_sized(&mut self, len: u32) -> Result<NonNull<u8>, Self::Error>;
}
