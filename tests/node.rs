use sandpit::{Gc, GcError, GcPtr, Mutator, Trace, TraceLeaf};
use std::cell::Cell;

#[derive(Trace)]
pub struct Node {
    left: GcPtr<Node>,
    right: GcPtr<Node>,
    val: Cell<usize>,
}

impl Node {
    pub fn alloc<M: Mutator>(mutator: &M, val: usize) -> Result<GcPtr<Self>, GcError> {
        mutator.alloc(Node::new(val))
    }

    pub fn new(val: usize) -> Self {
        Self {
            left: GcPtr::null(),
            right: GcPtr::null(),
            val: Cell::new(val),
        }
    }

    pub fn kill_children(this: &GcPtr<Node>) {
        this.left.set_null();
        this.right.set_null();
    }

    pub fn set_left<M: Mutator>(this: &GcPtr<Node>, mutator: &M, new_left: GcPtr<Node>) {
        mutator.write_barrier(this.clone(), new_left, |this: &Node| &this.left);
    }

    pub fn right_val(this: &GcPtr<Node>) -> usize {
        this.right.val.get()
    }

    pub fn left_val(this: &GcPtr<Node>) -> usize {
        this.left.val.get()
    }

    pub fn set_right<M: Mutator>(this: &GcPtr<Node>, mutator: &M, new_right: GcPtr<Node>) {
        mutator.write_barrier(this.clone(), new_right, |this: &Node| &this.right);
    }

    pub fn insert<M: Mutator>(this: &GcPtr<Node>, mutator: &M, new_val: usize) -> GcPtr<Node> {
        if new_val > this.val.get() {
            if this.left.is_null() {
                // create a new node and set it as left
                let node_ptr = Node::alloc(mutator, new_val).unwrap();
                Node::set_left(this, mutator, node_ptr.clone());
                node_ptr
            } else {
                Node::insert(&this.left, mutator, new_val)
            }
        } else if this.right.is_null() {
            let node_ptr = Node::alloc(mutator, new_val).unwrap();
            Node::set_right(this, mutator, node_ptr.clone());
            node_ptr
        } else {
            Node::insert(&this.right, mutator, new_val)
        }
    }

    pub fn collect(this: &GcPtr<Node>) -> Vec<usize> {
        let mut result = vec![];
        Self::traverse(this, &mut result);

        result
    }

    // don't call this on a cyclic graph
    pub fn traverse(this: &GcPtr<Node>, vals: &mut Vec<usize>) {
        if !this.right.is_null() {
            Self::traverse(&this.right, vals)
        }

        vals.push(this.val.get());
        if !this.left.is_null() {
            Self::traverse(&this.left, vals)
        }
    }

    pub fn find(this: &GcPtr<Node>, val: usize) -> Option<GcPtr<Node>> {
        let current_val = this.val.get();

        if current_val > val && !this.right.is_null() {
            Self::find(&this.right, val)
        } else if current_val < val && !this.left.is_null() {
            Self::find(&this.left, val)
        } else if current_val == val {
            Some(this.clone())
        } else {
            None
        }
    }

    pub fn create_balanced_tree<M: Mutator>(this: &GcPtr<Node>, mutator: &M, size: usize) {
        Node::kill_children(this);
        this.val.set(size / 2);
        Node::inner_create_balanced_tree(this, mutator, 0, size)
    }

    fn inner_create_balanced_tree<M: Mutator>(
        this: &GcPtr<Node>,
        mutator: &M,
        low: usize,
        high: usize,
    ) {
        if this.val.get() > low {
            let right_val = low + ((this.val.get() - low) / 2);
            let right = Node::alloc(mutator, right_val).unwrap();
            Node::set_right(this, mutator, right.clone());
            Node::inner_create_balanced_tree(&right, mutator, low, this.val.get());
        }

        if (this.val.get() + 1) < high {
            let left_val = this.val.get() + ((high - this.val.get()) / 2);
            let left = Node::alloc(mutator, left_val).unwrap();
            Node::set_left(this, mutator, left.clone());
            Node::inner_create_balanced_tree(&left, mutator, this.val.get() + 1, high);
        }
    }
}

