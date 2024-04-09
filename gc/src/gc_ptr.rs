use std::cell::Cell;
use std::ops::Deref;
use std::ptr::NonNull;

use super::mutator::Mutator;
use super::trace::Trace;

pub struct GcPtr<T: Trace> {
    ptr: NonNull<T>,
}

impl<T: Trace> From<GcPtr<T>> for GcCellPtr<T> {
    fn from(ptr: GcPtr<T>) -> Self {
        Self {
            cell: Cell::new(Some(ptr)),
        }
    }
}

pub struct StrongGcPtr<'a, T: Trace> {
    ptr: &'a GcPtr<T>,
}

impl<'a, T: Trace> StrongGcPtr<'a, T> {
    fn downgrade(self) -> GcPtr<T> {
        *self.ptr
    }
}

impl<T: Trace> Deref for GcPtr<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { self.ptr.as_ref() }
    }
}

impl<T: Trace> GcPtr<T> {
    pub fn new(ptr: NonNull<T>) -> Self {
        Self { ptr }
    }

    pub fn as_ptr(&self) -> NonNull<T> {
        self.ptr
    }

    pub fn write_barrier<V: Trace, M: Mutator>(
        &self,
        mutator: &mut M,
        new_ptr: GcPtr<V>,
        callback: fn(&T) -> &GcCellPtr<V>,
    ) {
        let old_ptr = callback(self);
        unsafe { old_ptr.unsafe_set(mutator, new_ptr) } // safe b/c we call write_barrier after
        mutator.write_barrier(self.as_ptr());
    }
}

pub struct GcCellPtr<T: Trace> {
    cell: Cell<Option<GcPtr<T>>>,
}

impl<T: Trace> Deref for GcCellPtr<T> {
    type Target = Option<GcPtr<T>>;

    fn deref(&self) -> &Self::Target {
        unsafe { self.cell.as_ptr().as_ref().unwrap() }
    }
}

impl<T: Trace> GcCellPtr<T> {
    pub fn set<M: Mutator>(&self, scope: &M, new_ptr: StrongGcPtr<T>) {
        unsafe { self.unsafe_set(scope, new_ptr.downgrade()) }
    }

    pub fn set_null(&self) {
        self.cell.set(None)
    }

    pub fn new_null() -> Self {
        Self {
            cell: Cell::new(None),
        }
    }

    pub fn is_null(&self) -> bool {
        let opt_ref = unsafe { &*self.cell.as_ptr() as &Option<GcPtr<T>> };

        opt_ref.is_none()
    }

    pub fn is_some(&self) -> bool {
        !self.is_null()
    }

    pub fn as_ptr(&self) -> Option<NonNull<T>> {
        unsafe {
            self.cell.as_ptr().as_ref().unwrap().as_ref().map(|ptr| ptr.as_ptr())
        }
    }

    // This is unsafe b/c tracing may be happening concurrently at time of swap.
    // Therefore the caller of this function must ensure either the new_ptr
    // is NOT reachable from the root by the end of this mutation scope, otherwise
    // it must be ensured that new_ptr is scanned before the end of the mutation scope.
    //
    // If that invariant isn't upheld then new_ptr will be freed at the end of mutation scope,
    // and will become a dangling ptr.
    //
    // To ensure that the invariant is upheld don't use this function and instead
    // use the safe version which uses a strongPtr, or update the ptr through a
    // write barrier.
    pub unsafe fn unsafe_set<M: Mutator>(&self, _: &M, new_ptr: GcPtr<T>) {
        self.cell.set(Some(new_ptr))
    }
}

impl<T: Trace> Clone for GcCellPtr<T> {
    fn clone(&self) -> Self {
        let opt_ref = unsafe { &*self.cell.as_ptr() as &Option<GcPtr<T>> };

        Self {
            cell: Cell::new(*opt_ref),
        }
    }
}

impl<T: Trace> Clone for GcPtr<T> {
    fn clone(&self) -> Self {
        Self {
            ptr: self.ptr,
        }
    }
}

impl<T: Trace> Copy for GcPtr<T> {}
