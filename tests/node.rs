use sandpit::{Gc, GcError, GcArena, Mutator, Trace};
use std::cell::Cell;

#[derive(Trace)]
pub struct Node {
    left: Gc<Node>,
    right: Gc<Node>,
    val: Cell<usize>,
}

impl Node {
    pub fn alloc<M: Mutator>(mu: &M, val: usize) -> Result<Gc<Self>, GcError> {
        mu.alloc(Node::new(val, mu))
    }

    pub fn new<M: Mutator>(val: usize, mu: &M) -> Self {
        Self {
            left: mu.new_null(),
            right: mu.new_null(),
            val: Cell::new(val),
        }
    }

    pub fn kill_children(this: &Gc<Node>) {
        this.left.set_null();
        this.right.set_null();
    }

    pub fn set_left<M: Mutator>(this: Gc<Node>, mu: &M, new_left: Gc<Node>) {
        unsafe { this.left.swap(new_left.clone()); }

        if mu.is_marked(this) && !mu.is_marked(new_left.clone()) {
            mu.retrace(new_left);
        }
    }

    pub fn set_right<M: Mutator>(this: Gc<Node>, mu: &M, new_right: Gc<Node>) {
        unsafe { this.right.swap(new_right.clone()); }

        if mu.is_marked(this) && !mu.is_marked(new_right.clone()) {
            mu.retrace(new_right);
        }
    }

    pub fn right_val(this: &Gc<Node>) -> usize {
        this.right.val.get()
    }

    pub fn left_val(this: &Gc<Node>) -> usize {
        this.left.val.get()
    }


    pub fn insert<M: Mutator>(this: &Gc<Node>, mu: &M, new_val: usize) -> Gc<Node> {
        if new_val > this.val.get() {
            if this.left.is_null() {
                // create a new node and set it as left
                let node_ptr = Node::alloc(mu, new_val).unwrap();
                Node::set_left(this.clone(), mu, node_ptr.clone());
                node_ptr
            } else {
                Node::insert(&this.left, mu, new_val)
            }
        } else if this.right.is_null() {
            let node_ptr = Node::alloc(mu, new_val).unwrap();
            Node::set_right(this.clone(), mu, node_ptr.clone());
            node_ptr
        } else {
            Node::insert(&this.right, mu, new_val)
        }
    }

    pub fn collect(this: &Gc<Node>) -> Vec<usize> {
        let mut result = vec![];
        Self::traverse(this, &mut result);

        result
    }

    // don't call this on a cyclic graph
    pub fn traverse(this: &Gc<Node>, vals: &mut Vec<usize>) {
        if !this.right.is_null() {
            Self::traverse(&this.right, vals)
        }

        vals.push(this.val.get());
        if !this.left.is_null() {
            Self::traverse(&this.left, vals)
        }
    }

