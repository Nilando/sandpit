#[cfg(test)]
mod tests {
    use sandpit::{Gc, GcPtr, Mutator};

    #[test]
    fn trace_option() {
        let gc = Gc::build(|mutator| {
            let inner: Option<usize> = Some(420);
            let outer = Some(mutator.alloc(inner).unwrap());
            mutator.alloc(outer).unwrap()
        });

        gc.major_collect();

        gc.mutate(|root, mutator| {
            assert!(root.as_ref().unwrap().unwrap() == 420);
            let new_null = GcPtr::null();
            mutator.write_barrier(root.clone(), new_null, |this| this.as_ref().unwrap());
            assert!(root.as_ref().unwrap().is_null());
        })
    }
}