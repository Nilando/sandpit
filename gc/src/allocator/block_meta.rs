use super::constants;
use super::header::Header;
use super::header::Mark;
use super::allocate::Marker;
use super::size_class::SizeClass;
use std::sync::atomic::{AtomicU8, Ordering};

pub struct BlockMeta {
    lines: *const AtomicU8,
}

impl BlockMeta {
    pub fn new(block_ptr: *const u8) -> BlockMeta {
        let meta = Self::from_block(block_ptr);
        meta.reset();
        meta
    }

    pub fn from_block(block_ptr: *const u8) -> Self {
        Self {
            lines: unsafe { block_ptr.add(constants::LINE_MARK_START) as *const AtomicU8 },
        }
    }

    pub fn from_header(header: *const Header) -> Self {
        let offset = (header as usize) % constants::BLOCK_SIZE;
        let block_ptr = unsafe { (header as *const u8).sub(offset) };

        Self::from_block(block_ptr)
    }

    pub fn mark(&self, header: &Header, mark: Mark) {
        let addr = header as *const Header as usize;

        let relative_ptr = addr % constants::BLOCK_SIZE;

        let line = relative_ptr / constants::LINE_SIZE;

        if header.get_size_class() == SizeClass::Small {
            self.set_line(line, mark);
        } else {
            let num_lines = header.get_size() / constants::LINE_SIZE as u16;

            for i in 0..num_lines {
                self.set_line(line + i as usize, mark);
            }
        }

        self.set_block(mark);
    }

    pub fn free_unmarked(&self, mark: Mark) {
        if self.get_block() != mark {
            self.set_block(Mark::New);
        }

        for i in 0..(constants::LINE_COUNT - 1) {
            if self.get_line(i) != mark {
                self.set_line(i, mark);
            }
        }
    }

    pub fn get_block(&self) -> Mark {
        self.get_line(constants::LINE_COUNT - 1)
    }

    fn get_line(&self, line: usize) -> Mark {
        self.mark_at(line).load(Ordering::SeqCst).into()
    }

    fn mark_at(&self, line: usize) -> &AtomicU8 {
        unsafe { &*self.lines.add(line) }
    }

    pub fn set_line(&self, index: usize, mark: Mark) {
        self.mark_at(index).store(mark as u8, Ordering::SeqCst)
    }

    pub fn set_block(&self, mark: Mark) {
        self.set_line(constants::LINE_COUNT - 1, mark)
    }

    pub fn reset(&self) {
        for i in 0..constants::LINE_COUNT {
            self.set_line(i, Mark::New);
        }
    }

    pub fn find_next_available_hole(
        &self,
        starting_at: usize,
        alloc_size: usize,
    ) -> Option<(usize, usize)> {
        let mut count = 0;
        let starting_line = starting_at / constants::LINE_SIZE;
        let lines_required = (alloc_size + constants::LINE_SIZE - 1) / constants::LINE_SIZE;
        let mut end = starting_line;

        for index in (0..starting_line).rev() {
            let line_mark = self.get_line(index);

            if line_mark.is_new() {
                count += 1;

                if index == 0 && count >= lines_required {
                    let limit = index * constants::LINE_SIZE;
                    let cursor = end * constants::LINE_SIZE;
                    return Some((cursor, limit));
                }
            } else {
                if count > lines_required {
                    let limit = (index + 2) * constants::LINE_SIZE;
                    let cursor = end * constants::LINE_SIZE;
                    return Some((cursor, limit));
                }

                count = 0;
                end = index;
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::super::block::Block;
    use super::*;

    #[test]
    fn test_mark_block() {
        // A set of marked lines with a couple holes.
        // The first hole should be seen as conservatively marked.
        // The second hole should be the one selected.
        let block = Block::default().unwrap();
        let meta = BlockMeta::new(block.as_ptr());

        meta.set_block(Mark::Red);
        let got = meta.get_block();

        assert_eq!(got, Mark::Red);
    }

    #[test]
    fn test_mark_line() {
        // A set of marked lines with a couple holes.
        // The first hole should be seen as conservatively marked.
        // The second hole should be the one selected.
        let block = Block::default().unwrap();
        let meta = BlockMeta::new(block.as_ptr());

        meta.set_line(0, Mark::Red);

        let expect = Mark::Red;
        let got = meta.get_line(0);

        assert_eq!(got, expect);
    }

    #[test]
    fn test_find_next_hole() {
        // A set of marked lines with a couple holes.
        // The first hole should be seen as conservatively marked.
        // The second hole should be the one selected.
        let block = Block::default().unwrap();
        let meta = BlockMeta::new(block.as_ptr());

        meta.set_line(0, Mark::Red);
        meta.set_line(1, Mark::Red);
        meta.set_line(2, Mark::Red);
        meta.set_line(4, Mark::Red);
        meta.set_line(10, Mark::Red);

        // line 5 should be conservatively marked
        let expect = Some((10 * constants::LINE_SIZE, 6 * constants::LINE_SIZE));

        let got = meta.find_next_available_hole(
            10 * constants::LINE_SIZE,
            constants::LINE_SIZE,
        );

        assert_eq!(got, expect);
    }

    #[test]
    fn test_find_next_hole_at_line_zero() {
        // Should find the hole starting at the beginning of the block
        let block = Block::default().unwrap();
        let meta = BlockMeta::new(block.as_ptr());

        meta.set_line(3, Mark::Red);
        meta.set_line(4, Mark::Red);
        meta.set_line(5, Mark::Red);

        let expect = Some((3 * constants::LINE_SIZE, 0));

        let got = meta.find_next_available_hole(
            3 * constants::LINE_SIZE,
            constants::LINE_SIZE,
        );

        assert_eq!(got, expect);
    }

    #[test]
    fn test_find_next_hole_at_block_end() {
        // The first half of the block is marked.
        // The second half of the block should be identified as a hole.
        let block = Block::default().unwrap();
        let meta = BlockMeta::new(block.as_ptr());
        let halfway = constants::LINE_COUNT / 2;

        for i in halfway..constants::LINE_COUNT {
            meta.set_line(i, Mark::Red);
        }

        // because halfway line should be conservatively marked
        let expect = Some((halfway * constants::LINE_SIZE, 0));
        let got = meta.find_next_available_hole(
            constants::BLOCK_CAPACITY,
            constants::LINE_SIZE,
        );

        assert_eq!(got, expect);
    }

    #[test]
    fn test_find_hole_all_conservatively_marked() {
        // Every other line is marked.
        // No hole should be found.
        let block = Block::default().unwrap();
        let meta = BlockMeta::new(block.as_ptr());

        for i in 0..constants::LINE_COUNT {
            if i % 2 == 0 {
                // there is no stable step function for range
                meta.set_line(i, Mark::Red);
            }
        }

        let got = meta.find_next_available_hole(
            constants::BLOCK_CAPACITY,
            constants::LINE_SIZE,
        );
        assert_eq!(got, None);
    }

    #[test]
    fn test_find_entire_block() {
        // No marked lines. Entire block is available.
        let block = Block::default().unwrap();
        let meta = BlockMeta::new(block.as_ptr());
        let expect = Some((constants::BLOCK_CAPACITY, 0));
        let got = meta.find_next_available_hole(
            constants::BLOCK_CAPACITY,
            constants::LINE_SIZE,
        );

        assert_eq!(got, expect);
    }
}
