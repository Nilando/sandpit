use super::alloc_head::AllocHead;
use super::arena::Arena;
use super::block_meta::BlockMeta;
use super::constants::{aligned_size, ALIGN};
use super::errors::AllocError;
use super::header::Header;
use super::header::Mark;
use super::size_class::SizeClass;
use crate::allocate::{Allocate, GenerationalArena};
use std::ptr::write;
use std::ptr::NonNull;
use std::mem::{align_of, size_of};
use std::alloc::Layout;

pub struct Allocator {
    head: AllocHead,
}

impl Allocator {
    fn get_space(&self, layout: Layout) -> Result<*const u8, AllocError> {
        self.head.alloc(layout)
    }

    fn get_header<T>(object: &NonNull<T>) -> &Header {
        unsafe {
            let align = std::cmp::max(align_of::<Header>(), align_of::<T>());
            let header_size = size_of::<Header>();
            let padding = header_size % align;
            let ptr = object.as_ptr().cast::<u8>();
            let header_ptr = ptr.sub(header_size + padding) as *mut Header;

            &*header_ptr
        }
    }

    fn aligned_array_size(size: usize) -> usize {
        if size % ALIGN == 0 {
            size
        } else {
            size + (ALIGN - (size % ALIGN))
        }
    }
}

impl Allocate for Allocator {
    type Arena = Arena;
    type Error = AllocError;

    fn new(arena: &Self::Arena) -> Self {
        Self {
            head: AllocHead::new(arena.get_block_store(), arena.current_mark()),
        }
    }

    fn alloc(&self, layout: Layout) -> Result<NonNull<u8>, AllocError> {
        let align = std::cmp::max(align_of::<Header>(), layout.align());
        let header_size = size_of::<Header>();
        let padding = header_size % align;
        let alloc_size = header_size + padding + layout.size();
        let size_class = SizeClass::get_for_size(alloc_size)?;
        let header = Header::new(size_class, alloc_size as u16);

        unsafe {
            let space = self.get_space(Layout::from_size_align_unchecked(alloc_size, align))?;
            let object_space = space.add(header_size + padding);
            write(space as *mut Header, header);
            // write(object_space as *mut T, object);
            Ok(NonNull::new(object_space as *mut u8).unwrap())
        }
    }

    fn get_mark<T>(ptr: NonNull<T>) -> Mark {
        let header = Self::get_header(&ptr);

        header.get_mark()
    }

    fn set_mark<T>(ptr: NonNull<T>, mark: Mark) {
        let header = Self::get_header(&ptr);
        let size_class = header.get_size_class();

        header.set_mark(mark);

        if size_class != SizeClass::Large {
            let mut meta = BlockMeta::from_obj(ptr.cast());

            meta.mark(ptr.cast(), size_class, header.get_size().into(), mark);
        }
    }
}
