use super::block::Block;
use super::block_meta::BlockMeta;
use super::constants::{BLOCK_CAPACITY, SMALL_OBJECT_MIN};
use super::error::AllocError;
use std::alloc::Layout;

pub struct BumpBlock {
    cursor: usize,
    limit: usize,
    block: Block,
    meta: BlockMeta,
}

impl BumpBlock {
    pub fn new() -> Result<BumpBlock, AllocError> {
        let inner_block = Block::default()?;
        let block_ptr = inner_block.as_ptr();
        let block = BumpBlock {
            cursor: BLOCK_CAPACITY,
            limit: 0,
            block: inner_block,
            meta: BlockMeta::new(block_ptr),
        };

        Ok(block)
    }

    pub fn reset_hole(&mut self, mark: u8) {
        self.meta.free_unmarked(mark);

        if self.meta.get_block() != mark {
            self.cursor = BLOCK_CAPACITY;
            self.limit = 0;
            return;
        }

        if let Some((cursor, limit)) = self
            .meta
            .find_next_available_hole(BLOCK_CAPACITY, SMALL_OBJECT_MIN)
        {
            self.cursor = cursor;
            self.limit = limit;
        } else {
            self.cursor = 0;
            self.limit = 0;
        }
    }

    pub fn inner_alloc(&mut self, layout: Layout) -> Option<*const u8> {
        loop {
            let next_ptr = self.cursor.checked_sub(layout.size())? & !(layout.align() - 1);

            if self.limit <= next_ptr {
                self.cursor = next_ptr;

                return Some(self.block.at_offset(self.cursor));
            }

            if let Some((cursor, limit)) = self
                .meta
                .find_next_available_hole(self.limit, layout.size())
            {
                self.cursor = cursor;
                self.limit = limit;
            } else {
                return None;
            }
        }
    }

    pub fn current_hole_size(&self) -> usize {
        self.cursor - self.limit
    }

    pub fn is_marked(&self, mark: u8) -> bool {
        self.meta.get_block() == mark
    }
}

#[cfg(test)]
mod tests {
    use super::super::constants::LINE_COUNT;
    use super::*;

    #[test]
    fn test_empty_block() {
        let mut b = BumpBlock::new().unwrap();

        for i in 0..BLOCK_CAPACITY {
            let ptr = b.inner_alloc(Layout::new::<u8>()).unwrap();

            let offset = BLOCK_CAPACITY - (i + 1);
            assert_eq!(b.cursor, offset);
            assert!(ptr as usize == b.block.as_ptr() as usize + offset);
        }

        assert!(b.inner_alloc(Layout::new::<u8>()).is_none());
    }

    #[test]
    fn test_full_block() {
        let mut b = BumpBlock::new().unwrap();

        for i in 0..LINE_COUNT {
            b.meta.set_line(i, 1);
        }

        b.reset_hole(1);

        assert!(b.inner_alloc(Layout::new::<u8>()).is_none());
    }

    #[test]
    fn test_half_block() {
        let mut b = BumpBlock::new().unwrap();

        for i in ((LINE_COUNT - 2) / 2)..LINE_COUNT {
            b.meta.set_line(i, 1);
        }

        b.reset_hole(1);

        for i in 0..(BLOCK_CAPACITY / 2) {
            let ptr = b.inner_alloc(Layout::new::<u8>()).unwrap();

            let offset = (BLOCK_CAPACITY / 2) - (i + 1);
            assert_eq!(b.cursor, offset);
            assert!(ptr as usize == b.block.as_ptr() as usize + offset);
        }

        assert!(b.inner_alloc(Layout::new::<u8>()).is_none());
        assert_eq!(b.cursor, 0);
        assert_eq!(b.limit, 0);
    }

    #[test]
    fn test_conservatively_marked_block() {
        let mut b = BumpBlock::new().unwrap();

        for i in 0..LINE_COUNT {
            if i % 2 == 0 {
                b.meta.set_line(i, 1);
            }
        }

        b.meta.set_block(1);

        b.reset_hole(1);

        assert!(b.inner_alloc(Layout::new::<u8>()).is_none());
        assert_eq!(b.cursor, 0);
        assert_eq!(b.limit, 0);
    }

    #[test]
    fn test_current_hole_size() {
        let block = BumpBlock::new().unwrap();
        let expect = block.current_hole_size();

        assert_eq!(expect, BLOCK_CAPACITY);
    }
}