    pub fn find(this: &Gc<Node>, val: usize) -> Option<Gc<Node>> {
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

    pub fn create_balanced_tree<M: Mutator>(this: &Gc<Node>, mu: &M, size: usize) {
        Node::kill_children(this);
        this.val.set(size / 2);
        Node::inner_create_balanced_tree(this, mu, 0, size)
    }

    fn inner_create_balanced_tree<M: Mutator>(
        this: &Gc<Node>,
        mu: &M,
        low: usize,
        high: usize,
    ) {
        if this.val.get() > low {
            let right_val = low + ((this.val.get() - low) / 2);
            let right = Node::alloc(mu, right_val).unwrap();
            Node::set_right(this.clone(), mu, right.clone());
            Node::inner_create_balanced_tree(&right, mu, low, this.val.get());
        }

        if (this.val.get() + 1) < high {
            let left_val = this.val.get() + ((high - this.val.get()) / 2);
            let left = Node::alloc(mu, left_val).unwrap();
            Node::set_left(this.clone(), mu, left.clone());
            Node::inner_create_balanced_tree(&left, mu, this.val.get() + 1, high);
        }
    }
}

// TESTS BELOW

#[test]
fn root_node() {
    let gc = GcArena::build((), |mu, _| Node::alloc(mu, 69).unwrap());

    gc.mutate((), |root, _, _| {
        root.val.set(69);
        assert_eq!(root.val.get(), 69);

        root.val.set(420);
        assert_eq!(root.val.get(), 420);
    });
}

#[test]
fn insert() {
    let gc = GcArena::build((), |mu, _| Node::alloc(mu, 0).unwrap());

    gc.mutate((), |root, mu, _| {
        for i in 1..1_000 {
            Node::insert(root, mu, i);
        }
    });

    gc.mutate((), |root, _, _| {
        let vals = Node::collect(root);
        let result: Vec<usize> = (0..1_000).collect();

        assert_eq!(vals, result);
    });
}

#[test]
fn find() {
    let gc = GcArena::build((), |mu, _| {
        let root = Node::alloc(mu, 0).unwrap();
        for _ in 0..1_000 {
            Node::insert(&root, mu, 123);
        }

        Node::insert(&root, mu, 420);

        root
    });

    gc.major_collect();

    gc.mutate((), |root, _, _| {
        let node = Node::find(root, 420).unwrap();

        assert_eq!(node.val.get(), 420);

        let node = Node::find(&node, 1_001);

        assert!(node.is_none());

        Node::kill_children(root);
    });
}

#[test]
fn multiple_collects() {
    let gc: GcArena<Gc<Node>> = GcArena::build((), |mu, _| {
        let root = Node::alloc(mu, 0).unwrap();
        for _ in 0..1_000 {
            Node::insert(&root, mu, 123);
        }
        Node::insert(&root, mu, 69);
        root
    });

    for i in 0..10 {
        if i % 2 == 0 {
            gc.minor_collect();
        } else {
            gc.major_collect();
        }
    }

    gc.mutate((), |root, _, _| {
        assert!(Node::find(root, 69).is_some());
    });
}

#[test]
fn monitor_requests_yield() {
    let gc = GcArena::build((), |mu, _| Node::alloc(mu, 0).unwrap());

    gc.mutate((), |root, mu, _| loop {
        Node::insert(root, mu, 0);

        if mu.yield_requested() {
            Node::kill_children(root);
            break;
        }
    });
}

#[test]
fn objects_marked_metric() {
    let gc = GcArena::build((), |mu, _| Node::alloc(mu, 0).unwrap());

    gc.mutate((), |root, mu, _| {
        for i in 0..99 {
            Node::insert(root, mu, i);
        }

        assert_eq!(Node::collect(root).len(), 100);
    });

    gc.major_collect();
    assert_eq!(gc.metrics().old_objects_count, 100);

    gc.mutate((), |root, _, _| {
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
    let gc = GcArena::build((), |mu, _| Node::alloc(mu, 0).unwrap());

    gc.mutate((), |root, mu, _| {
        Node::set_right(root.clone(), mu, root.clone());
        Node::set_left(root.clone(), mu, root.clone());

        assert!(Node::right_val(root) == 0);
        assert!(Node::left_val(root) == 0);
    });

    gc.major_collect();
}

#[test]
fn build_and_collect_balanced_tree_sync() {
    let gc = GcArena::build((), |mu, _| Node::alloc(mu, 0).unwrap());

    for _ in 0..2 {
        gc.major_collect();
        gc.minor_collect();
    }

    assert_eq!(gc.metrics().old_objects_count, 1);

    gc.mutate((), |root, mu, _| {
        Node::create_balanced_tree(root, mu, 100);
    });

    for _ in 0..2 {
        gc.major_collect();
        gc.minor_collect();
    }

    // at this major collect should set objects count to 0
    // then do a full trace of the tree... marking all
    assert_eq!(gc.metrics().old_objects_count, 100);

    gc.mutate((), |root, _, _| {
        let actual: Vec<usize> = Node::collect(root);
        let expected: Vec<usize> = (0..100).collect();
        assert_eq!(actual, expected)
    });
}

#[test]
fn build_and_collect_balanced_tree_concurrent() {
    let gc = GcArena::build((), |mu, _| Node::alloc(mu, 0).unwrap());

    gc.mutate((), |root, mu, _| {
        for _ in 0..100 {
            Node::create_balanced_tree(root, mu, 100);
        }
    });

    gc.major_collect();

    assert_eq!(gc.metrics().old_objects_count, 100);

    gc.mutate((), |root, _, _| {
        let actual: Vec<usize> = Node::collect(root);
        let expected: Vec<usize> = (0..100).collect();
        assert!(actual == expected)
    });
}

#[test]
fn multi_threaded_tree_building() {
    let gc: GcArena<()> = GcArena::build((), |_, _| ());

    fn tree_builder(gc: &GcArena<()>) {
        gc.mutate((), |_, mu, _| {
            let root = Node::alloc(mu, 0).unwrap();

            loop {
                Node::create_balanced_tree(&root, mu, 100);

                let actual: Vec<usize> = Node::collect(&root);
                let expected: Vec<usize> = (0..100).collect();
                assert!(actual == expected);

                if mu.yield_requested() {
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
    n1: Gc<Node>,
    n2: Gc<Node>,
    n3: Gc<Node>,
    n4: Gc<Node>,
}

#[test]
fn multi_threaded_root_mutation() {
    let gc = GcArena::build((), |mu, _| {
        let n1 = Node::alloc(mu, 0).unwrap();
        let n2 = Node::alloc(mu, 0).unwrap();
        let n3 = Node::alloc(mu, 0).unwrap();
        let n4 = Node::alloc(mu, 0).unwrap();

        Root { n1, n2, n3, n4 }
    });

    fn grow_forest<M: Mutator>(node: &Gc<Node>, mu: &M) {
        loop {
            Node::create_balanced_tree(node, mu, 100);

            if mu.yield_requested() {
                break;
            }
        }
    }

    std::thread::scope(|scope| {
        scope.spawn(|| {
            gc.mutate((), |root, mu, _| grow_forest(&root.n1, mu));
        });

        scope.spawn(|| {
            gc.mutate((), |root, mu, _| grow_forest(&root.n2, mu));
        });

        scope.spawn(|| {
            gc.mutate((), |root, mu, _| grow_forest(&root.n3, mu));
        });

        scope.spawn(|| {
            gc.mutate((), |root, mu, _| grow_forest(&root.n4, mu));
        });
    });
}
