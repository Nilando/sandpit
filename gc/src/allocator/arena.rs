use std::sync::Arc;
use super::block_store::BlockStore;
use crate::allocate::GenerationalArena;


#[derive(Clone)]
pub struct Arena {
    block_store: Arc<BlockStore>
}

impl Arena {
    pub fn new() -> Self {
        Self {
            block_store: Arc::new(BlockStore::new())
        }
    }

    pub fn get_block_store(&self) -> Arc<BlockStore> {
        self.block_store.clone()
    }
}

impl GenerationalArena for Arena {
    fn start_eden_trace(&self) {}
    fn start_full_trace(&self) {}
    fn complete_trace(&self) {}
}
