use super::pointee::{sized_alloc_layout, slice_alloc_layout};
use std::alloc::Layout;
use std::sync::atomic::{AtomicU8, Ordering};
use std::marker::PhantomData;
use std::num::NonZero;

// does the allocator need to be aware of the header being used?
// to mark an object we need its alloc layout
// we need to mark a layout to mark an object
pub trait GcHeader: Sized {
    fn get_mark(&self) -> GcMark;
    fn set_mark(&self, mark: GcMark);
    fn get_alloc_layout(&self) -> Layout;
}

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

impl From<GcMark> for u8 {
    fn from(value: GcMark) -> Self {
        match value {
            GcMark::Red => 1,
            GcMark::Green => 2,
            GcMark::Blue => 3,
        }
    }
}

impl From<u8> for GcMark {
    fn from(value: u8) -> Self {
        match value {
            1 => GcMark::Red,
            2 => GcMark::Green,
            3 => GcMark::Blue,
            _ => {
                println!("Bad GC Mark, aborting process!");
                std::process::abort();
            }
        }
    }
}

impl From<GcMark> for NonZero<u8> {
    fn from(value: GcMark) -> Self {
        NonZero::new(value.into()).unwrap()
    }
}

pub struct SizedHeader<T> {
    mark: AtomicU8,
    _item_type: PhantomData<T>
}

impl<T> SizedHeader<T> {
    pub fn new(mark: GcMark) -> Self {
        Self {
            mark: AtomicU8::new(mark.into()),
            _item_type: PhantomData::<T>
        }
    }
}

impl<T> GcHeader for SizedHeader<T> {
    fn set_mark(&self, mark: GcMark) {
        self.mark.store(mark.into(), Ordering::Release);
    }

    fn get_mark(&self) -> GcMark {
        self.mark.load(Ordering::Acquire).into()
    }

    fn get_alloc_layout(&self) -> Layout {
        let (layout, _) = sized_alloc_layout::<T>();

        layout
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
            mark: AtomicU8::new(mark.into()),
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
        self.mark.store(mark.into(), Ordering::SeqCst);
    }

    fn get_mark(&self) -> GcMark {
        self.mark.load(Ordering::SeqCst).into()
    }

    fn get_alloc_layout(&self) -> Layout {
        let (layout, _) = slice_alloc_layout::<T>(self.len);

        layout
    }
}
