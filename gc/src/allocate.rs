use std::ptr::NonNull;

pub trait Allocate {
    type Arena: GenerationalArena;
    type Error;
    // type Mark

    fn new(arena: &Self::Arena) -> Self;
    fn alloc<T>(&self, object: T) -> Result<NonNull<T>, Self::Error>;
    fn alloc_sized(&mut self, len: u32) -> Result<NonNull<u8>, Self::Error>;
    // get mark -> Mark
    // set mark
}

pub trait GenerationalArena {
    fn new() -> Self;
    fn refresh(&self);
    fn get_size(&self) -> usize;
}
