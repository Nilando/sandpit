use super::gc_ptr::GcPtr;
use super::trace::Trace;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::ptr::NonNull;
use super::tracer::Tracer;
use super::mutator::Mutator;
use std::mem::{size_of, align_of};
use std::alloc::Layout;
use super::error::GcError;

pub struct GcArrayMeta<T: Trace> {
    data: GcPtr<GcPtr<T>>,
    len: AtomicUsize,
    cap: AtomicUsize,
}

pub struct GcArray<T: Trace> {
    meta: GcPtr<GcArrayMeta<T>>,
}

// This is a shallow clone! (ie the underlying meta is the same)
// Maybe we should add a deep clone
impl<T: Trace> Clone for GcArray<T> {
    fn clone(&self) -> Self {
        Self {
            meta: self.meta.clone()
        }
    }
}

impl<T: Trace> GcArrayMeta<T> {
    pub fn new(data: GcPtr<GcPtr<T>>, len: usize, cap: usize) -> Self {
        Self {
            data,
            len: AtomicUsize::new(len),
            cap: AtomicUsize::new(cap),
        }
    }

    pub fn at(&self, idx: usize) -> GcPtr<T> {
        let len = self.len.load(Ordering::Relaxed);

        if len <= idx {
            panic!("Out of Bounds GcArray Index");
        }

        unsafe {
            let offset = self.data.as_ptr().add(idx);
            let gc_ptr = GcPtr::new(NonNull::new_unchecked(offset));

            (*gc_ptr).clone()
        }
    }

    pub fn set<M: Mutator>(this: &GcPtr<Self>, mutator: &M, idx: usize, item: GcPtr<T>) {
        this.internal_set(idx, item);
        this.trigger_write_barrier(mutator);
    }

    pub fn internal_set(&self, idx: usize, item: GcPtr<T>) {
        let len = self.len.load(Ordering::Relaxed);

        if len <= idx {
            panic!("Out of Bounds GcArray Index");
        }

        unsafe {
            let offset = self.data.as_ptr().add(idx);
            let gc_ptr = GcPtr::new(NonNull::new_unchecked(offset));

            (*gc_ptr).unsafe_set(item);
        }
    }

    pub fn push<M: Mutator>(this: &GcPtr<Self>, mutator: &M, obj: GcPtr<T>) {
        let len = this.len.load(Ordering::Relaxed);
        let cap = this.cap.load(Ordering::Relaxed);

        if len == cap {
            let new_cap = (cap / 2) + cap;
            unsafe {
                let layout = Layout::from_size_align_unchecked(size_of::<GcPtr<T>>() * new_cap, align_of::<GcPtr<T>>());
                let new_data = mutator.alloc_layout(layout).expect("failed to grow array");
                let new_meta = Self::new(new_data.cast(), len, new_cap);
                for i in 0..len {
                    new_meta.internal_set(i, this.at(i));
                }
                this.cap.store(new_cap, Ordering::Relaxed);
                this.data.unsafe_set(new_data.cast());
            }
        }

        this.len.fetch_add(1, Ordering::Relaxed);

        unsafe {
            let offset = this.data.as_ptr().add(len);
            let gc_ptr = GcPtr::new(NonNull::new_unchecked(offset));

            (*gc_ptr).unsafe_set(obj);
        }

        this.trigger_write_barrier(mutator);
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
    pub fn alloc<M: Mutator>(mutator: &M) -> Result<Self, GcError> {
        Self::alloc_with_capacity(mutator, 8)
    }

    pub fn alloc_with_capacity<M: Mutator>(mutator: &M, capacity: usize) -> Result<Self, GcError> {
        let layout = unsafe { Layout::from_size_align_unchecked(size_of::<GcPtr<T>>() * capacity, align_of::<GcPtr<T>>()) };
        let data_ptr = mutator.alloc_layout(layout)?;
        let casted_ptr = unsafe { data_ptr.cast() };
        let meta = GcArrayMeta::new(casted_ptr, 0, capacity);
        let meta_ptr = mutator.alloc(meta)?;

        Ok(GcArray::new(meta_ptr))
    }

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

    pub fn push<M: Mutator>(&self, mutator: &M, obj: GcPtr<T>) {
        GcArrayMeta::push(&self.meta, mutator, obj);
    }

    pub fn pop(&self) -> Option<GcPtr<T>> {
        self.meta.pop()
    }

    pub fn at(&self, idx: usize) -> GcPtr<T> {
        self.meta.at(idx)
    }

    pub fn set<M: Mutator>(&self, mutator: &M, idx: usize, item: GcPtr<T>) {
        GcArrayMeta::set(&self.meta, mutator, idx, item)
    }

    pub fn iter(&self) -> GcArrayIter<T> {
        GcArrayIter {
            pos: 0,
            array: self.clone()
        }
    }
}

pub struct GcArrayIter<T: Trace> {
    pos: usize,
    array: GcArray<T>
}

impl<T: Trace> Iterator for GcArrayIter<T> {
    type Item = GcPtr<T>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos > self.array.len() {
            self.pos += 1;

            Some(self.array.at(self.pos - 1))
        } else {
            None
        }
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
