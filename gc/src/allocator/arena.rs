use super::block_store::BlockStore;
use super::constants::BLOCK_SIZE;
use super::header::Mark;
use crate::allocate::GenerationalArena;
use std::sync::Arc;
use std::sync::Mutex;

#[derive(Clone)]
pub struct Arena {
    block_store: Arc<BlockStore>,
    current_mark: Arc<Mutex<Mark>>,
}

impl Arena {
    pub fn get_block_store(&self) -> Arc<BlockStore> {
        self.block_store.clone()
    }
}

impl GenerationalArena for Arena {
    type Mark = Mark;

    fn new() -> Self {
        Self {
            block_store: Arc::new(BlockStore::new()),
            current_mark: Arc::new(Mutex::new(Mark::Red)),
        }
    }

    fn refresh(&self) {
        self.block_store.refresh();
    }

    fn get_size(&self) -> usize {
        let block_space = self.block_store.block_count() * BLOCK_SIZE;
        let large_space = self.block_store.count_large_space();

        block_space + large_space
    }

    fn current_mark(&self) -> Self::Mark {
        *self.current_mark.lock().unwrap()
    }

    fn rotate_mark(&self) {
        let mut mark = self.current_mark.lock().unwrap();

        *mark = mark.rotate();
    }
}
