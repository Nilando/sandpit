use std::alloc::Layout;
use std::sync::atomic::{AtomicU8, Ordering};

// does the allocator need to be aware of the header being used?
// to mark an object we need its alloc layout
// we need to mark a layout to mark an object
pub trait GcHeader {
    fn get_mark(&self) -> GcMark;
    fn set_mark(&self, mark: GcMark);
    fn get_alloc_layout<T>(&self) -> Layout;
    fn as_ptr(&self) -> *mut u8 {
        self as *const Self as *const u8 as *mut u8
    }
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
            GcMark::Red   => GcMark::Green,
            GcMark::Green => GcMark::Blue,
            GcMark::Blue  => GcMark::Red,
            _             => panic!("Attempted to rotate a mark that shouldn't be rotated"),
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

pub struct SizedHeader {
    mark: AtomicU8,
}

impl SizedHeader {
    pub fn new() -> Self {
        Self {
            mark: AtomicU8::new(GcMark::New as u8),
        }
    }
}

impl GcHeader for SizedHeader {
    fn set_mark(&self, mark: GcMark) {
        self.mark.store(mark as u8, Ordering::Release);
    }

    fn get_mark(&self) -> GcMark {
        self.mark.load(Ordering::Acquire).into()
    }

    fn get_alloc_layout<T>(&self) -> Layout {
        let header_layout = Layout::new::<SizedHeader>();
        let val_layout = Layout::new::<T>();
        let (alloc_layout, _) = header_layout
            .extend(val_layout)
            .expect("remove this expect");

        alloc_layout.pad_to_align()
    }
}

// for dynamically sized types
pub struct SliceHeader {
    mark: AtomicU8,
    len: usize,
}

impl SliceHeader {
    pub fn new(len: usize) -> Self {
        Self {
            mark: AtomicU8::new(GcMark::New as u8),
            len
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }
}

impl GcHeader for SliceHeader {
    fn set_mark(&self, mark: GcMark) {
        self.mark.store(mark as u8, Ordering::Release);
    }

    fn get_mark(&self) -> GcMark {
        self.mark.load(Ordering::Acquire).into()
    }

    fn get_alloc_layout<T>(&self) -> Layout {
        let header_layout = Layout::new::<SliceHeader>();
        let slice_layout = Layout::array::<T>(self.len).expect("todo remove this expect");
        let (alloc_layout, _) = header_layout
            .extend(slice_layout)
            .expect("todo remove this expect");
        alloc_layout.pad_to_align()
    }
}