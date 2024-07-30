use sandpit::{Gc, Arena, Mutator, Trace};
use std::cell::Cell;

#[derive(Trace, Clone)]
pub struct Node<'gc> {
    left: Option<Gc<'gc, Node<'gc>>>,
    right: Option<Gc<'gc, Node<'gc>>>,
    val: Cell<usize>,
}

/*
impl<'gc> Node<'gc> {
    pub fn new<M: Mutator<'gc>>(mu: &M, val: usize, left: Option<Gc<'gc, Node<'gc>>>, right: Option<Gc<'gc, Node<'gc>>>) -> Self {
        Self {
            left,
            right,
            val: Cell::new(val),
        }
    }

    pub fn get_val(&self) -> usize {
        self.val.get()
    }

    pub fn get_right(&self) -> Option<Gc<'gc, Node<'gc>>> {
        self.right.clone()
    }
    
    pub fn get_left(&self) -> Option<Gc<'gc, Node<'gc>>> {
        self.left.clone()
    }

    pub fn collect(&self) -> Vec<usize> {
        let mut result = vec![];

        self.traverse(&mut result);

        result
    }

    // don't call this on a cyclic graph
    pub fn traverse(&self, vals: &mut Vec<usize>) {
        if let Some(ref right) = self.right {
            right.traverse(vals)
        }

        vals.push(self.val.get());

        if let Some(ref left) = self.left {
            left.traverse(vals)
        }
    }

    pub fn find(&self, val: usize) -> Option<&Node> {
        let current_val = self.val.get();

        if current_val > val && self.right.is_some() {
            self.right.as_ref().unwrap().find(val)
        } else if current_val < val && self.left.is_some() {
            self.left.as_ref().unwrap().find(val)
        } else if current_val == val {
            Some(self)
        } else {
            None
        }
    }

    pub fn create_balanced_tree<M: Mutator<'gc>>(mu: &M, layers: usize) {

        let prev_layer = vec![];

        for layer in 0..layers {
            if prev_layer.is_empty() {

            }
        }
        let mut i = 0;

        for i < size {
            Gc::new(mu, i);
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
*/