// TESTS BELOW

#[test]
fn root_node() {
    let gc = Gc::build(|mutator| Node::alloc(mutator, 69).unwrap());

    gc.mutate(|root, _| {
        root.val.set(69);
        assert_eq!(root.val.get(), 69);

        root.val.set(420);
        assert_eq!(root.val.get(), 420);
    });
}

#[test]
fn insert() {
    let gc = Gc::build(|mutator| Node::alloc(mutator, 0).unwrap());

    gc.mutate(|root, mutator| {
        for i in 1..1_000 {
            Node::insert(root, mutator, i);
        }
    });

    gc.mutate(|root, _| {
        let vals = Node::collect(root);
        let result: Vec<usize> = (0..1_000).collect();

        assert_eq!(vals, result);
    });
}

#[test]
fn find() {
    let gc = Gc::build(|mutator| {
        let root = Node::alloc(mutator, 0).unwrap();
        for _ in 0..1_000 {
            Node::insert(&root, mutator, 123);
        }

        Node::insert(&root, mutator, 420);

        root
    });

    gc.major_collect();

    gc.mutate(|root, _| {
        let node = Node::find(root, 420).unwrap();

        assert_eq!(node.val.get(), 420);

        let node = Node::find(&node, 1_001);

        assert!(node.is_none());

        Node::kill_children(root);
    });
}

#[test]
fn multiple_collects() {
    let gc: Gc<GcPtr<Node>> = Gc::build(|mutator| {
        let root = Node::alloc(mutator, 0).unwrap();
        for _ in 0..1_000 {
            Node::insert(&root, mutator, 123);
        }
        Node::insert(&root, mutator, 69);
        root
    });

    for i in 0..10 {
        if i % 2 == 0 {
            gc.minor_collect();
        } else {
            gc.major_collect();
        }
    }

    gc.mutate(|root, _mutator| {
        assert!(Node::find(root, 69).is_some());
    });
}

#[test]
fn monitor_requests_yield() {
    let gc = Gc::build(|mutator| Node::alloc(mutator, 0).unwrap());

    gc.mutate(|root, mutator| loop {
        Node::insert(root, mutator, 0);

        if mutator.yield_requested() {
            Node::kill_children(root);
            break;
        }
    });
}

#[test]
fn objects_marked_metric() {
    let gc = Gc::build(|mutator| Node::alloc(mutator, 0).unwrap());

    gc.mutate(|root, mutator| {
        for i in 0..99 {
            Node::insert(root, mutator, i);
        }

        assert_eq!(Node::collect(root).len(), 100);
    });

    gc.major_collect();
    assert_eq!(gc.metrics().old_objects_count, 100);

    gc.mutate(|root, _| {
        let node = Node::find(root, 48).unwrap();
        Node::kill_children(&node);

        assert_eq!(Node::collect(root).len(), 50);
        assert!(Node::find(root, 49).is_none());
        assert!(Node::find(root, 99).is_none());
    });

    gc.major_collect();

    assert_eq!(gc.metrics().old_objects_count, 50);
}

#[test]
fn cyclic_graph() {
    let gc = Gc::build(|mutator| Node::alloc(mutator, 0).unwrap());

    gc.mutate(|root, mutator| {
        Node::set_right(root, mutator, root.clone());
        Node::set_left(root, mutator, root.clone());

        assert!(Node::right_val(root) == 0);
        assert!(Node::left_val(root) == 0);
    });

    gc.major_collect();
}

