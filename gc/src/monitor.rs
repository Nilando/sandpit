use super::allocate::GenerationalArena;
use std::thread;
use std::time;
use std::sync::Arc;

pub struct Monitor<T: GenerationalArena> {
    arena: Arc<T>
}

unsafe impl<T: GenerationalArena> Send for Monitor<T> {}
unsafe impl<T: GenerationalArena> Sync for Monitor<T> {}

impl<T: GenerationalArena> Monitor<T> {
    pub fn new(arena: Arc<T>) -> Self {
        Self {
            arena
        }
    }

    pub fn monitor(&self) {
        let ten_millis = time::Duration::from_millis(500);

        thread::sleep(ten_millis);}
}
