use super::allocator::AllocMark;
use super::block::Block;
use super::bump_block::BumpBlock;
use super::error::AllocError;
use std::alloc::Layout;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

pub struct BlockStore {
    block_count: AtomicUsize,
    free: Mutex<Vec<BumpBlock>>,
    recycle: Mutex<Vec<BumpBlock>>,
    rest: Mutex<Vec<BumpBlock>>,
    large: Mutex<Vec<Block>>,
    sweep_lock: Mutex<()>,
}

impl BlockStore {
    pub fn new() -> Self {
        Self {
            block_count: AtomicUsize::new(0),
            free: Mutex::new(vec![]),
            recycle: Mutex::new(vec![]),
            rest: Mutex::new(vec![]),
            large: Mutex::new(vec![]),
            sweep_lock: Mutex::new(()),
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

    // large objects are stored with a single byte of meta info to store their mark
    pub fn create_large(&self, obj_layout: Layout) -> Result<*const u8, AllocError> {
        let header_layout = Layout::new::<AllocMark>();
        let (header_obj_layout, obj_offset) = header_layout
            .extend(obj_layout)
            .expect("todo: turn this into an alloc error");
        let block = Block::new(header_obj_layout)?;
        let ptr = unsafe { block.as_ptr().add(obj_offset) };

        self.large.lock().unwrap().push(block);

        Ok(ptr)
    }

    pub fn is_sweeping(&self) -> bool {
        self.sweep_lock.try_lock().is_err()
    }

    pub fn sweep(&self, mark: u8) {
        let _sweep_lock = self.sweep_lock.lock();
        let mut free = self.free.lock().unwrap();
        let mut rest = self.rest.lock().unwrap();
        let mut large = self.large.lock().unwrap();
        let mut recycle = self.recycle.lock().unwrap();
        let mut new_rest = vec![];
        let mut new_recycle = vec![];
        let mut new_large = vec![];

        while let Some(mut block) = recycle.pop() {
            block.reset_hole(mark);

            if block.is_marked(mark) {
                new_recycle.push(block);
            } else {
                free.push(block);
            }
        }

        while let Some(mut block) = rest.pop() {
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

        while let Some(block) = large.pop() {
            let header_mark = unsafe { &*(block.as_ptr() as *const AllocMark) };

            if header_mark.load(Ordering::Relaxed) == mark {
                new_large.push(block);
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
