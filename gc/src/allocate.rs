use std::ptr::NonNull;

pub trait Allocate {
    type Arena: GenerationalArena;
    type Error;

    fn new(arena: &Self::Arena) -> Self;
    fn alloc<T>(&self, object: T) -> Result<NonNull<T>, Self::Error>;
    fn alloc_sized(&mut self, len: u32) -> Result<NonNull<u8>, Self::Error>;
}

pub trait GenerationalArena {
    fn new() -> Self;
    fn refresh(&self);
}
