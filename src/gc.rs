use std::sync::{
    Mutex,
    atomic::{AtomicU8, Ordering}
};
use crate::mutator::Mutator;

unsafe impl Send for Gc {}
unsafe impl Sync for Gc {}

#[repr(u8)]
#[derive(Copy, Clone)]
enum GcState {
    Default,
    EdenCollection,
    FullCollection,
}

impl From<u8> for GcState {
    fn from(n: u8) -> Self {
        match n {
            0 => GcState::Default,
            1 => GcState::PartialTracing,
            2 => GcState::FullTracing,
        }
    }
}

pub struct Gc {
    state: AtomicU8,
    mutators: Mutex<Vec<Mutator>>,
    // eden spaces
    // old spaces
}

impl Gc {
    pub fn new() -> Self {
        Self {
            state: AtomicU8::new(GcState::Default as u8),
            mutators: Mutex::new(vec![]),
        }
    }

    pub fn new_mutator(&self) -> &Mutator {
        let mutator = Mutator::new(self);
        let mut mutators = self.mutators.lock().unwrap();

        mutators.push(mutator);
        &mutator
    }

    pub fn get_state(&self) -> GcState {
        self.state.load(Ordering::Relaxed).into()
    }

    pub fn set_state(&self) -> GcState {
        todo!()
    }
}

mod tests {
    use super::*;

    #[test]
    fn gc_is_send() {
        let gc = Gc::new();
        let mut threads = vec![];

        for _ in 0..10 {
            threads.push(std::thread::spawn(move || {
                let mutator = gc.new_mutator();
            }));
        }

        threads.iter().map(|thread| thread.join());
    }

}
