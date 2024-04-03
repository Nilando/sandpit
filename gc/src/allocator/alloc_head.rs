use super::block_store::BlockStore;
use super::bump_block::BumpBlock;
use super::errors::AllocError;
use super::header::Mark;
use super::size_class::SizeClass;
use std::cell::Cell;
use std::sync::Arc;

pub struct AllocHead {
    head: Cell<Option<BumpBlock>>,
    overflow: Cell<Option<BumpBlock>>,
    block_store: Arc<BlockStore>,
    mark: Mark,
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
    pub fn new(block_store: Arc<BlockStore>, mark: Mark) -> Self {
        Self {
            head: Cell::new(None),
            overflow: Cell::new(None),
            block_store,
            mark,
        }
    }

    pub fn alloc(&self, alloc_size: usize, size_class: SizeClass) -> Result<*const u8, AllocError> {
        if let Some(space) = self.head_alloc(alloc_size) {
            return Ok(space);
        }

        match size_class {
            SizeClass::Small => self.small_alloc(alloc_size),
            SizeClass::Medium => self.medium_alloc(alloc_size),
            SizeClass::Large => self.block_store.create_large(alloc_size),
        }
    }

    fn small_alloc(&self, alloc_size: usize) -> Result<*const u8, AllocError> {
        loop {
            // this is okay be we already tried to alloc in head and didn't have space
            self.get_new_head()?;

            if let Some(space) = self.head_alloc(alloc_size) {
                return Ok(space);
            }
        }
    }

    fn medium_alloc(&self, alloc_size: usize) -> Result<*const u8, AllocError> {
        loop {
            if let Some(space) = self.overflow_alloc(alloc_size) {
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

        if rest_block.is_some() {
            self.block_store.push_rest(rest_block.unwrap());
        }

        Ok(())
    }

    fn get_new_overflow(&self) -> Result<(), AllocError> {
        let new_overflow = self.block_store.get_overflow()?;
        let recycle_block = self.overflow.take();
        self.overflow.set(Some(new_overflow));

        if recycle_block.is_some() {
            self.block_store.push_recycle(recycle_block.unwrap());
        }

        Ok(())
    }

    fn head_alloc(&self, alloc_size: usize) -> Option<*const u8> {
        match self.head.take() {
            Some(mut head) => {
                let result = head.inner_alloc(alloc_size, self.mark);
                self.head.set(Some(head));
                result
            }
            None => None,
        }
    }

    fn overflow_alloc(&self, alloc_size: usize) -> Option<*const u8> {
        match self.overflow.take() {
            Some(mut overflow) => {
                let result = overflow.inner_alloc(alloc_size, self.mark);
                self.overflow.set(Some(overflow));
                result
            }
            None => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::constants;
    use super::*;

    #[test]
    fn test_recycle_alloc() {
        let store = Arc::new(BlockStore::new());
        let mut blocks = AllocHead::new(store.clone(), Mark::Red);

        blocks
            .alloc(
                constants::BLOCK_CAPACITY - constants::LINE_SIZE,
                SizeClass::Medium,
            )
            .unwrap();
        assert_eq!(store.block_count(), 1);

        blocks
            .alloc(
                constants::BLOCK_CAPACITY - constants::LINE_SIZE,
                SizeClass::Medium,
            )
            .unwrap();
        assert_eq!(store.block_count(), 2);

        blocks
            .alloc(
                constants::BLOCK_CAPACITY - constants::LINE_SIZE,
                SizeClass::Medium,
            )
            .unwrap();
        assert_eq!(store.block_count(), 3);

        // this alloc should alloc should fill the head
        blocks
            .alloc(constants::LINE_SIZE, SizeClass::Small)
            .unwrap();
        assert_eq!(store.block_count(), 3);

        // this alloc should alloc should fill the overflow head
        blocks
            .alloc(constants::LINE_SIZE, SizeClass::Small)
            .unwrap();
        assert_eq!(store.block_count(), 3);

        // this alloc should alloc should fill the recycle
        blocks
            .alloc(constants::LINE_SIZE, SizeClass::Small)
            .unwrap();
        assert_eq!(store.block_count(), 3);

        // this alloc should alloc should need a new block
        blocks
            .alloc(constants::LINE_SIZE, SizeClass::Small)
            .unwrap();
        assert_eq!(store.block_count(), 4);
    }

    #[test]
    fn test_alloc_many_blocks() {
        let store = Arc::new(BlockStore::new());
        let mut blocks = AllocHead::new(store.clone(), Mark::Red);

        for i in 1..100 {
            blocks
                .alloc(constants::BLOCK_CAPACITY, SizeClass::Medium)
                .unwrap();
            assert_eq!(store.block_count(), i);
        }
    }

    #[test]
    fn test_alloc_into_overflow() {
        let store = Arc::new(BlockStore::new());
        let mut blocks = AllocHead::new(store.clone(), Mark::Red);

        blocks
            .alloc(
                constants::BLOCK_CAPACITY - constants::LINE_SIZE,
                SizeClass::Small,
            )
            .unwrap();
        blocks
            .alloc(constants::BLOCK_CAPACITY / 2, SizeClass::Medium)
            .unwrap();
        blocks
            .alloc(constants::BLOCK_CAPACITY / 2, SizeClass::Medium)
            .unwrap();
        assert_eq!(store.block_count(), 2);

        blocks
            .alloc(constants::BLOCK_CAPACITY / 2, SizeClass::Medium)
            .unwrap();
        blocks
            .alloc(constants::BLOCK_CAPACITY / 2, SizeClass::Medium)
            .unwrap();
        assert_eq!(store.block_count(), 3);
    }
}
