use std::alloc::Layout;
use std::sync::atomic::{AtomicU8, Ordering};
use std::marker::PhantomData;

// does the allocator need to be aware of the header being used?
// to mark an object we need its alloc layout
// we need to mark a layout to mark an object
pub trait GcHeader {
    fn get_mark(&self) -> GcMark;
    fn set_mark(&self, mark: GcMark);
    fn get_alloc_layout(&self) -> Layout;
}

#[repr(u8)]
#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub enum GcMark {
    Red,
    Green,
    Blue,
}

impl GcMark {
    pub fn rotate(&self) -> Self {
        match self {
            GcMark::Red   => GcMark::Green,
            GcMark::Green => GcMark::Blue,
            GcMark::Blue  => GcMark::Red,
        }
    }

    pub fn prev(&self) -> Self {
        self.rotate().rotate()
    }
}

impl From<u8> for GcMark {
    fn from(value: u8) -> Self {
        match value {
            x if x == GcMark::Red as u8 => GcMark::Red,
            x if x == GcMark::Green as u8 => GcMark::Green,
            x if x == GcMark::Blue as u8 => GcMark::Blue,
            _ => panic!("Bad GC GcMark"),
        }
    }
}

pub struct SizedHeader<T> {
    mark: AtomicU8,
    _item_type: PhantomData<T>
}

impl<T> SizedHeader<T> {
    pub fn new(mark: GcMark) -> Self {
        Self {
            mark: AtomicU8::new(mark as u8),
            _item_type: PhantomData::<T>
        }
    }
}

impl<T> GcHeader for SizedHeader<T> {
    fn set_mark(&self, mark: GcMark) {
        self.mark.store(mark as u8, Ordering::Release);
    }

    fn get_mark(&self) -> GcMark {
        self.mark.load(Ordering::Acquire).into()
    }

    fn get_alloc_layout(&self) -> Layout {
        let header_layout = Layout::new::<SizedHeader<T>>();
        let val_layout = Layout::new::<T>();
        let (alloc_layout, _) = header_layout
            .extend(val_layout)
            .expect("remove this expect");

        alloc_layout.pad_to_align()
    }
}

// for dynamically sized types
pub struct SliceHeader<T> {
    mark: AtomicU8,
    len: usize,
    _item_type: PhantomData<T>
}

impl<T> SliceHeader<T> {
    pub fn new(mark: GcMark, len: usize) -> Self {
        Self {
            mark: AtomicU8::new(mark as u8),
            len,
            _item_type: PhantomData::<T>
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }
}

impl<T> GcHeader for SliceHeader<T> {
    fn set_mark(&self, mark: GcMark) {
        self.mark.store(mark as u8, Ordering::Release);
    }

    fn get_mark(&self) -> GcMark {
        self.mark.load(Ordering::Acquire).into()
    }

    fn get_alloc_layout(&self) -> Layout {
        let header_layout = Layout::new::<SliceHeader<T>>();
        let slice_layout = Layout::array::<T>(self.len).expect("todo remove this expect");
        let (alloc_layout, _) = header_layout
            .extend(slice_layout)
            .expect("todo remove this expect");
        alloc_layout.pad_to_align()
    }
}
