use std::alloc::Layout;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicU8, Ordering};

pub unsafe trait GcHeader {
    fn get_mark(&self) -> GcMark;
    fn set_mark(&self, mark: GcMark);
    fn as_ptr(&self) -> *mut u8;
    fn get_layout<T>(&self) -> Layout;
}

#[repr(u8)]
#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub enum GcMark {
    New,
    // these are the marks that rotate
    Red,
    Green,
    Blue,
}

impl GcMark {
    pub fn new() -> Self {
        Self::New
    }

    pub fn is_new(&self) -> bool {
        *self == Self::New
    }

    pub fn rotate(&self) -> Self {
        match self {
            GcMark::Red => GcMark::Green,
            GcMark::Green => GcMark::Blue,
            GcMark::Blue => GcMark::Red,
            _ => panic!("Attempted to rotate a mark that shouldn't be rotated"),
        }
    }
}

impl From<u8> for GcMark {
    fn from(value: u8) -> Self {
        match value {
            x if x == GcMark::New as u8 => GcMark::New,
            x if x == GcMark::Red as u8 => GcMark::Red,
            x if x == GcMark::Green as u8 => GcMark::Green,
            x if x == GcMark::Blue as u8 => GcMark::Blue,
            _ => panic!("Bad GC GcMark"),
        }
    }
}

pub struct Header {
    mark: AtomicU8,
}

impl Header {
    pub fn new() -> Self {
        Self {
            mark: AtomicU8::new(GcMark::New as u8),
        }
    }
}

unsafe impl GcHeader for Header {
    fn set_mark(&self, mark: GcMark) {
        self.mark.store(mark as u8, Ordering::Release);
    }

    fn get_mark(&self) -> GcMark {
        self.mark.load(Ordering::Acquire).into()
    }

    fn as_ptr(&self) -> *mut u8 {
        self as *const Header as *mut u8
    }

    fn get_layout<T: ?Sized>(&self) -> Layout {
        todo!()
    }
}


// for dynamically sized types
pub struct DynHeader {
    mark: AtomicU8,
    layout: Layout,
}

impl DynHeader {
    pub fn new(layout: Layout) -> Self {
        Self {
            mark: AtomicU8::new(GcMark::New as u8),
            layout
        }
    }
}

unsafe impl GcHeader for DynHeader {
    fn set_mark(&self, mark: GcMark) {
        todo!()
    }

    fn get_mark(&self) -> GcMark {
        todo!()
    }

    fn as_ptr(&self) -> *mut u8 {
        todo!()
    }

    fn get_layout<T>(&self) -> Layout {
        todo!()
    }
}
