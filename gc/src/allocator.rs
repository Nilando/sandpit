use super::allocate::Allocate;

pub struct Allocator {}

impl Allocator {}

impl Allocate for Allocator {
    type Arena = ();

    fn new_arena() -> Self::Arena { () }
    fn new_allocator(arena: &Self::Arena) -> Self {
        Self { }
    }
}
