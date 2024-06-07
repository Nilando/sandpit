use super::block::Block;
use super::bump_block::BumpBlock;
use super::errors::AllocError;
use super::header::Header;
use super::header::Mark;
use std::alloc::Layout;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

pub struct BlockStore {
    block_count: AtomicUsize,
    free: Mutex<Vec<BumpBlock>>,
    recycle: Mutex<Vec<BumpBlock>>,
    rest: Mutex<Vec<BumpBlock>>,
    large: Mutex<Vec<Block>>,
}

impl BlockStore {
    pub fn new() -> Self {
        Self {
            block_count: AtomicUsize::new(0),
            free: Mutex::new(vec![]),
            recycle: Mutex::new(vec![]),
            rest: Mutex::new(vec![]),
            large: Mutex::new(vec![]),
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
            self.new_block()
        }
    }

    pub fn get_overflow(&self) -> Result<BumpBlock, AllocError> {
        if let Some(free_block) = self.free.lock().unwrap().pop() {
            Ok(free_block)
        } else {
            self.new_block()
        }
    }

    fn new_block(&self) -> Result<BumpBlock, AllocError> {
        self.block_count.fetch_add(1, Ordering::SeqCst);
        BumpBlock::new()
    }

    pub fn block_count(&self) -> usize {
        self.block_count.load(Ordering::Relaxed)
    }

    pub fn count_large_space(&self) -> usize {
        self.large
            .lock()
            .unwrap()
            .iter()
            .fold(0, |sum, block| sum + block.get_size())
    }

    pub fn create_large(&self, layout: Layout) -> Result<*const u8, AllocError> {
        let block = Block::new(layout)?;
        let ptr = block.as_ptr();

        self.large.lock().unwrap().push(block);
        Ok(ptr)
    }

    pub fn refresh(&self, mark: Mark) {
        let mut free = self.free.lock().unwrap();
        let mut rest = self.rest.lock().unwrap();
        let mut large = self.large.lock().unwrap();
        let mut recycle = self.recycle.lock().unwrap();
        let mut new_rest = vec![];
        let mut new_recycle = vec![];
        let mut new_large = vec![];

        loop {
            match recycle.pop() {
                Some(mut block) => {
                    block.reset_hole(mark);

                    if block.is_marked(mark) {
                        new_recycle.push(block);
                    } else {
                        free.push(block);
                    }
                }
                None => break,
            }
        }

        loop {
            match rest.pop() {
                Some(mut block) => {
                    block.reset_hole(mark);

                    if block.is_marked(mark) {
                        if block.current_hole_size() != 0 {
                            new_recycle.push(block);
                        } else {
                            new_rest.push(block);
                        }
                    } else {
                        free.push(block);
                    }
                }
                None => break,
            }
        }

        loop {
            match large.pop() {
                Some(block) => {
                    let header_ptr = block.as_ptr() as *const Header;
                    if Header::get_mark(header_ptr) == mark {
                        new_large.push(block);
                    }
                }
                None => break,
            }
        }

        *rest = new_rest;
        *recycle = new_recycle;
        *large = new_large;

        // TODO: ADD 10 as a CONFIG FREE_RATE
        for _ in 0..10_000 {
            if free.len() == 0 {
                break;
            }

            self.block_count.fetch_sub(1, Ordering::Relaxed);
            free.pop();
        }
    }
}