#[test]
fn build_and_collect_balanced_tree_sync() {
    let gc = Gc::build(|m| Node::alloc(m, 0).unwrap());

    for _ in 0..2 {
        gc.major_collect();
        gc.minor_collect();
    }

    assert_eq!(gc.metrics().old_objects_count, 1);

    gc.mutate(|root, m| {
        Node::create_balanced_tree(root, m, 10_000);
    });

    for _ in 0..2 {
        gc.major_collect();
        gc.minor_collect();
    }

    // at this major collect should set objects count to 0
    // then do a full trace of the tree... marking all
    assert_eq!(gc.metrics().old_objects_count, 10_000);

    gc.mutate(|root, _| {
        let actual: Vec<usize> = Node::collect(root);
        let expected: Vec<usize> = (0..10_000).collect();
        assert_eq!(actual, expected)
    });
}

#[test]
fn build_and_collect_balanced_tree_concurrent() {
    let gc = Gc::build(|m| Node::alloc(m, 0).unwrap());

    gc.mutate(|root, m| {
        for _ in 0..1000 {
            Node::create_balanced_tree(root, m, 10_000);
        }
    });

    gc.major_collect();
    assert_eq!(gc.metrics().old_objects_count, 10_000);

    gc.mutate(|root, _| {
        let actual: Vec<usize> = Node::collect(root);
        let expected: Vec<usize> = (0..10_000).collect();
        assert!(actual == expected)
    });
}

#[test]
fn multi_threaded_tree_building() {
    let gc: Gc<()> = Gc::build(|_| ());

    fn tree_builder(gc: &Gc<()>) {
        gc.mutate(|_, m| {
            let root = Node::alloc(m, 0).unwrap();

            loop {
                Node::create_balanced_tree(&root, m, 1_000_000);

                let actual: Vec<usize> = Node::collect(&root);
                let expected: Vec<usize> = (0..1_000_000).collect();
                assert!(actual == expected);

                if m.yield_requested() {
                    break;
                }
            }
        });
    }

    std::thread::scope(|scope| {
        for _ in 0..100 {
            scope.spawn(|| tree_builder(&gc));
        }
    });
}

unsafe impl Send for Root {}
unsafe impl Sync for Root {}

#[derive(Trace)]
struct Root {
    n1: GcPtr<Node>,
    n2: GcPtr<Node>,
    n3: GcPtr<Node>,
    n4: GcPtr<Node>,
}

#[test]
fn multi_threaded_root_mutation() {
    let gc = Gc::build(|m| {
        let n1 = Node::alloc(m, 0).unwrap();
        let n2 = Node::alloc(m, 0).unwrap();
        let n3 = Node::alloc(m, 0).unwrap();
        let n4 = Node::alloc(m, 0).unwrap();

        Root { n1, n2, n3, n4 }
    });

    fn grow_forest<M: Mutator>(node: &GcPtr<Node>, m: &M) {
        loop {
            Node::create_balanced_tree(node, m, 10_000_000);

            if m.yield_requested() {
                break;
            }
        }
    }

    std::thread::scope(|scope| {
        scope.spawn(|| {
            gc.mutate(|root, m| grow_forest(&root.n1, m));
        });

        scope.spawn(|| {
            gc.mutate(|root, m| grow_forest(&root.n2, m));
        });

        scope.spawn(|| {
            gc.mutate(|root, m| grow_forest(&root.n3, m));
        });

        scope.spawn(|| {
            gc.mutate(|root, m| grow_forest(&root.n4, m));
        });
    });
}

#[test]
fn insert_and_extract_node_val() {
    let gc = Gc::build(|mutator| Node::alloc(mutator, 0).unwrap());

    gc.mutate(|root, mutator| { Node::insert(root, mutator, 333); } );

    let val = gc.extract(|root| Node::find(root, 333).unwrap().val.get());

    assert!(val == 333);

    gc.insert(420, |root, new_val| Node::find(root, 333).unwrap().val.set(new_val));

    let val = gc.extract(|root| Node::find(root, 420).unwrap().val.get());

    assert!(val == 420);
}


#[derive(TraceLeaf)]
struct TestLeaf<T: TraceLeaf> {
    foo: usize,
    bar: T,
}
