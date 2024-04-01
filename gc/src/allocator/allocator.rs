use crate::allocate::Allocate;
use std::slice::from_raw_parts_mut;
use super::size_class::SizeClass;
use super::errors::AllocError;
use super::alloc_head::AllocHead;
use super::header::Header;
use super::constants::{ALIGN, aligned_size};
use super::arena::Arena;
use std::ptr::NonNull;
use std::ptr::write;

pub struct Allocator {
    head: AllocHead,
}

impl Allocator {
    fn get_space(&self, size_class: SizeClass, alloc_size: usize) -> Result<*const u8, AllocError>
    {
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
            head: AllocHead::new(arena.get_block_store()),
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
}
