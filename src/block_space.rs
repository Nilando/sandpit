use crate::space::Space;
use crate::constants::BLOCK_SIZE;
use crate::bump_block::BumpBlock;

pub struct BlockSpace {
    space: Space,
    used: Vec<BumpBlock>,
    recycling: Vec<BumpBlock>,
    free: Vec<BumpBlock>,
}

impl BlockSpace {
    pub fn new() -> Self {
        let space = Space::new();
        let blocks = Self::blocks_from_space(&space);

        Self {
            space,
            used: vec![],
            recycling: vec![],
            free: blocks
        }
    }

    pub fn blocks_from_space(space: &Space) -> Vec<BumpBlock> {
        let mut ptr = space.start();
        let mut blocks = vec![];

        while ptr < space.end() {
            let block = BumpBlock::from_ptr(ptr as *mut u8);
            blocks.push(block);
            unsafe { ptr = ptr.add(BLOCK_SIZE) };
        }

        blocks
    }
}


mod test {
    use super::*;

    #[test]
    fn get_blocks() {
        let space = BlockSpace::new();
    }
}
