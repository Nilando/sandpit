use std::ptr::NonNull;

pub trait Allocate {
    type Arena: GenerationalArena;
    type Error;

    fn new(arena: &Self::Arena) -> Self;
    fn alloc<T>(&self, object: T) -> Result<NonNull<T>, Self::Error>;
    fn alloc_sized(&self, len: u32) -> Result<NonNull<u8>, Self::Error>;
    fn get_mark<T>(ptr: NonNull<T>) -> <<Self as Allocate>::Arena as GenerationalArena>::Mark;
    fn set_mark<T>(ptr: NonNull<T>, mark: <<Self as Allocate>::Arena as GenerationalArena>::Mark);
    fn swap_mark<T>(
        ptr: NonNull<T>,
        mark: <<Self as Allocate>::Arena as GenerationalArena>::Mark,
    ) -> <<Self as Allocate>::Arena as GenerationalArena>::Mark;
}

pub trait GenerationalArena {
    type Mark: Copy + Clone;

    fn new() -> Self;
    fn refresh(&self);
    fn get_size(&self) -> usize;
    fn current_mark(&self) -> Self::Mark;
    fn rotate_mark(&self);
}