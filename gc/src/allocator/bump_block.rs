use super::block::Block;
use super::block_meta::BlockMeta;
use super::constants;
use super::errors::AllocError;
use super::header::Mark;

pub struct BumpBlock {
    cursor: *const u8,
    limit: *const u8,
    block: Block,
    meta: BlockMeta,
}

impl BumpBlock {
    pub fn new() -> Result<BumpBlock, AllocError> {
        let inner_block = Block::default()?;
        let block_ptr = inner_block.as_ptr();
        let block = BumpBlock {
            cursor: unsafe { block_ptr.add(constants::BLOCK_CAPACITY) },
            limit: block_ptr,
            block: inner_block,
            meta: BlockMeta::new(block_ptr),
        };

        Ok(block)
    }

    pub fn reset_hole(&mut self, mark: Mark) {
        if let Some((cursor, limit)) = self
            .meta
            .find_next_available_hole(constants::BLOCK_CAPACITY, constants::SMALL_OBJECT_MIN, mark)
        {
            self.cursor = unsafe { self.block.as_ptr().add(cursor) };
            self.limit = unsafe { self.block.as_ptr().add(limit) };
        } else {
            self.cursor = self.block.as_ptr();
            self.limit = self.block.as_ptr();
        }
    }

    pub fn inner_alloc(&mut self, alloc_size: usize, mark: Mark) -> Option<*const u8> {
        loop {
            let ptr = self.cursor as usize;
            let next_ptr = ptr.checked_sub(alloc_size)? & constants::ALLOC_ALIGN_MASK;

            if self.current_hole_size() >= alloc_size {
                self.cursor = next_ptr as *const u8;
                return Some(self.cursor);
            }

            let block_relative_limit =
                unsafe { self.limit.sub(self.block.as_ptr() as usize) as usize };

            if let Some((cursor, limit)) = self
                .meta
                .find_next_available_hole(block_relative_limit, alloc_size, mark)
            {
                self.cursor = unsafe { self.block.as_ptr().add(cursor) };
                self.limit = unsafe { self.block.as_ptr().add(limit) };
            } else {
                return None;
            }
        }
    }

    pub fn current_hole_size(&self) -> usize {
        self.cursor as usize - self.limit as usize
    }

    pub fn reset_meta(&mut self) {
        self.meta.reset()
    }

    pub fn is_marked(&self, mark: Mark) -> bool {
        self.meta.get_mark() == mark
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    const TEST_UNIT_SIZE: usize = constants::ALLOC_ALIGN_BYTES;

    fn loop_check_allocate(b: &mut BumpBlock) -> usize {
        let mut v = Vec::new();
        let mut index = 0;

        loop {
            if let Some(ptr) = b.inner_alloc(TEST_UNIT_SIZE, Mark::Red) {
                let u32ptr = ptr as *mut u32;

                assert!(!v.contains(&u32ptr));

                v.push(u32ptr);
                unsafe { *u32ptr = index }

                index += 1;
            } else {
                break;
            }
        }

        for (index, u32ptr) in v.iter().enumerate() {
            unsafe {
                assert_eq!(**u32ptr, index as u32);
            }
        }

        index as usize
    }

    #[test]
    fn test_empty_block() {
        let mut b = BumpBlock::new().unwrap();

        let count = loop_check_allocate(&mut b);
        let expect = constants::BLOCK_CAPACITY / TEST_UNIT_SIZE;

        assert_eq!(count, expect);
    }

    #[test]
    fn test_half_block() {
        let mut b = BumpBlock::new().unwrap();

        for i in 0..(constants::LINE_COUNT / 2) {
            b.meta.mark_line(i, Mark::Red);
        }
        let occupied_bytes = (constants::LINE_COUNT / 2) * constants::LINE_SIZE;

        b.limit = b.cursor; // block is recycled

        let count = loop_check_allocate(&mut b);
        let expect =
            (constants::BLOCK_CAPACITY - constants::LINE_SIZE - occupied_bytes) / TEST_UNIT_SIZE;

        assert_eq!(count, expect);
    }

    #[test]
    fn test_conservatively_marked_block() {
        let mut b = BumpBlock::new().unwrap();

        for i in 0..constants::LINE_COUNT {
            if i % 2 == 0 {
                b.meta.mark_line(i, Mark::Red);
            }
        }

        b.limit = b.cursor;

        let count = loop_check_allocate(&mut b);

        assert_eq!(count, 0);
    }

    #[test]
    fn test_current_hole_size() {
        let block = BumpBlock::new().unwrap();
        let expect = block.current_hole_size();

        assert_eq!(expect, constants::BLOCK_CAPACITY);
    }
}
