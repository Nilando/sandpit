use super::allocator::{Allocator, Allocate, GenerationalArena, Marker};
use super::gc::{Gc};
use super::trace::{Trace, TraceJob, TracerController};
use super::barrier::WriteBarrier;

use std::alloc::Layout;
use std::cell::RefCell;
use std::mem::{align_of, size_of};
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
        arena: &<Allocator as Allocate>::Arena,
        tracer_controller: &'gc TracerController,
        _lock: RwLockReadGuard<'gc, ()>,
    ) -> Self {
        let allocator = <Allocator as Allocate>::new(arena);

        Self {
            allocator,
            tracer_controller,
            rescan: RefCell::new(vec![]),
            _lock,
        }
    }

    pub fn alloc<T: Trace>(&self, obj: T) -> Gc<'gc, T> {
        let layout = Layout::new::<T>();

        unsafe { 
            let gc_raw = self.alloc_layout(layout).cast();

            write(gc_raw.as_ptr(), obj);

            Gc::from_nonnull(gc_raw)
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

    pub unsafe fn alloc_layout(&self, layout: Layout) -> NonNull<u8> {
        // TODO: the allocc lock needs to be reworked
        // doesn't really take into account the need to also stop the mutators
        // from access the write barrier... maybe copy this logic into the write barrier
        //
        if self.tracer_controller.is_alloc_lock() {
            drop(self.tracer_controller.get_alloc_lock());
        }

        match self.allocator.alloc(layout) {
            Ok(ptr) => ptr.cast(),
            Err(()) => panic!("failed to allocate"), // TODO: should this return an error?
        }
    }

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

        self.allocator.is_old(gc.as_nonnull())
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
