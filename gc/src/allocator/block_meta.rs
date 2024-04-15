use super::constants;
use super::header::Header;
use super::header::Mark;
use super::size_class::SizeClass;
use core::ptr::NonNull;

pub struct BlockMeta {
    lines: *mut u8,
}

impl BlockMeta {
    pub fn new(block_ptr: *const u8) -> BlockMeta {
        let mut meta = Self::from_block(block_ptr);
        meta.reset();
        meta
    }

    pub fn from_block(block_ptr: *const u8) -> Self {
        Self {
            lines: unsafe { block_ptr.add(constants::LINE_MARK_START) as *mut u8 },
        }
    }

    pub fn from_header(header: &Header) -> Self {
        let addr =  header as *const Header as usize;
        let block_start = (addr - (addr % constants::BLOCK_SIZE)) as *const u8;

        Self::from_block(block_start as *const u8)
    }

    pub fn mark(&mut self, header: &Header, mark: Mark) {
        let addr =  header as *const Header as usize;

        let relative_ptr = (addr as usize) % constants::BLOCK_SIZE;

        let line = relative_ptr / constants::LINE_SIZE;

        if header.get_size_class() == SizeClass::Small {
            self.mark_line(line, mark);
        } else {
            let num_lines = header.get_size() / constants::LINE_SIZE as u16;

            for i in 0..num_lines {
                self.mark_line(line + i as usize, mark);
            }
        }

        self.mark_block(mark);
    }

    pub fn get_mark(&self) -> Mark {
        unsafe { Mark::from(*self.lines.add(constants::LINE_COUNT - 1)) }
    }

    unsafe fn as_line_mark(&mut self, line: usize) -> &mut u8 {
        &mut *self.lines.add(line)
    }

    pub fn mark_line(&mut self, index: usize, mark: Mark) {
        unsafe { *self.as_line_mark(index) = mark as u8 };
    }

    pub fn mark_block(&mut self, mark: Mark) {
        unsafe { *self.lines.add(constants::LINE_COUNT - 1) = mark as u8 }
    }

    pub fn reset(&mut self) {
        unsafe {
            for i in 0..constants::LINE_COUNT {
                *self.lines.add(i) = Mark::New as u8;
            }
        }
    }

    pub fn find_next_available_hole(
        &self,
        starting_at: usize,
        alloc_size: usize,
        mark: Mark,
    ) -> Option<(usize, usize)> {
        let mut count = 0;
        let starting_line = starting_at / constants::LINE_SIZE;
        let lines_required = (alloc_size + constants::LINE_SIZE - 1) / constants::LINE_SIZE;
        let mut end = starting_line;

        for index in (0..starting_line).rev() {
            let line_mark = unsafe { *self.lines.add(index) };

            if line_mark != mark as u8 {
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
        let mut meta = BlockMeta::new(block.as_ptr());

        meta.mark_block(Mark::Red);
        let got = meta.get_mark();

        assert_eq!(got, Mark::Red);
    }

    #[test]
    fn test_mark_line() {
        // A set of marked lines with a couple holes.
        // The first hole should be seen as conservatively marked.
        // The second hole should be the one selected.
        let block = Block::default().unwrap();
        let mut meta = BlockMeta::new(block.as_ptr());

        meta.mark_line(0, Mark::Red);

        let expect = 1;
        let got = unsafe { meta.as_line_mark(0) };

        assert_eq!(*got, expect);
    }

    #[test]
    fn test_find_next_hole() {
        // A set of marked lines with a couple holes.
        // The first hole should be seen as conservatively marked.
        // The second hole should be the one selected.
        let block = Block::default().unwrap();
        let mut meta = BlockMeta::new(block.as_ptr());

        meta.mark_line(0, Mark::Red);
        meta.mark_line(1, Mark::Red);
        meta.mark_line(2, Mark::Red);
        meta.mark_line(4, Mark::Red);
        meta.mark_line(10, Mark::Red);

        // line 5 should be conservatively marked
        let expect = Some((10 * constants::LINE_SIZE, 6 * constants::LINE_SIZE));

        let got = meta.find_next_available_hole(
            10 * constants::LINE_SIZE,
            constants::LINE_SIZE,
            Mark::Red,
        );

        assert_eq!(got, expect);
    }

    #[test]
    fn test_find_next_hole_at_line_zero() {
        // Should find the hole starting at the beginning of the block
        let block = Block::default().unwrap();
        let mut meta = BlockMeta::new(block.as_ptr());

        meta.mark_line(3, Mark::Red);
        meta.mark_line(4, Mark::Red);
        meta.mark_line(5, Mark::Red);

        let expect = Some((3 * constants::LINE_SIZE, 0));

        let got = meta.find_next_available_hole(
            3 * constants::LINE_SIZE,
            constants::LINE_SIZE,
            Mark::Red,
        );

        assert_eq!(got, expect);
    }

    #[test]
    fn test_find_next_hole_at_block_end() {
        // The first half of the block is marked.
        // The second half of the block should be identified as a hole.
        let block = Block::default().unwrap();
        let mut meta = BlockMeta::new(block.as_ptr());
        let halfway = constants::LINE_COUNT / 2;

        for i in halfway..constants::LINE_COUNT {
            meta.mark_line(i, Mark::Red);
        }

        // because halfway line should be conservatively marked
        let expect = Some((halfway * constants::LINE_SIZE, 0));
        let got = meta.find_next_available_hole(
            constants::BLOCK_CAPACITY,
            constants::LINE_SIZE,
            Mark::Red,
        );

        assert_eq!(got, expect);
    }

    #[test]
    fn test_find_hole_all_conservatively_marked() {
        // Every other line is marked.
        // No hole should be found.
        let block = Block::default().unwrap();
        let mut meta = BlockMeta::new(block.as_ptr());

        for i in 0..constants::LINE_COUNT {
            if i % 2 == 0 {
                // there is no stable step function for range
                meta.mark_line(i, Mark::Red);
            }
        }

        let got = meta.find_next_available_hole(
            constants::BLOCK_CAPACITY,
            constants::LINE_SIZE,
            Mark::Red,
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
            Mark::Red,
        );

        assert_eq!(got, expect);
    }
}
