use super::gc_ptr::GcPtr;
use super::trace::Trace;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::ptr::NonNull;
use super::tracer::Tracer;


pub struct GcArrayMeta<T: Trace> {
    data: GcPtr<GcPtr<T>>,
    len: AtomicUsize,
    cap: AtomicUsize,
}

pub struct GcArray<T: Trace> {
    meta: GcPtr<GcArrayMeta<T>>,
}

impl<T: Trace> GcArrayMeta<T> {
    pub fn new(data: GcPtr<GcPtr<T>>, len: usize, cap: usize) -> Self {
        Self {
            data,
            len: AtomicUsize::new(len),
            cap: AtomicUsize::new(cap),
        }
    }

    pub fn push(&self, obj: GcPtr<T>) {
        let len = self.len.load(Ordering::Relaxed);
        let cap = self.cap.load(Ordering::Relaxed);

        if len == cap {
            // create a new & larger internal array
            // copy over the old data to the new array
            // push the new value
            // set the new data
            // increment the len
            // update the cap
            todo!()
        }

        self.len.fetch_add(1, Ordering::Relaxed);

        unsafe {
            let offset = self.data.as_ptr().add(len);
            let gc_ptr = GcPtr::new(NonNull::new_unchecked(offset));

            (*gc_ptr).unsafe_set(obj);
        }
    }

    pub fn pop(&self) -> Option<GcPtr<T>> {
        let len = self.len.load(Ordering::Relaxed);

        if len == 0 {
            None
        } else {
            let len = self.len.fetch_sub(1, Ordering::Relaxed);

            unsafe {
                let offset = self.data.as_ptr().add(len - 1);
                let gc_ptr = GcPtr::new(NonNull::new_unchecked(offset));

                Some((*gc_ptr).clone())
            }
        }
    }
}

impl<T: Trace> GcArray<T> {
    pub fn new(meta: GcPtr<GcArrayMeta<T>>) -> Self {
        Self {
            meta
        }
    }

    pub fn len(&self) -> usize {
        self.meta.len.load(Ordering::Relaxed)
    }

    pub fn cap(&self) -> usize {
        self.meta.cap.load(Ordering::Relaxed)
    }

    pub fn push(&self, obj: GcPtr<T>) {
        self.meta.push(obj);
    }

    pub fn pop(&self) -> Option<GcPtr<T>> {
        self.meta.pop()
    }
}

unsafe impl<T: Trace> Trace for GcArray<T> {
    fn trace<U: Tracer>(&self, tracer: &mut U) {
        self.meta.trace(tracer)
    }
}

unsafe impl<T: Trace> Trace for GcArrayMeta<T> {
    fn trace<U: Tracer>(&self, tracer: &mut U) {
        let len = self.len.load(Ordering::Relaxed);

        for i in 0..len {
            unsafe {
                let offset = self.data.as_ptr().add(i);
                let gc_ptr = GcPtr::new(NonNull::new_unchecked(offset));

                (*gc_ptr).trace(tracer)
            }
        }
    }
}
