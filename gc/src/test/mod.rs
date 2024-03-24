use crate::{Gc, MutatorRunner, Mutator, GcPtr, GcCell, Trace};

struct Node {
    left: GcCell<Option<GcPtr<Node>>>,
    right: GcCell<Option<GcPtr<Node>>>,
    val: usize,
}

unsafe impl Trace for Node {
    fn trace(&self) {}
}

struct TestMutator {
    root: Option<GcPtr<Node>>
}

impl TestMutator {
    pub fn new() -> Self {
        Self {
            root: None
        }
    }
}

impl MutatorRunner for TestMutator {
    type Root = Option<GcPtr<Node>>;

    fn run<'a, T: Mutator>(root: &mut Self::Root, mutator: &'a mut T) {
        let my_gc_ptr = mutator.alloc(69);
    }

    fn get_root(&mut self) -> &mut Self::Root {
        &mut self.root
    }
}

#[test]
fn create_mutator_runner() {
    let gc = Gc::new();
    let mut mutator = TestMutator::new();

    gc.mutate(&mut mutator);
}
