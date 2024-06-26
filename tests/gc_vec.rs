use std::sync::atomic::{AtomicUsize, Ordering};
use sandpit::{Trace, Tracer, Mutator, Gc};

const DEFAULT_CAP: usize = 8;
const VEC_GROW_RATIO: f64 = 0.5;

struct GcVecData<T: Trace> {
    data: Gc<T>, // WARNING! really Gc<[Gc<[T]]>>
}

impl<T: Trace> GcVecData<T> {
    unsafe fn set(data: *mut Gc<T>, idx: usize, new: Gc<T>) {
        let ptr = data.add(idx);
        let old = &*ptr;

        old.swap(new);
    }

    unsafe fn at(data: *mut Gc<T>, idx: usize) -> Gc<T> {
        (*data.add(idx)).clone()
    }
}

unsafe impl<A: Trace> Trace for GcVecData<A> {
    // the tracing of individual items is handled in the trace of GcVec,
    // having empty impl here prevents an atGcted trace of self.at[0]
    fn trace<T: Tracer>(&self, _tracer: &mut T) {}
}

pub struct GcVec<T: Trace> {
    cap: AtomicUsize,
    len: AtomicUsize,
    data: Gc<GcVecData<T>>,
}

impl<T: Trace> GcVec<T> {
    pub fn cast_data(&self) -> *mut Gc<T> {
        self.data.as_ptr() as *mut Gc<T>
    }

    pub fn alloc<M: Mutator>(mutator: &M) -> Gc<Self> {
        let data: Gc<GcVecData<T>> = mutator.alloc_array::<GcVecData<T>>(DEFAULT_CAP).unwrap();
        let new = Self {
            len: AtomicUsize::new(0),
            cap: AtomicUsize::new(DEFAULT_CAP),
            data,
        };

        mutator.alloc(new).unwrap()
    }

    pub fn push<M: Mutator>(this: Gc<Self>, mutator: &M, val: Gc<T>) {
        let len = this.len();
        let cap = this.cap();

        if len == cap {
            let new_cap = 
                if cap == 0 {
                    DEFAULT_CAP
                } else {
                    cap + (cap as f64 * VEC_GROW_RATIO).ceil() as usize
                };

            let new_data: Gc<GcVecData<T>> =
                mutator.alloc_array::<GcVecData<T>>(new_cap).unwrap();

            for i in 0..len {
                unsafe {
                    let copy = GcVecData::at(this.cast_data(), i);

                    GcVecData::set(new_data.as_ptr() as *mut Gc<T>, i, copy);
                }
            }

            // safe b/c we retrace right after
            unsafe {
                this.data.swap(new_data);
            }

            if mutator.is_marked(this.clone()) {
                // this will only mark the new data array
                // and will not trace all items again
                mutator.retrace(this.data.clone());
            }

            this.cap.store(new_cap, Ordering::SeqCst);
        }

        unsafe {
            GcVecData::set(this.cast_data(), len, val.clone());
        }

        if mutator.is_marked(this.clone()) {
            mutator.retrace(val);
        }

        this.len.fetch_add(1, Ordering::SeqCst);
    }

    pub fn set<M: Mutator>(this: Gc<Self>, mutator: &M, index: usize, val: Gc<T>) {
        let len = this.len();

        if index >= len {
            panic!(
                "Out of Bounds GcVec access: index {}, on {} sized GcVec",
                index, len
            );
        }

        unsafe {
            GcVecData::set(this.cast_data(), index, val.clone());
        }

        if mutator.is_marked(this) {
            mutator.retrace(val);
        }
    }

    pub fn pop(&self) -> Option<Gc<T>> {
        let len = self.len();

        if len == 0 {
            return None;
        }

        self.len.fetch_sub(1, Ordering::SeqCst);

        unsafe { Some(GcVecData::at(self.cast_data(), len - 1)) }
    }

    pub fn at(&self, index: usize) -> Gc<T> {
        let len = self.len();

        if index >= len {
            panic!(
                "Out of Bounds GcVec access: index {}, on {} sized GcVec",
                index, len
            );
        }

        unsafe { GcVecData::at(self.cast_data(), index) }
    }

    pub fn clear(&self) {
        self.len.store(0, Ordering::SeqCst);
    }

    pub fn len(&self) -> usize {
        self.len.load(Ordering::SeqCst)
    }

    pub fn cap(&self) -> usize {
        self.cap.load(Ordering::SeqCst)
    }
}

unsafe impl<T: Trace> Trace for GcVec<T> {
    fn trace<R: Tracer>(&self, tracer: &mut R) {
        self.data.trace(tracer);

        for i in 0..self.len() {
            self.at(i).trace(tracer);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sandpit::GcArena;

    #[test]
    fn alloc_vec() {
        GcArena::build((), |mu, _| {
            let vec = GcVec::<u8>::alloc(mu);

            assert!(vec.cap() == DEFAULT_CAP);
            assert!(vec.len() == 0);
        });
    }

    #[test]
    fn vec_push_pop() {
        let gc: GcArena<Gc<GcVec<usize>>> = GcArena::build((), |mu, _| GcVec::<usize>::alloc(mu));

        gc.mutate((), |root, mu, _| {
            for i in 0..10_000 {
                let v = mu.alloc(i).unwrap();
                GcVec::push(root.clone(), mu, v);

                //for k in 0..i {
                    //assert_eq!(*root.at(k), k);
                //}
            }
        });

        gc.major_collect();

        gc.mutate((), |root, _, _| {
            for i in (0..10_000).rev() {
                assert_eq!(*root.pop().unwrap(), i);
            }
            root.clear();
        });

        gc.major_collect();

        gc.mutate((), |root, _, _| assert!(root.pop().is_none()));
    }

    #[test]
    fn vec_set() {
        let gc: GcArena<Gc<GcVec<usize>>> = GcArena::build((), |mu, _| GcVec::<usize>::alloc(mu));

        gc.mutate((), |root, mu, _| {
            let v = mu.alloc(69).unwrap();
            GcVec::push(root.clone(), mu, v);
            assert!(*root.at(0) == 69);
            let v = mu.alloc(420).unwrap();
            GcVec::set(root.clone(), mu, 0, v);
            assert!(*root.at(0) == 420);
        });
    }
}
