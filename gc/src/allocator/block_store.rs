use super::block::Block;
use super::bump_block::BumpBlock;
use super::constants::ALIGN;
use super::errors::AllocError;

use super::header::Mark;
use std::collections::LinkedList;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

pub struct BlockStore {
    block_count: AtomicUsize,
    // alloc_size: AtomicUsize,
    free: Mutex<Vec<BumpBlock>>,
    recycle: Mutex<Vec<BumpBlock>>,
    rest: Mutex<Vec<BumpBlock>>,
    large: Mutex<LinkedList<Block>>,
}

impl BlockStore {
    pub fn new() -> Self {
        Self {
            block_count: AtomicUsize::new(0),
            //alloc_size: AtomicUsize::new(0),
            free: Mutex::new(vec![]),
            recycle: Mutex::new(vec![]),
            rest: Mutex::new(vec![]),
            large: Mutex::new(LinkedList::new()),
        }
    }

    pub fn push_rest(&self, block: BumpBlock) {
        self.rest.lock().unwrap().push(block);
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

    pub fn count_large_space(&self) -> usize {
        self.large
            .lock()
            .unwrap()
            .iter()
            .map(|block| block.size())
            .sum::<usize>()
    }

    pub fn create_large(&self, alloc_size: usize) -> Result<*const u8, AllocError> {
        let block = Block::new(alloc_size, ALIGN)?;
        let ptr = block.as_ptr();
        self.large.lock().unwrap().push_front(block);
        // self.block_count.fetch_add(1, Ordering::SeqCst); // technically this is a block.. but
        // really is being considered a 'large' instead
        Ok(ptr)
    }

    pub fn refresh(&self) {
        // this should use tri color
        // reserve
        // new
        // old
        //
        // partial
    }
}
