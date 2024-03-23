use crate::{Gc, MutatorRunner, Mutator};

struct TestMutator {

}

impl TestMutator {
    pub fn new() -> Self {
        Self {}
    }
}

impl MutatorRunner for TestMutator {
    fn run<'a, T: Mutator>(&mut self, scope: &'a T) {

    }
}

#[test]
fn foo() {
    let gc = Gc::new();
    let mut mutator = TestMutator::new();

    gc.mutate(&mut mutator);
}
