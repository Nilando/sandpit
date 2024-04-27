pub const BLOCK_SIZE: usize = 1024 * 32;
pub const LINE_SIZE: usize = 128;

// How many total lines are in a block
pub const LINE_COUNT: usize = BLOCK_SIZE / LINE_SIZE;

pub const BLOCK_CAPACITY: usize = BLOCK_SIZE - LINE_COUNT;
pub const LINE_MARK_START: usize = BLOCK_CAPACITY;

pub const MAX_ALLOC_SIZE: usize = std::u32::MAX as usize;
pub const SMALL_OBJECT_MIN: usize = 1;
pub const SMALL_OBJECT_MAX: usize = LINE_SIZE;
pub const MEDIUM_OBJECT_MIN: usize = SMALL_OBJECT_MAX + 1;
pub const MEDIUM_OBJECT_MAX: usize = BLOCK_CAPACITY;
pub const LARGE_OBJECT_MIN: usize = MEDIUM_OBJECT_MAX + 1;
pub const LARGE_OBJECT_MAX: usize = MAX_ALLOC_SIZE;
