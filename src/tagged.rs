use std::mem::ManuallyDrop;
use std::sync::atomic::{AtomicUsize, Ordering};
use super::gc::GcPointer;
use std::marker::PhantomData;


// TODO: Add a TaggedUsize<T: Tag> type

pub unsafe trait Tag: Sized {
    const VARIANTS: usize;

    fn into_usize(&self) -> usize;
    fn from_usize(tag: usize) -> Option<Self>;
}

pub union Tagged<A: GcPointer, B: Tag> {
    ptr: ManuallyDrop<A>,
    raw: ManuallyDrop<AtomicUsize>,
    _tag_type: PhantomData<B>
} 

impl<A: GcPointer, B: Tag> From<A> for Tagged<A, B> {
    fn from(value: A) -> Self {
        Self::const_assert();

        Self {
            ptr: ManuallyDrop::new(value)
        }
    }
}

impl<A: GcPointer, B: Tag> Clone for Tagged<A, B> {
    fn clone(&self) -> Self {
        let raw = unsafe { 
            self.raw.load(Ordering::Relaxed)
        };

        Self {
            raw: ManuallyDrop::new(AtomicUsize::new(raw))
        }
    }
}

impl<A: GcPointer, B: Tag> TryFrom<usize> for Tagged<A, B> {
    type Error = ();

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        Self::const_assert();

        match B::from_usize(value) {
            None => Err(()),
            Some(_) => {
                Ok(Self {
                    raw: ManuallyDrop::new(AtomicUsize::new(value))
                })
            }
        }
    }
}

impl<PTR: GcPointer, TAG: Tag> Tagged<PTR, TAG> {
    fn const_assert() {
        const { assert!(TAG::VARIANTS < PTR::POINTEE_ALIGNMENT) };
    }
}

impl<A: GcPointer, B: Tag> Tagged<A, B> {
    const TAG_MASK: usize = A::POINTEE_ALIGNMENT - 1;

    pub fn is_ptr(&self) -> bool {
        self.get_tag().is_none()
    }

    pub fn is_tagged(&self) -> bool {
        !self.is_ptr()
    }

    pub fn get_ptr(&self) -> Option<A> {
        if self.is_ptr() {
            unsafe {
                return Some((*self.ptr).clone())
            }
        }

        None
    }

    pub fn get_tag(&self) -> Option<B> {
        unsafe {
            let raw = self.raw.load(Ordering::Relaxed);

            B::from_usize(raw & Self::TAG_MASK)
        }
    }

    pub fn set_tag(&self, tag: B) {
        unsafe {
            let old_raw = self.raw.load(Ordering::Relaxed);
            let new_raw = Self::apply_tag(old_raw, tag);

            self.raw.store(new_raw, Ordering::Relaxed);
        }
    }

    pub fn get_raw(&self) -> Option<usize> {
        if self.is_ptr() {
            return None;
        }

        unsafe {
            Some(self.raw.load(Ordering::Relaxed))
        }
    }

    pub fn set_tagged_raw(&self, value: usize, tag: B) {
        let new_raw = Self::apply_tag(value, tag);

        unsafe {
            self.raw.store(new_raw, Ordering::Relaxed);
        }
    }

    pub unsafe fn set_ptr(&self, ptr: A) {
        self.ptr.set(ptr);
    }

    pub fn apply_tag(n: usize, tag: B) -> usize {
        Self::strip_tag(n) | tag.into_usize()
    }

    pub fn strip_tag(n: usize) -> usize {
        n & !Self::TAG_MASK
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{gc::{Gc, GcOpt}, Arena, Root};
    use sandpit_derive::Tag;

    #[derive(Tag)]
    enum MyTag {
        A,
        B,
        C,
    }

    #[test]
    fn test_into_pointer() {
        let _: Arena<Root![_]> = Arena::new(|mu| {
            let ptr = Gc::new(mu, 69);
            let tag = Tagged::<Gc<usize>, MyTag>::from(ptr.clone());
            let untagged_pointer = tag.get_ptr().unwrap();

            assert!(ptr.as_thin() == untagged_pointer.as_thin());
        });
    }


    #[test]
    fn test_into_bytes() {
        let _: Arena<Root![_]> = Arena::new(|_| {
            let tagged_bytes = 1;
            let tag = Tagged::<Gc<usize>, MyTag>::try_from(tagged_bytes).unwrap();
            let bytes = tag.get_raw();

            assert!(tagged_bytes == bytes.unwrap());
        });
    }

    #[test]
    fn test_setting_tag() {
        let _: Arena<Root![_]> = Arena::new(|mu| {
            let ptr = GcOpt::new(mu, 69);
            let tagged_bytes = Tagged::<GcOpt<usize>, MyTag>::apply_tag(2592345111, MyTag::A);
            let tag = Tagged::<GcOpt<usize>, MyTag>::from(ptr);
            tag.set_tagged_raw(tagged_bytes, MyTag::A);
            let bytes = tag.get_raw();

            assert!(tagged_bytes == bytes.unwrap());
        });
    }

    #[test]
    fn test_invalid_tag() {
        assert!(Tagged::<Gc<usize>, MyTag>::try_from(124879).is_err());
    }

    #[test]
    fn test_slice_of_tagged_pointers() {
        let _: Arena<Root![_]> = Arena::new(|_| {
            assert!(std::mem::size_of::<Gc<()>>() == std::mem::size_of::<Tagged<Gc<()>, MyTag>>());
            assert!(std::mem::size_of::<GcOpt<()>>() == std::mem::size_of::<Tagged<GcOpt<()>, MyTag>>());
        });
    }
}
