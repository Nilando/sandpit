use super::gc_ptr::GcPtr;
use super::trace::{Trace, Tracer};
use super::mutator::Mutator;
use std::cell::Cell;

const DEFAULT_CAP: usize = 8;
const VEC_GROW_RATIO: f64 = 0.5;

struct GcVecData<T: Trace> {
    data: GcPtr<T>, // WARNING! really GcPtr<[GcPtr<[T]]>>
}

impl<T: Trace> GcVecData<T> {
    fn cast_data(&self) -> *mut GcPtr<T> {
        self as *const Self as *mut GcPtr<T>
    }

    unsafe fn set(data: *mut GcPtr<T>, idx: usize, new: GcPtr<T>) {
        let ptr = data.add(idx);
        let old = &*ptr;

        old.swap(new);
    }

    unsafe fn at(data: *mut GcPtr<T>, idx: usize) -> GcPtr<T> {
        (*data.add(idx)).clone()
    }
}

unsafe impl<A: Trace> Trace for GcVecData<A> {
    // the tracing of individual items is handled in the trace of GcVec,
    // having empty impl here prevents an attempted trace of self.at[0]
    fn trace<T: Tracer>(&self, _tracer: &mut T) {}
}

pub struct GcVec<T: Trace> {
    cap: Cell<usize>,
    len: Cell<usize>,
    data: GcPtr<GcVecData<T>>,
}

impl<T: Trace> GcVec<T> {
    pub fn cast_data(&self) -> *mut GcPtr<T> {
        self.data.as_ptr() as *mut GcPtr<T>
    }

    pub fn alloc<M: Mutator>(mutator: &M) -> GcPtr<Self> {
        let data: GcPtr<GcVecData<T>> = mutator.alloc_array::<GcVecData<T>>(DEFAULT_CAP).unwrap();

        mutator.alloc(
            Self {
                len: Cell::new(0),
                cap: Cell::new(DEFAULT_CAP),
                data
            }
        ).unwrap()
    }

    pub fn push<M: Mutator>(this: GcPtr<Self>, mutator: &M, val: GcPtr<T>) {
        let len = this.len.get();
        let cap = this.cap.get();

        this.len.set(len + 1);

        if len == cap {
            let new_cap: usize = cap + (cap as f64 * VEC_GROW_RATIO).ceil() as usize;
            let new_data: GcPtr<GcVecData<T>> = mutator.alloc_array::<GcVecData<T>>(new_cap).unwrap();

            for i in 0..len {
                unsafe {
                    let copy = GcVecData::at(this.cast_data(), i);

                    GcVecData::set(new_data.as_ptr() as *mut GcPtr<T>, i, copy);
                }
            }

            // safe b/c we retrace right after
            unsafe { this.data.swap(new_data); }

            if mutator.is_marked(this.clone()) {
                // this will only mark the new data array
                // and will not trace all items again
                mutator.retrace(this.data.clone());
            }

            this.cap.set(new_cap);
        }

        Self::set(this, mutator, len, val);
    }

    pub fn set<M: Mutator>(this: GcPtr<Self>, mutator: &M, index: usize, val: GcPtr<T>) {
        let len = this.len.get();

        if index >= len {
            panic!("Out of Bounds GcVec access: index {}, on {} sized GcVec", index, len);
        }

        unsafe { GcVecData::set(this.cast_data(), index, val.clone()); }

        if mutator.is_marked(this) {
            mutator.retrace(val);
        }
    }

    pub fn pop(&self) -> Option<GcPtr<T>> {
        let len = self.len.get();

        if len == 0 {
            return None;
        }

        self.len.set(len - 1);

        unsafe { Some(GcVecData::at(self.cast_data(), len - 1)) }
    }

    pub fn at(&self, index: usize) -> GcPtr<T> {
        let len = self.len.get();

        if index >= len {
            panic!("Out of Bounds GcVec access: index {}, on {} sized GcVec", index, len);
        }

        unsafe { GcVecData::at(self.cast_data(), len - 1) }
    }

    pub fn clear(&self) {
        self.len.set(0);
    }

    pub fn len(&self) -> usize {
        self.len.get()
    }

    pub fn cap(&self) -> usize {
        self.cap.get()
    }
}

unsafe impl<T: Trace> Trace for GcVec<T> {
    fn trace<R: Tracer>(&self, tracer: &mut R) {
        self.data.trace(tracer);

        for i in 0..self.len.get() {
            unsafe { 
                let val: &T = &*GcVecData::at(self.cast_data(), i);
                // don't mark the val's they don't have headers, but still trace
                // their pointers.
                val.trace(tracer); }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Gc;

    #[test]
    fn alloc_vec() {
        Gc::build(|mu| {
            let vec = GcVec::<u8>::alloc(mu);

            assert!(vec.cap() == DEFAULT_CAP);
            assert!(vec.len() == 0);
        });
    }

    #[test]
    fn vec_push_pop() {
        let gc: Gc<GcPtr<GcVec<usize>>> = Gc::build(|mu| GcVec::<usize>::alloc(mu));

        gc.mutate(|root, mu| {
            for i in 0..1000 {
                let v = mu.alloc(i).unwrap();
                GcVec::push(root.clone(), mu, v);
            }
        });

        gc.major_collect();

        gc.mutate(|root, _| {
            for i in (0..1000).rev() {
                assert_eq!(*root.pop().unwrap(), i);
            }
            root.clear();
        });

        gc.major_collect();

        gc.mutate(|root, _| {
            assert!(root.pop().is_none())
        });
    }

    #[test]
    fn vec_set() {
        let gc: Gc<GcPtr<GcVec<usize>>> = Gc::build(|mu| GcVec::<usize>::alloc(mu));

        gc.mutate(|root, mu| {
            let v = mu.alloc(69).unwrap();
            GcVec::push(root.clone(), mu, v);
            assert!(*root.at(0) == 69);
            let v = mu.alloc(420).unwrap();
            GcVec::set(root.clone(), mu, 0, v);
            assert!(*root.at(0) == 420);
        });
    }
}
