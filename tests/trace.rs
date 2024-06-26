#[cfg(test)]
mod tests {
    use sandpit::{GcArena, Mutator};

    #[test]
    fn trace_option() {
        let gc = GcArena::build((), |mutator, _| {
            let inner: Option<usize> = Some(420);
            let outer = Some(mutator.alloc(inner).unwrap());
            mutator.alloc(outer).unwrap()
        });

        gc.major_collect();

        gc.mutate((), |root, _, _| {
            assert!(root.as_ref().unwrap().unwrap() == 420);
            root.set_null();
            assert!(root.is_null());
        })
    }
}
