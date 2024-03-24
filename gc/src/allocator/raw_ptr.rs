use std::ptr::NonNull;

#[derive(Debug)]
pub struct RawPtr<T: Sized> {
    ptr: NonNull<T>,
}

impl<T: Sized> RawPtr<T> {
    pub fn new(ptr: *const T) -> RawPtr<T> {
        RawPtr {
            ptr: unsafe { NonNull::new_unchecked(ptr as *mut T) },
        }
    }

    pub fn as_ptr(self) -> *const T {
        self.ptr.as_ptr()
    }

    pub fn as_word(self) -> usize {
        self.ptr.as_ptr() as usize
    }

    pub fn as_untyped(self) -> NonNull<()> {
        self.ptr.cast()
    }

    pub unsafe fn as_ref(&self) -> &T {
        self.ptr.as_ref()
    }

    // very unsafe!
    pub unsafe fn as_mut_ref(&mut self) -> &mut T {
        self.ptr.as_mut()
    }
}

impl<T: Sized> Clone for RawPtr<T> {
    fn clone(&self) -> RawPtr<T> {
        RawPtr { ptr: self.ptr }
    }
}

impl<T: Sized> Copy for RawPtr<T> {}

impl<T: Sized> PartialEq for RawPtr<T> {
    fn eq(&self, other: &RawPtr<T>) -> bool {
        self.ptr == other.ptr
    }
}
