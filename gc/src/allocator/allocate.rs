use std::alloc::Layout;
use std::fmt::Debug;
use std::ptr::NonNull;

pub trait Allocate: 'static {
    type Arena: GenerationalArena;
    type Error;

    fn new(arena: &Self::Arena) -> Self;
    fn alloc(&self, layout: Layout) -> Result<NonNull<u8>, Self::Error>;
    fn get_mark<T>(ptr: NonNull<T>) -> <<Self as Allocate>::Arena as GenerationalArena>::Mark;
    fn set_mark<T>(ptr: NonNull<T>, mark: <<Self as Allocate>::Arena as GenerationalArena>::Mark);

    fn check_if_old<T>(&self, ptr: NonNull<T>) -> bool;
}

pub trait GenerationalArena {
    type Mark: Marker;

    fn new() -> Self;
    fn refresh(&self);
    fn get_size(&self) -> usize;
    fn current_mark(&self) -> Self::Mark;
    fn rotate_mark(&self) -> Self::Mark;
}

pub trait Marker: Copy + Clone + PartialEq + Eq + Debug + Send + Sync {
    fn new() -> Self;
    fn is_new(&self) -> bool;
}
