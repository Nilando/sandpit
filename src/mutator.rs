use super::allocator::Allocator;
use super::barrier::WriteBarrier;
use super::gc::{Gc};
use super::header::{GcHeader, SizedHeader, SliceHeader};
use super::trace::{Trace, TraceJob, TracerController};
use std::alloc::Layout;
use std::cell::RefCell;
use std::ptr::{write, NonNull};
use std::sync::RwLockReadGuard;

enum GcError {
    LayoutError,
}

pub struct Mutator<'gc> {
    tracer_controller: &'gc TracerController,
    allocator: Allocator,
    rescan: RefCell<Vec<TraceJob>>,
    _lock: RwLockReadGuard<'gc, ()>,
}

impl<'gc> Drop for Mutator<'gc> {
    fn drop(&mut self) {
        let work = self.rescan.take();
        self.tracer_controller.send_work(work);
    }
}

impl<'gc> Mutator<'gc> {
    pub(crate) fn new(
        allocator: Allocator,
        tracer_controller: &'gc TracerController,
        _lock: RwLockReadGuard<'gc, ()>,
    ) -> Self {
        Self {
            allocator,
            tracer_controller,
            rescan: RefCell::new(vec![]),
            _lock,
        }
    }

    pub fn alloc<T: Trace>(&self, value: T) -> Gc<'gc, T> {
        let header_layout = Layout::new::<SizedHeader>();
        let val_layout = Layout::new::<T>();
        let (alloc_layout_not_padded, val_offset) = header_layout
            .extend(val_layout)
            .expect("remove this expect");
        let alloc_layout = alloc_layout_not_padded.pad_to_align();

        unsafe {
            match self.allocator.alloc(alloc_layout) {
                // SAFETY: the alloc layout was extended to have capacity
                // for the header and object to be written into. 
                
                // Creating the Gc<T> from the obj_ptr is safe, b/c it upholds
                // the Gc invariant that a Gc<T> points to a T with a padded header.
                Ok(ptr) => {
                    let val_ptr = ptr.add(val_offset).cast();
                    let header_ptr = ptr.cast();

                    write(val_ptr, value);
                    write(header_ptr, SizedHeader::new());

                    Gc::from_ptr(val_ptr)
                },
                Err(_) => panic!("failed to allocate"), // TODO: should this return an error?
            }
        }
    }

    pub fn alloc_array<T: Trace + Clone>(&'gc self, value: T, len: usize) -> Gc<[T]> {
        self.alloc_array_from_fn(len, |_| value.clone())
    }

    pub fn alloc_array_from_slice<T: Trace + Clone>(&'gc self, slice: &[T]) -> Gc<[T]> {
        self.alloc_array_from_fn(slice.len(), |idx| {
            slice[idx].clone()
        })
    }

    pub fn alloc_array_from_fn<T, F>(&'gc self, len: usize, mut cb: F) -> Gc<[T]> 
    where
        T: Trace,
        F: FnMut(usize) -> T
    {
        let header_layout = Layout::new::<SliceHeader>();
        let slice_layout = Layout::array::<T>(len).expect("todo remove this expect");
        let (alloc_layout_not_padded, slice_offset) = header_layout
            .extend(slice_layout)
            .expect("todo remove this expect");
        let alloc_layout = alloc_layout_not_padded.pad_to_align();

        unsafe {
            match self.allocator.alloc(alloc_layout) {
                Ok(ptr) => {
                    let header_ptr = ptr.cast();
                    let slice_ptr: *mut T = ptr.add(slice_offset).cast();

                    for i in 0..len {
                        let item = cb(i);
                        write(slice_ptr.add(i), item);
                    }

                    let slice: &[T] = std::slice::from_raw_parts(slice_ptr, len);
                    write(header_ptr, SliceHeader::new(len));

                    Gc::from_ptr(slice as *const [T] )
                },
                Err(_) => panic!("failed to allocate"), // TODO: should this return an error?
            }
        }
    }

    /// This flag will be set to true when a trace is near completion.
    /// The mutation callback should be exited if yield_requested returns true.
    /// And this should be called by the mutator at a somewhat frequent and 
    /// constant interval. I wou
    pub fn gc_yield(&self) -> bool {
        if self.tracer_controller.yield_flag() {
            return true;
        } else {
            let _lock = self.tracer_controller.get_time_slice_lock();
            self.tracer_controller.yield_flag()
        }
    }

    pub fn retrace<P, V>(&self, gc_ptr: P) 
    where
        P: TryInto<Gc<'gc, V>>,
        V: Trace + 'gc
    {
        if let Ok(gc_ptr) = gc_ptr.try_into() {
            let ptr: NonNull<V> = NonNull::new(&*gc_ptr as *const V as *mut V).unwrap();

            let trace_job = TraceJob::new(ptr);

            self.rescan.borrow_mut().push(trace_job);

            if self.rescan.borrow().len() >= self.tracer_controller.mutator_share_min {
                let work = self.rescan.take();
                self.tracer_controller.send_work(work);
            }
        }
    }

    pub fn is_marked<P, V>(&self, gc_ptr: P) -> bool 
    where
        P: TryInto<Gc<'gc, V>>,
        V: Trace + 'gc
    {
        if let Ok(gc_ptr) = gc_ptr.try_into() {
            gc_ptr.get_header().get_mark() == self.tracer_controller.get_current_mark()
        } else {
            false
        }
    }

    pub fn write_barrier<P, V, F>(&self, gc_ptr: P, f: F)
    where
        P: TryInto<Gc<'gc, V>>,
        V: Trace + 'gc,
        F: FnOnce(&WriteBarrier<V>),
    {
        if let Ok(gc_ptr) = gc_ptr.try_into() {
            let barrier = WriteBarrier::new(&*gc_ptr);

            f(&barrier);

            if self.is_marked(gc_ptr) {
                self.retrace(gc_ptr);
            }
        }
    }
}
