use std::ptr::NonNull;

pub trait Allocate {
    type Arena: GenerationalArena;
    type Error;

    fn new_arena() -> Self::Arena;
    fn new_allocator(arena: &Self::Arena) -> Self;
    fn alloc<T>(&self, object: T) -> Result<NonNull<T>, Self::Error>;
    fn alloc_sized(&mut self, len: u32) -> Result<NonNull<u8>, Self::Error>;
    // fn mark(NonNull<T>, mark: u8);
    // fn get_mark(NonNull<T>) -> u8;
}

pub trait GenerationalArena {
    //type Mark: Marker;

    fn start_eden_trace(&self); // preps objects in eden space for freeing
    fn start_full_trace(&self); // preps all object space for freeing
    fn complete_trace(&self); // completes the trace, final step for freeing memory
}

//trait Marker {

//}
