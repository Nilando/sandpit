use std::ptr::NonNull;
use std::cell::Cell;

use super::mutator::Mutator;
use super::trace::Trace;

pub struct GcPtr<T: Trace> {
    ptr: NonNull<T>,
}

impl<T: Trace> From<GcPtr<T>> for GcCellPtr<T> {
    fn from(ptr: GcPtr<T>) -> Self {
        Self {
            cell: Cell::new(Some(ptr.clone()))
        }
    }
}

pub struct StrongGcPtr<'a, T: Trace> {
    ptr: &'a GcPtr<T>,
}

impl<'a, T: Trace> StrongGcPtr<'a, T> {
    fn downgrade(self) -> GcPtr<T> {
        self.ptr.clone()
    }
}

pub struct GcCellPtr<T: Trace> {
    cell: Cell<Option<GcPtr<T>>>,
}

impl<T: Trace> GcPtr<T> {
    pub fn new(ptr: NonNull<T>) -> Self {
        Self { ptr }
    }

    pub fn as_ref<'a, M: Mutator>(&self, _: &'a M) -> &'a T {
        unsafe { self.ptr.as_ref() }
    }

    fn as_ptr(&self) -> NonNull<T> {
        self.ptr.clone()
    }

    pub fn write_barrier<V: Trace, M: Mutator>(
        &self,
        scope: &mut M,
        new_ptr: GcPtr<V>,
        callback: fn(&T) -> &GcCellPtr<V>
    ) {
        let old_ref = self.as_ref(scope);
        let old_ptr = callback(old_ref);
        unsafe { old_ptr.unsafe_set(scope, new_ptr) }
        scope.write_barrier(self.as_ptr());
    }
}

impl<T: Trace> GcCellPtr<T> {
    pub fn as_ref<'a, M: Mutator>(&self, scope: &'a M) -> Option<&'a T> {
        let opt_ref = unsafe { &*self.cell.as_ptr() as &Option<GcPtr<T>> };

        opt_ref.as_ref().map(|gc_ptr| gc_ptr.as_ref(scope))
    }

    // TODO: make a version of this write_barrier which allows updating many ptrs
    // at once
    pub fn write_barrier<M: Mutator>(
        &self,
        scope: &M,
        new_ptr: GcPtr<T>,
        callback: fn(&T) -> &GcCellPtr<T>
    ) {
        if let Some(old_ref) = self.as_ref(scope) {
            let old_ptr = callback(old_ref);
            unsafe { old_ptr.unsafe_set(scope, new_ptr) }
        // TODO: send this ptr to unscanned if it is marked as old!
        } else {
            panic!("Tried to enter write barrier for a null GcCellPtr")
        }
    }

    pub fn set<M: Mutator>(&self, scope: &M, new_ptr: StrongGcPtr<T>) {
        unsafe { self.unsafe_set(scope, new_ptr.downgrade()) }
    }

    pub fn is_null(&self) -> bool {
        let opt_ref = unsafe { &*self.cell.as_ptr() as &Option<GcPtr<T>> };

        opt_ref.is_none()
    }

    pub fn is_some(&self) -> bool {
        !self.is_null()
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
            cell: Cell::new(opt_ref.clone())
        }
    }
}

impl<T: Trace> Clone for GcPtr<T> {
    fn clone(&self) -> Self {
        Self {
            ptr: self.ptr.clone().into()
        }
    }
}
