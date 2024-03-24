use super::bump_block::BumpBlock;
use super::errors::AllocError;
use super::constants::ALIGN;
use super::block::Block;

use std::sync::Mutex;
use std::collections::LinkedList;
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct BlockStore {
    block_count: AtomicUsize,
    // alloc_size: AtomicUsize,
    free: Mutex<Vec<BumpBlock>>,
    recycle: Mutex<Vec<BumpBlock>>,
    rest: Mutex<Vec<BumpBlock>>,
    used: Mutex<Vec<BumpBlock>>,
    large: Mutex<LinkedList<Block>>
}

impl BlockStore {
    pub fn new() -> Self {
        Self {
            block_count: AtomicUsize::new(0),
            //alloc_size: AtomicUsize::new(0),
            free: Mutex::new(vec![]),
            recycle: Mutex::new(vec![]),
            rest: Mutex::new(vec![]),
            used: Mutex::new(vec![]),
            large: Mutex::new(LinkedList::new()),
        }
    }

    pub fn push_used(&self, block: BumpBlock) {
        self.used.lock().unwrap().push(block);
    }

    pub fn push_recycle(&self, block: BumpBlock) {
        self.recycle.lock().unwrap().push(block);
    }

    pub fn get_head(&self) -> Result<BumpBlock, AllocError> {
        if let Some(recycle_block) = self.recycle.lock().unwrap().pop() {
            Ok(recycle_block)
        } else if let Some(free_block) = self.free.lock().unwrap().pop() {
            Ok(free_block)
        } else {
            self.block_count.fetch_add(1, Ordering::SeqCst);
            Ok(BumpBlock::new()?)
        }
    }

    pub fn get_overflow(&self) -> Result<BumpBlock, AllocError> {
        if let Some(free_block) = self.free.lock().unwrap().pop() {
            Ok(free_block)
        } else {
            self.block_count.fetch_add(1, Ordering::SeqCst);
            Ok(BumpBlock::new()?)
        }
    }

    pub fn block_count(&self) -> usize {
        self.block_count.load(Ordering::SeqCst)
    }

    pub fn create_large(&self, alloc_size: usize) -> Result<*const u8, AllocError> {
        let block = Block::new(alloc_size, ALIGN)?;
        let ptr = block.as_ptr();
        self.large.lock().unwrap().push_front(block);
        self.block_count.fetch_add(1, Ordering::SeqCst);
        Ok(ptr)
    }
}
