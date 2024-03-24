use crate::allocate::Allocate;
use super::size_class::SizeClass;
use super::errors::AllocError;
use super::block_list::BlockList;
use super::block_store::BlockStore;
use super::header::Header;
use super::constants::ALIGN;
use std::sync::Arc;
use std::ptr::NonNull;
use std::mem::size_of;

use std::ptr::write;

pub struct Allocator {
    blocks: BlockList,
}

impl Allocator {
    pub fn new(block_store: Arc<BlockStore>) -> Allocator {
        Self {
            blocks: BlockList::new(block_store),
        }
    }

    fn get_space(&self, size_class: SizeClass, alloc_size: usize) -> Result<*const u8, AllocError>
    {
        self.blocks.alloc(alloc_size, size_class)
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

fn size_class<T>() -> SizeClass { todo!() }

impl Allocate for Allocator {
    type Arena = Arc<BlockStore>;
    type Error = AllocError;

    fn new_arena() -> Self::Arena {
        Arc::new(BlockStore::new())
    }

    fn new_allocator(arena: &Self::Arena) -> Self {
        Self::new(arena.clone())
    }

    fn alloc<T>(&self, object: T) -> Result<NonNull<T>, AllocError> {
        let obj_size = size_of::<T>();
        let obj_class = size_class::<T>();
        let space = self.get_space(obj_class, obj_size)?;
        let header = Header::new(obj_size as u16);

        unsafe {
            let object_space = space.add(Header::ALIGNED_SIZE);
            write(space as *mut Header, header);
            write(object_space as *mut T, object);
            Ok(NonNull::new(object_space as *mut T).unwrap())
        }
    }

    fn alloc_sized(&mut self, len: u32) -> Result<NonNull<u8>, AllocError> {
        todo!()
        /*
        let alloc_size = Header::ALIGNED_SIZE + Self::aligned_array_size(len as usize);
        let size_class = SizeClass::get_for_size(alloc_size)?;
        let space = self.get_space(size_class, alloc_size)?;
        let header = Header::array_header(alloc_size as u32, size_class);

        unsafe {
            let array_space = space.offset(Header::ALIGNED_SIZE as isize);
            let array = from_raw_parts_mut(array_space as *mut u8, alloc_size);

            write(space as *mut Header, header);
            for byte in array {
                *byte = 0;
            }
            
            Ok(RawPtr::new(array_space as *const u8))
        }
        */
    }
}
