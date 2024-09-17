use std::sync::atomic::{AtomicU8, Ordering};
use std::ptr::NonNull;
use std::alloc::Layout;

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
            x if x == GcMark::New as u8   => GcMark::New,
            x if x == GcMark::Red as u8   => GcMark::Red,
            x if x == GcMark::Green as u8 => GcMark::Green,
            x if x == GcMark::Blue as u8  => GcMark::Blue,
            _ => panic!("Bad GC GcMark"),
        }
    }
}

pub struct DynHeader {
    mark: AtomicU8,
    layout: Layout,
}

pub struct Header {
    mark: AtomicU8,
}

impl Header {
    pub fn get_ptr<T>(ptr: NonNull<T>) -> *const Self {
        let header_layout = Layout::new::<Header>();
        let object_layout = Layout::new::<T>();
        let (_, object_offset) = header_layout.extend(object_layout).expect("Bad Alloc Layout");

        unsafe {
            let raw_ptr: *mut u8 = ptr.as_ptr().cast();
            let header_ptr = raw_ptr.sub(object_offset);

            header_ptr.cast()
        }
    }

    pub fn new() -> Self {
        Header {
            mark: AtomicU8::new(GcMark::New as u8),
        }
    }

    pub fn get_mark(&self) -> GcMark {
        self.mark.load(Ordering::Acquire).into()
    }

    pub fn set_mark(&self, mark: GcMark) {
        self.mark.store(mark as u8, Ordering::Release);
    }
}
