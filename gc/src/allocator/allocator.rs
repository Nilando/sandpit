use super::alloc_head::AllocHead;
use super::arena::Arena;
use super::constants::{aligned_size, ALIGN};
use super::errors::AllocError;
use super::header::Header;
use super::header::Mark;
use super::size_class::SizeClass;
use super::block_meta::BlockMeta;
use crate::allocate::{Allocate, GenerationalArena};
use std::ptr::write;
use std::ptr::NonNull;
use std::slice::from_raw_parts_mut;

pub struct Allocator {
    head: AllocHead,
}

impl Allocator {
    fn get_space(&self, size_class: SizeClass, alloc_size: usize) -> Result<*const u8, AllocError> {
        self.head.alloc(alloc_size, size_class)
    }

    fn get_header(object: &NonNull<()>) -> &Header {
        unsafe { &*(object.as_ptr() as *const Header).offset(-1) }
    }

    fn get_object(header: &Header) -> NonNull<()> {
        let obj_addr = unsafe { (header as *const Header).offset(1) as *mut () };
        NonNull::new(obj_addr).unwrap()
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

    fn alloc<T>(&self, object: T) -> Result<NonNull<T>, AllocError> {
        let alloc_size = aligned_size::<Header>() + aligned_size::<T>();
        let size_class = SizeClass::get_for_size(alloc_size)?;
        let header = Header::new(size_class, alloc_size as u16);
        let space = self.get_space(size_class, alloc_size)?;

        unsafe {
            let object_space = space.add(aligned_size::<Header>());
            write(space as *mut Header, header);
            write(object_space as *mut T, object);
            Ok(NonNull::new(object_space as *mut T).unwrap())
        }
    }

    fn alloc_sized(&self, len: u32) -> Result<NonNull<u8>, AllocError> {
        let alloc_size = aligned_size::<Header>() + Self::aligned_array_size(len as usize);
        let size_class = SizeClass::get_for_size(alloc_size)?;
        let header = Header::new(size_class, alloc_size as u16);
        let space = self.get_space(size_class, alloc_size)?;

        unsafe {
            let array_space = space.add(aligned_size::<Header>());
            write(space as *mut Header, header);
            let array = from_raw_parts_mut(array_space as *mut u8, len as usize);

            for byte in array {
                *byte = 0;
            }

            Ok(NonNull::new(array_space as *mut u8).unwrap())
        }
    }

    fn get_mark<T>(ptr: NonNull<T>) -> Mark {
        let binding = ptr.cast();
        let header = Self::get_header(&binding);

        header.get_mark()
    }

    fn swap_mark<T>(ptr: NonNull<T>, mark: Mark) -> Mark {
        let binding = ptr.cast();
        let header = Self::get_header(&binding);

        header.swap_mark(mark)
    }

    fn set_mark<T>(ptr: NonNull<T>, mark: Mark) {
        let binding = ptr.cast();
        let header = Self::get_header(&binding);
        let mut meta = BlockMeta::from_obj(ptr.cast());
        if header.get_size_class() == SizeClass::Large { todo!() }

        header.set_mark(mark);
        meta.mark(ptr.cast(), header.get_size_class(), header.get_size().into(), mark);
    }
}
