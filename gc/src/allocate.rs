use std::fmt::Debug;
use std::ptr::NonNull;

pub trait Allocate: 'static {
    type Arena: GenerationalArena;
    type Error;

    fn new(arena: &Self::Arena) -> Self;
    fn alloc<T>(&self, object: T) -> Result<NonNull<T>, Self::Error>;
    fn alloc_sized(&self, len: u32) -> Result<NonNull<u8>, Self::Error>;
    fn get_mark<T>(ptr: NonNull<T>) -> <<Self as Allocate>::Arena as GenerationalArena>::Mark;
    fn set_mark<T>(ptr: NonNull<T>, mark: <<Self as Allocate>::Arena as GenerationalArena>::Mark);
}

pub trait GenerationalArena {
    type Mark: Marker;

    fn new() -> Self;
    fn refresh(&self);
    fn get_size(&self) -> usize;
    fn block_count(&self) -> usize;
    fn current_mark(&self) -> Self::Mark;
    fn rotate_mark(&self);
}

pub trait Marker: Copy + Clone + PartialEq + Eq + Debug {
    fn is_new(&self) -> bool;
}
