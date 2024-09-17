use super::allocator::Allocator;
use super::barrier::WriteBarrier;
use super::gc::Gc;
use super::header::Header;
use super::trace::{Trace, TraceJob, TracerController};

use std::alloc::Layout;
use std::cell::RefCell;
use std::ptr::{write, NonNull};
use std::sync::RwLockReadGuard;

pub struct Mutator<'gc> {
    allocator: Allocator,
    tracer_controller: &'gc TracerController,
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
    pub fn new(
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

    pub fn alloc<T: Trace>(&self, obj: T) -> Gc<'gc, T> {
        let header_layout = Layout::new::<Header>();
        let object_layout = Layout::new::<T>();
        let (alloc_layout, object_offset) = header_layout
            .extend(object_layout)
            .expect("Bad Alloc Layout");

        match self.allocator.alloc(alloc_layout) {
            Ok(ptr) => unsafe {
                let obj_ptr = ptr.as_ptr().add(object_offset).cast();

                write(obj_ptr, obj);
                write(ptr.as_ptr().cast(), Header::new());

                Gc::from_nonnull(NonNull::new_unchecked(obj_ptr))
            },
            Err(_) => panic!("failed to allocate"), // TODO: should this return an error?
        }
    }

    /*
        fn alloc_array<T: Trace + Default>(&'gc self, size: usize) -> GcArray<'gc, T> {
            let layout = Layout::from_size_align(size_of::<T>() * size, align_of::<T>()).unwrap();

            unsafe {
                let gc_raw = self.alloc_layout(layout);
            }
            todo!()
                /*
            let byte_ptr = ptr.as_ptr();

            for i in 0..layout.size() {
                    *byte_ptr.add(i) = 0;
            }
            */
        }
    */

    /*
    pub unsafe fn alloc_layout(&self, object_layout: Layout) -> NonNull<u8> {
        // TODO: the allocc lock needs to be reworked
        // doesn't really take into account the need to also stop the mutators
        // from access the write barrier... maybe copy this logic into the write barrier
        //
        if self.tracer_controller.is_alloc_lock() {
            drop(self.tracer_controller.get_alloc_lock());
        }

        let header_layout = Layout::new::<Header>();

        let (header_object_layout, object_offset) = header_layout.extend(object_layout).expect("Bad Alloc Layout");


        // TODO: check that the layout size isn't too large?


        // this needs to create a layout by adding to it the layout
        match self.allocator.alloc(header_object_layout) {
            Ok(ptr) => {
                unsafe {
                    todo!()
                    // Header::new(size)


                    NonNull::new_unchecked(
                        ptr.as_ptr().add(object_offset).cast()
                    )
                }
            },
            Err(_) => panic!("failed to allocate"), // TODO: should this return an error?
        }
    }
    */

    /// This flag will be set to true when a trace is near completion.
    /// The mutation callback should be exited if yield_requested returns true.
    pub fn yield_requested(&self) -> bool {
        self.tracer_controller.yield_flag()
    }

    pub fn retrace<T: Trace + 'gc>(&self, gc_into: impl TryInto<Gc<'gc, T>>) {
        let gc = gc_into.try_into().ok().unwrap(); // TODO: handle
        let trace_job = TraceJob::new(gc.as_nonnull());

        self.rescan.borrow_mut().push(trace_job);

        if self.rescan.borrow().len() >= self.tracer_controller.mutator_share_min {
            let work = self.rescan.take();
            self.tracer_controller.send_work(work);
        }
    }

    pub fn is_marked<T: Trace + 'gc>(&self, gc_into: impl Into<Gc<'gc, T>>) -> bool {
        let gc: Gc<'gc, T> = gc_into.into();
        let header = unsafe { &*Header::get_ptr(gc.as_nonnull()) };

        header.get_mark() == self.tracer_controller.get_current_mark()
    }

    pub fn write_barrier<F, T>(&self, gc_into: impl Into<Gc<'gc, T>>, f: F)
    where
        F: FnOnce(&WriteBarrier<T>),
        T: Trace + 'gc,
    {
        let gc: Gc<'gc, T> = gc_into.into();
        let barrier = WriteBarrier::new(&*gc);

        f(&barrier);

        if self.is_marked(gc) {
            self.retrace(gc);
        }
    }
}
