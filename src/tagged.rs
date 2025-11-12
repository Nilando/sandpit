use crate::Trace;
use core::marker::PhantomData;
use core::mem::ManuallyDrop;
use core::sync::atomic::{AtomicUsize, Ordering};

use super::gc::Gc;

// TODO: Add a TaggedUsize<T: Tag> type

pub unsafe trait Tag: Sized {
    const VARIANTS: usize;
    const MIN_ALIGNMENT: usize;

    fn into_usize(&self) -> usize;
    fn from_usize(tag: usize) -> Option<Self>;
    fn is_ptr(&self) -> bool;
    fn trace_tagged<'gc>(tagged_ptr: &Tagged<'gc, Self>, tracer: &mut crate::Tracer);
}

pub struct Tagged<'gc, T: Tag> {
    raw: ManuallyDrop<AtomicUsize>,
    _tag_type: PhantomData<&'gc T>,
}

impl<'gc, T: Tag> Clone for Tagged<'gc, T> {
    fn clone(&self) -> Self {
        let raw = self.raw.load(Ordering::Relaxed);

        Self {
            raw: ManuallyDrop::new(AtomicUsize::new(raw)),
            _tag_type: PhantomData::<&'gc T>,
        }
    }
}

impl<'gc, T: Tag> TryFrom<usize> for Tagged<'gc, T> {
    type Error = ();

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        match T::from_usize(value & Self::TAG_MASK) {
            None => Err(()),
            Some(_) => Ok(Self {
                raw: ManuallyDrop::new(AtomicUsize::new(value)),
                _tag_type: PhantomData::<&'gc T>,
            }),
        }
    }
}

impl<'gc, T: Tag> Tagged<'gc, T> {
    const TAG_MASK: usize = T::MIN_ALIGNMENT - 1;

    fn const_assert() {
        const { assert!(T::VARIANTS <= T::MIN_ALIGNMENT) };
    }

    pub unsafe fn as_gc<A: Trace>(&self) -> Option<Gc<'gc, A>> {
        Self::const_assert();

        let raw = self.raw.load(Ordering::Acquire);

        if !Self::get_raw_tag(raw).is_ptr() {
            return None;
        }

        let tagged_value = Self::strip_tag(raw);

        Some(Gc::from_ptr(tagged_value as *const _))
    }

    pub unsafe fn from_ptr<A: Trace>(value: Gc<'gc, A>, tag: T) -> Self {
        Self::const_assert();

        assert!(tag.is_ptr(), "Tag must be a pointer variant");

        let tagged_value = Self::apply_tag(value.scoped_deref() as *const _ as usize, tag);

        Self {
            raw: ManuallyDrop::new(AtomicUsize::new(tagged_value)),
            _tag_type: PhantomData::<&'gc T>,
        }
    }

    pub fn from_raw(value: usize, tag: T) -> Self {
        Self::const_assert();
        assert!(!tag.is_ptr(), "Tag must be a non-pointer variant");

        let tagged_value = Self::apply_tag(value, tag);

        Self {
            raw: ManuallyDrop::new(AtomicUsize::new(tagged_value)),
            _tag_type: PhantomData::<&'gc T>,
        }
    }

    pub fn is_ptr(&self) -> bool {
        self.get_tag().is_ptr()
    }

    pub fn get_tag(&self) -> T {
        T::from_usize(self.get_raw() & Self::TAG_MASK).expect("Invalid tag value")
    }

    pub fn get_raw_tag(raw: usize) -> T {
        T::from_usize(raw & Self::TAG_MASK).expect("Invalid tag value")
    }

    pub fn get_stripped_raw(&self) -> usize {
        Self::strip_tag(self.raw.load(Ordering::Relaxed))
    }

    pub fn get_raw(&self) -> usize {
        self.raw.load(Ordering::Relaxed)
    }

    pub fn apply_tag(n: usize, tag: T) -> usize {
        Self::strip_tag(n) | tag.into_usize()
    }

    pub fn strip_tag(n: usize) -> usize {
        n & !Self::TAG_MASK
    }

    pub fn set_raw(&self, value: usize, tag: T) {
        Self::const_assert();

        assert!(!tag.is_ptr(), "Tag must be a non-pointer variant");

        let tagged_value = Self::apply_tag(value, tag);

        self.raw.store(tagged_value, Ordering::Relaxed);
    }

    pub unsafe fn set(&self, new_val: usize) {
        self.raw.store(new_val, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{gc::Gc, Arena, Root};
    use sandpit_derive::Tag;

    #[derive(Tag)]
    enum MyTag {
        #[ptr(usize)]
        Usize,
        #[ptr(isize)]
        Isize,
        #[ptr(Gc<'gc, usize>)]
        Gc,
        RawData,
    }

    #[test]
    fn test_pointer_variant() {
        let _: Arena<Root![_]> = Arena::new(|mu| {
            let ptr = Gc::new(mu, 69usize);
            let tagged = MyTag::from_usize(ptr.clone());

            assert!(tagged.is_ptr());
            assert!(matches!(tagged.get_tag(), MyTag::Usize));

            let extracted = MyTag::get_usize(tagged).unwrap();
            assert_eq!(*extracted, 69);
        });
    }

    #[test]
    fn test_gc_data_variant() {
        let _: Arena<Root![_]> = Arena::new(|mu| {
            let gc_ptr = Gc::new(mu, Gc::new(mu, 100usize));
            let tagged = MyTag::from_gc(gc_ptr);
            let extracted: Gc<Gc<usize>> = MyTag::get_gc(tagged).unwrap();

            assert_eq!(**extracted, 100);
        });
    }

    #[test]
    fn test_raw_data_variant() {
        let _: Arena<Root![_]> = Arena::new(|_| {
            let tagged = Tagged::from_raw(2048, MyTag::RawData);

            assert!(!tagged.is_ptr());
            assert!(matches!(tagged.get_tag(), MyTag::RawData));
            assert_eq!(tagged.get_stripped_raw(), 2048);
        });
    }

    #[test]
    fn test_wrong_extraction() {
        let _: Arena<Root![_]> = Arena::new(|mu| {
            let ptr = Gc::new(mu, 69usize);
            let tagged = MyTag::from_usize(ptr);

            // Should return None when trying to extract as wrong type
            assert!(MyTag::get_isize(tagged).is_none());
        });
    }

    #[test]
    fn test_size_preservation() {
        let _: Arena<Root![_]> = Arena::new(|_| {
            assert_eq!(
                core::mem::size_of::<*const u8>(),
                core::mem::size_of::<Tagged<MyTag>>()
            );
        });
    }
}
