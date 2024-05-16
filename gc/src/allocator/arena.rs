use super::allocate::GenerationalArena;
use super::block_store::BlockStore;
use super::constants::BLOCK_SIZE;
use super::header::Mark;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};

#[derive(Clone)]
pub struct Arena {
    block_store: Arc<BlockStore>,
    current_mark: Arc<AtomicU8>,
}

impl Arena {
    pub fn get_block_store(&self) -> Arc<BlockStore> {
        self.block_store.clone()
    }

    pub fn get_current_mark_ref(&self) -> Arc<AtomicU8> {
        self.current_mark.clone()
    }
}

impl GenerationalArena for Arena {
    type Mark = Mark;

    fn new() -> Self {
        Self {
            block_store: Arc::new(BlockStore::new()),
            current_mark: Arc::new(AtomicU8::new(Mark::Red as u8)),
        }
    }

    fn refresh(&self) {
        self.block_store.refresh(self.current_mark());
    }

    fn get_size(&self) -> usize {
        let block_space = self.block_store.block_count() * BLOCK_SIZE;
        let large_space = self.block_store.count_large_space();

        block_space + large_space
    }

    fn current_mark(&self) -> Self::Mark {
        Mark::from(self.current_mark.load(Ordering::SeqCst))
    }

    fn rotate_mark(&self) -> Self::Mark {
        let mark = self.current_mark();
        let new_mark = mark.rotate();

        self.current_mark.store(new_mark as u8, Ordering::SeqCst);

        new_mark
    }
}
