pub trait Allocate {
    type Arena;

    fn new_arena() -> Self::Arena;
    fn new_allocator(arena: &Self::Arena) -> Self;
}
