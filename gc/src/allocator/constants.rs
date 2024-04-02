pub const BLOCK_SIZE: usize = 1024 * 32;
pub const LINE_SIZE: usize = 128;

// How many total lines are in a block
pub const LINE_COUNT: usize = BLOCK_SIZE / LINE_SIZE;

pub const BLOCK_CAPACITY: usize = BLOCK_SIZE - LINE_COUNT;
pub const LINE_MARK_START: usize = BLOCK_CAPACITY;

pub const ALLOC_ALIGN_BYTES: usize = 16;
pub const ALLOC_ALIGN_MASK: usize = !(ALLOC_ALIGN_BYTES - 1);

pub const MAX_ALLOC_SIZE: usize = std::u32::MAX as usize;
pub const SMALL_OBJECT_MIN: usize = 1;
pub const SMALL_OBJECT_MAX: usize = LINE_SIZE;
pub const MEDIUM_OBJECT_MIN: usize = SMALL_OBJECT_MAX + 1;
pub const MEDIUM_OBJECT_MAX: usize = BLOCK_CAPACITY;
pub const LARGE_OBJECT_MIN: usize = MEDIUM_OBJECT_MAX + 1;
pub const LARGE_OBJECT_MAX: usize = MAX_ALLOC_SIZE;

pub const ALIGN: usize = std::mem::size_of::<usize>();
pub const fn aligned_size<T: Sized>() -> usize {
    let size = std::mem::size_of::<T>();
    if size % ALIGN == 0 {
        size
    } else {
        size + (ALIGN - (size % ALIGN))
    }
}
