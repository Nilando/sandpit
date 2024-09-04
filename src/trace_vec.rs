use std::sync::atomic::{AtomicUsize, Ordering};
use sandpit::{Trace, Tracer, Mutator, Gc};

const DEFAULT_CAP: usize = 8;
const VEC_GROW_RATIO: f64 = 0.5;

struct TraceVecData<'gc, T: Trace> {
    // if T is traceleaf, this will be similar to Gc<[T]>
    // if T is not TraceLeaf this will be similar to Gc<[Gc<T>]>
    data: Gc<'gc, T>, // WARNING! really Gc<[Gc<[T]]>>
}

impl<'gc, T: Trace> TraceVecData<'gc, T> {
    unsafe fn set(data: *mut Gc<T>, idx: usize, new: Gc<T>) {
        let ptr = data.add(idx);
        let old = &*ptr;

        old.set(new);
    }

    unsafe fn at(data: *mut Gc<T>, idx: usize) -> Gc<T> {
        (*data.add(idx)).clone()
    }
}

unsafe impl<'gc, A: Trace> Trace for TraceVecData<'gc, A> {
    // the tracing of individual items is handled in the trace of TraceVec,
    // having empty impl here prevents an atGcted trace of self.at[0]
    fn trace<T: Tracer>(&self, _tracer: &mut T) {}
}

pub struct TraceVec<'gc, T: Trace> {
    cap: AtomicUsize,
    len: AtomicUsize,
    data: Gc<'gc, TraceVecData<'gc, T>>,
}

impl<'gc, T: Trace> TraceVec<'gc, T> {
    pub fn cast_data(&self) -> *mut Gc<T> {
        unsafe { self.data.as_ptr() as *mut Gc<T> }
    }

    pub fn new<M: Mutator<'gc>>(m: &'gc M) -> Self {
        let data: Gc<'gc, TraceVecData<'gc, T>> = unsafe { Gc::from_nonnull(m, m.alloc_array::<TraceVecData<'gc, T>>(DEFAULT_CAP)) };

        Self {
            len: AtomicUsize::new(0),
            cap: AtomicUsize::new(DEFAULT_CAP),
            data,
        }
    }

    pub fn push<M: Mutator<'gc>>(&self, m: &'gc M, val: Gc<'gc, T>) {

        // if tracevecdata is marked and has space
        //  send val to be traced
        //
        //  if we alloc a new data and the old tracevecdata is marked
        //    we mark the new tracevecdata and send val to be traced
        
        let len = self.len();
        let cap = self.cap();

        if len == cap {
            let new_cap = 
                if cap == 0 {
                    DEFAULT_CAP
                } else {
                    cap + (cap as f64 * VEC_GROW_RATIO).ceil() as usize
                };

            let new_data: Gc<'gc, TraceVecData<'gc, T>> = unsafe { Gc::from_nonnull(m, m.alloc_array::<TraceVecData<'gc, T>>(new_cap)) };

            for i in 0..len {
                unsafe {
                    let copy = TraceVecData::at(self.cast_data(), i);

                    TraceVecData::set(new_data.as_ptr() as *mut Gc<T>, i, copy);
                }
            }

            let old_data = self.data.clone();
            // safe b/c we retrace right after
            unsafe {
                self.data.set(new_data);
            }

            if m.is_marked(old_data) {
                // this will only mark the new data array
                // and will not trace all items again
                m.retrace(self.data.clone());
            }

            self.cap.store(new_cap, Ordering::SeqCst);
        }

        unsafe {
            TraceVecData::set(self.cast_data(), len, val.clone());
        }

        self.len.fetch_add(1, Ordering::SeqCst);

        if m.is_marked(self.data.clone()) {
            m.retrace(val);
        }
    }

    pub fn set<M: Mutator<'gc>>(&self, mutator: &'gc M, index: usize, val: Gc<'gc, T>) {
        let len = self.len();

        if index >= len {
            panic!(
                "Out of Bounds TraceVec access: index {}, on {} sized TraceVec",
                index, len
            );
        }

        unsafe {
            TraceVecData::set(self.cast_data(), index, val.clone());
        }

        if mutator.is_marked(self.data.clone()) {
            mutator.retrace(val);
        }
    }

    pub fn pop(&self) -> Option<Gc<T>> {
        let len = self.len();

        if len == 0 {
            return None;
        }

        self.len.fetch_sub(1, Ordering::SeqCst);

        unsafe { Some(TraceVecData::at(self.cast_data(), len - 1)) }
    }

    pub fn at(&self, index: usize) -> Gc<T> {
        let len = self.len();

        if index >= len {
            panic!(
                "Out of Bounds TraceVec access: index {}, on {} sized TraceVec",
                index, len
            );
        }

        unsafe { TraceVecData::at(self.cast_data(), index) }
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

unsafe impl<'gc, T: Trace> Trace for TraceVec<'gc, T> {
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
    use sandpit::{Arena, Root};

    #[test]
    fn alloc_vec() {
        let _arena: Arena<Root![TraceVec<'_, u8>]> = Arena::new(|m| {
            let vec = TraceVec::<u8>::new(m);

            assert!(vec.cap() == DEFAULT_CAP);
            assert!(vec.len() == 0);
            vec
        });
    }

    #[test]
    fn vec_push_pop() {
        let arena: Arena<Root![TraceVec<'_, usize>]> = Arena::new(|m| TraceVec::<usize>::new(m));

        arena.mutate(|m, root| {
            for i in 0..10_000 {
                let v = Gc::new(m, i);
                root.push(m, v);
            }
        });

        arena.major_collect();

        arena.mutate(|_, root| {
            for i in (0..10_000).rev() {
                assert_eq!(*root.pop().unwrap(), i);
            }

            root.clear();
        });

        arena.major_collect();

        arena.mutate(|_, root| assert!(root.pop().is_none()));
    }

    #[test]
    fn vec_set() {
        let arena: Arena<Root![TraceVec<'_, usize>]> = Arena::new(|m| TraceVec::<usize>::new(m));

        arena.mutate(|m, root| {
            let v = Gc::new(m, 69);
            root.push(m, v);
            assert!(*root.at(0) == 69);
            let v = Gc::new(m, 420);
            root.set(m, 0, v);
            assert!(*root.at(0) == 420);
        });
    }
}
