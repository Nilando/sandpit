use super::block_store::BlockStore;
use super::bump_block::BumpBlock;
use super::constants::BLOCK_SIZE;
use super::error::AllocError;
use super::size_class::SizeClass;
use std::alloc::Layout;
use std::cell::Cell;
use std::sync::Arc;

pub struct AllocHead {
    head: Cell<Option<BumpBlock>>,
    overflow: Cell<Option<BumpBlock>>,
    block_store: Arc<BlockStore>,
}

impl Clone for AllocHead {
    fn clone(&self) -> Self {
        Self {
            head: Cell::new(None),
            overflow: Cell::new(None),
            block_store: self.block_store.clone(),
        }
    }
}

impl Drop for AllocHead {
    fn drop(&mut self) {
        if let Some(head) = self.head.take() {
            self.block_store.push_recycle(head);
        }

        if let Some(overflow) = self.overflow.take() {
            self.block_store.push_recycle(overflow);
        }
    }
}

impl AllocHead {
    pub fn new(block_store: Arc<BlockStore>) -> Self {
        Self {
            head: Cell::new(None),
            overflow: Cell::new(None),
            block_store,
        }
    }

    pub fn alloc(&self, layout: Layout) -> Result<*const u8, AllocError> {
        if let Some(space) = self.head_alloc(layout) {
            return Ok(space);
        }

        let size_class = SizeClass::get_for_size(layout.size())?;
        match size_class {
            SizeClass::Small => self.small_alloc(layout),
            SizeClass::Medium => self.medium_alloc(layout),
            SizeClass::Large => self.block_store.create_large(layout),
        }
    }

    fn small_alloc(&self, layout: Layout) -> Result<*const u8, AllocError> {
        // this is okay be we already tried to alloc in head and didn't have space
        // and any block returned by get new head should have space for a small object
        loop {
            self.get_new_head()?;

            if let Some(ptr) = self.head_alloc(layout) {
                return Ok(ptr);
            }
        }
    }

    fn medium_alloc(&self, layout: Layout) -> Result<*const u8, AllocError> {
        loop {
            if let Some(space) = self.overflow_alloc(layout) {
                return Ok(space);
            }

            self.get_new_overflow()?;
        }
    }

    fn get_new_head(&self) -> Result<(), AllocError> {
        let new_head = match self.overflow.take() {
            Some(block) => block,
            None => self.block_store.get_head()?,
        };

        let rest_block = self.head.take();
        self.head.set(Some(new_head));

        if let Some(block) = rest_block {
            self.block_store.push_rest(block);
        }

        Ok(())
    }

    fn get_new_overflow(&self) -> Result<(), AllocError> {
        let new_overflow = self.block_store.get_overflow()?;
        let recycle_block = self.overflow.take();
        self.overflow.set(Some(new_overflow));

        if let Some(block) = recycle_block {
            self.block_store.push_recycle(block);
        }

        Ok(())
    }

    fn head_alloc(&self, layout: Layout) -> Option<*const u8> {
        match self.head.take() {
            Some(mut head) => {
                let result = head.inner_alloc(layout);
                self.head.set(Some(head));
                result
            }
            None => None,
        }
    }

    fn overflow_alloc(&self, layout: Layout) -> Option<*const u8> {
        match self.overflow.take() {
            Some(mut overflow) => {
                let result = overflow.inner_alloc(layout);
                self.overflow.set(Some(overflow));
                result
            }
            None => None,
        }
    }

    pub fn get_size(&self) -> usize {
        let block_space = self.block_store.block_count() * BLOCK_SIZE;
        let large_space = self.block_store.count_large_space();

        block_space + large_space
    }

    pub fn sweep(&self, live_mark: u8) {
        self.block_store.sweep(live_mark);
    }

    pub fn is_sweeping(&self) -> bool {
        self.block_store.is_sweeping()
    }
}

#[cfg(test)]
mod tests {
    use super::super::constants;
    use super::*;

    #[test]
    fn test_recycle_alloc() {
        let store = Arc::new(BlockStore::new());
        let blocks = AllocHead::new(store.clone());
        let medium_layout =
            Layout::from_size_align(constants::BLOCK_CAPACITY - constants::LINE_SIZE, 8).unwrap();
        let small_layout = Layout::from_size_align(constants::LINE_SIZE, 8).unwrap();

        blocks.alloc(medium_layout).unwrap();
        assert_eq!(store.block_count(), 1);

        blocks.alloc(medium_layout).unwrap();
        assert_eq!(store.block_count(), 2);

        blocks.alloc(medium_layout).unwrap();
        assert_eq!(store.block_count(), 3);

        // this alloc should alloc should fill the head
        blocks.alloc(small_layout).unwrap();
        assert_eq!(store.block_count(), 3);

        // this alloc should alloc should fill the overflow head
        blocks.alloc(small_layout).unwrap();
        assert_eq!(store.block_count(), 3);

        // this alloc should alloc should fill the recycle
        blocks.alloc(small_layout).unwrap();
        assert_eq!(store.block_count(), 3);

        // this alloc should alloc should need a new block
        blocks.alloc(small_layout).unwrap();
        assert_eq!(store.block_count(), 4);
    }

    #[test]
    fn test_alloc_many_blocks() {
        let store = Arc::new(BlockStore::new());
        let blocks = AllocHead::new(store.clone());
        let medium_layout = Layout::from_size_align(constants::BLOCK_CAPACITY, 8).unwrap();

        for i in 1..100 {
            blocks.alloc(medium_layout).unwrap();
            assert_eq!(store.block_count(), i);
        }
    }

    #[test]
    fn test_alloc_into_overflow() {
        let store = Arc::new(BlockStore::new());
        let blocks = AllocHead::new(store.clone());
        let medium_layout = Layout::from_size_align(constants::BLOCK_CAPACITY, 8).unwrap();
        let medium_layout_2 = Layout::from_size_align(constants::BLOCK_CAPACITY / 2, 8).unwrap();

        blocks.alloc(medium_layout).unwrap();
        blocks.alloc(medium_layout_2).unwrap();
        blocks.alloc(medium_layout_2).unwrap();
        assert_eq!(store.block_count(), 2);

        blocks.alloc(medium_layout_2).unwrap();
        blocks.alloc(medium_layout_2).unwrap();
        assert_eq!(store.block_count(), 3);
    }

    #[test]
    fn medium_and_small_allocs() {
        let store = Arc::new(BlockStore::new());
        let blocks = AllocHead::new(store.clone());
        let medium_layout = Layout::new::<[u8; constants::LINE_SIZE * 2]>();
        let small_layout = Layout::from_size_align(constants::LINE_SIZE, 8).unwrap();
        let mut small_ptrs = Vec::<*const u8>::new();
        let mut med_ptrs = Vec::<*const u8>::new();

        for _ in 0..2000 {
            let ptr = blocks.alloc(small_layout).unwrap();
            small_ptrs.push(ptr);

            let med_ptr = blocks.alloc(medium_layout).unwrap();
            med_ptrs.push(med_ptr);
        }

        while let Some(ptr) = small_ptrs.pop() {
            assert!(!med_ptrs.contains(&ptr));
            assert!(!small_ptrs.contains(&ptr));
        }
    }
}
