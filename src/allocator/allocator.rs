use super::alloc_head::AllocHead;
use super::allocate::Allocate;
use super::arena::Arena;
use super::errors::AllocError;
use super::header::Header;
use super::header::Mark;
use super::size_class::SizeClass;
use std::alloc::Layout;
use std::mem::{align_of, size_of};
use std::ptr::write;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;

pub struct Allocator {
    head: AllocHead,
    current_mark: Arc<AtomicU8>,
}

impl Allocator {
    pub fn get_header<'a, T>(object: NonNull<T>) -> *const Header {
        let align = std::cmp::max(align_of::<Header>(), align_of::<T>());
        let header_size = size_of::<Header>();
        let padding = (align - (header_size % align)) % align;
        let ptr: *mut u8 = object.as_ptr().cast::<u8>();

        debug_assert!((ptr as usize % align) == 0);
        debug_assert!((object.as_ptr() as usize % align_of::<T>()) == 0);

        unsafe { ptr.sub(header_size + padding) as *const Header }
    }

    fn get_current_mark(&self) -> Mark {
        Mark::from(self.current_mark.load(Ordering::SeqCst))
    }
}

impl Allocate for Allocator {
    type Arena = Arena;
    type Error = AllocError;

    fn new(arena: &Self::Arena) -> Self {
        let current_mark = arena.get_current_mark_ref();

        Self {
            head: AllocHead::new(arena.get_block_store()),
            current_mark,
        }
    }

    fn alloc(&self, layout: Layout) -> Result<NonNull<u8>, AllocError> {
        let align = std::cmp::max(align_of::<Header>(), layout.align());
        let header_size = size_of::<Header>();
        let padding = (align - (header_size % align)) % align;
        let alloc_size = header_size + padding + layout.size();
        let size_class = SizeClass::get_for_size(alloc_size)?;
        // Alloc size could be greater than u16, causing overflow conversion from (as u16).
        // This is okay though, b/c in that case the object will be SizeClass::Large
        // where the header size is unused. Normally the header size is used,
        // for marking block lines, but large objects are stored in bump blocks.
        let header = Header::new(size_class, alloc_size as u16);

        unsafe {
            let alloc_layout = Layout::from_size_align_unchecked(alloc_size, align);
            let space = self.head.alloc(alloc_layout)?;
            let object_space = space.add(header_size + padding);

            write(space as *mut Header, header);
            Header::mark_new(space as *const Header);
            Ok(NonNull::new(object_space as *mut u8).unwrap())
        }
    }

    fn get_mark<T>(ptr: NonNull<T>) -> Mark {
        let header = Self::get_header(ptr);

        debug_assert!(Header::debug::<T>(header));

        Header::get_mark(header)
    }

    fn set_mark<T>(ptr: NonNull<T>, mark: Mark) {
        let header = Self::get_header(ptr);

        debug_assert!(Header::debug::<T>(header));

        Header::set_mark(header, mark);
    }

    fn is_old<T>(&self, ptr: NonNull<T>) -> bool {
        let header = Self::get_header(ptr);

        debug_assert!(Header::debug::<T>(header));

        Header::get_mark(header) == self.get_current_mark()
    }
}
