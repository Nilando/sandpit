use std::mem::ManuallyDrop;
use std::sync::atomic::{AtomicUsize, Ordering};
use super::gc::{Gc, GcOpt, GcPointer};

pub union Tagged<T: GcPointer> {
    ptr: ManuallyDrop<T>,
    raw: ManuallyDrop<AtomicUsize>,
} 

impl<T: GcPointer> From<T> for Tagged<T> {
    fn from(value: T) -> Self {
        Self {
            ptr: ManuallyDrop::new(value)
        }
    }
}

impl<T: GcPointer> Clone for Tagged<T> {
    fn clone(&self) -> Self {
        let raw =
            unsafe { AtomicUsize::new(self.raw.load(Ordering::Relaxed)) };

        Self {
            raw: ManuallyDrop::new(raw)
        }
    }
}

impl<A: GcPointer> TryFrom<usize> for Tagged<A> {
    type Error = ();

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        if (Self::TAG_MASK & value) == 0 {
            Err(())
        } else {
            Ok(Self {
                raw: ManuallyDrop::new(AtomicUsize::new(value))
            })
        }
    }
}

impl<A: GcPointer> Tagged<A> {
    const TAG_MASK: usize = A::POINTEE_ALIGNMENT - 1;

    pub fn is_ptr(&self) -> bool {
        self.get_tag() == 0
    }

    pub fn get_tag(&self) -> usize {
        unsafe {
            Self::TAG_MASK & self.raw.load(Ordering::Relaxed)
        }
    }

    pub fn get_ptr(&self) -> Option<A> {
        if self.is_ptr() {
            unsafe {
                return Some((*self.ptr).clone())
            }
        }

        None
    }

    pub fn get_raw(&self) -> usize {
        unsafe {
            self.raw.load(Ordering::Relaxed)
        }
    }
    pub fn set_raw(&self, value: usize) {
        assert!(Self::TAG_MASK & value != 0, "Invalid pointer tag");

        unsafe {
            self.set_raw_unchecked(value);
        }
    }

    pub unsafe fn set_raw_unchecked(&self, value: usize) {
        self.raw.store(value, Ordering::Relaxed);
    }

    pub unsafe fn set_ptr(&self, ptr: A) {
        self.ptr.set(ptr);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{gc::Gc, Arena, Root};

    #[test]
    fn test_into_pointer() {
        let _: Arena<Root![_]> = Arena::new(|mu| {
            let ptr = Gc::new(mu, 69);
            let tag = Tagged::from(ptr.clone());
            let untagged_pointer = tag.get_ptr().unwrap();

            assert!(ptr.as_thin() == untagged_pointer.as_thin());
        });
    }


    #[test]
    fn test_into_bytes() {
        let _: Arena<Root![_]> = Arena::new(|_| {
            let tagged_bytes = 1;
            let tag = Tagged::<Gc<usize>>::try_from(tagged_bytes).unwrap();
            let bytes = tag.get_raw();

            assert!(tagged_bytes == bytes);
        });
    }

    #[test]
    fn test_setting_tag() {
        let _: Arena<Root![_]> = Arena::new(|mu| {
            let ptr = GcOpt::new(mu, 69);
            let tagged_bytes = 7;
            let tag = Tagged::from(ptr);

            tag.set_raw(tagged_bytes);
            let bytes = tag.get_raw();

            assert!(tagged_bytes == bytes);
        });
    }

    #[test]
    fn test_invalid_tag() {
        assert!(Tagged::<Gc<()>>::try_from(0).is_err());
    }

    #[test]
    fn test_slice_of_tagged_pointers() {
        let _: Arena<Root![_]> = Arena::new(|_| {
            assert!(std::mem::size_of::<Gc<()>>() == std::mem::size_of::<Tagged<Gc<()>>>());
            assert!(std::mem::size_of::<GcOpt<()>>() == std::mem::size_of::<Tagged<GcOpt<()>>>());
        });
    }
}
