use std::sync::Arc;
use super::block_store::BlockStore;
use crate::allocate::GenerationalArena;

#[derive(Clone)]
pub struct Arena {
    block_store: Arc<BlockStore>
}

impl Arena {
    pub fn get_block_store(&self) -> Arc<BlockStore> {
        self.block_store.clone()
    }
}

impl GenerationalArena for Arena {
    fn new() -> Self {
        Self {
            block_store: Arc::new(BlockStore::new())
        }
    }

    fn refresh(&self) {
    todo!()
    }
}
