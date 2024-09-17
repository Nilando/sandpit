use sandpit::{Gc, GcNullMut, GcMut, Arena, Mutator, Root, Trace, TraceLeaf, WriteBarrier, field};
use std::cell::{Cell, RefCell, RefMut, Ref};
use std::ptr::NonNull;

pub struct LinkedListIter<'gc, T: Trace> {
    next: Option<&'gc Node<'gc, T>>
}

impl<'gc, T: Trace> Iterator for LinkedListIter<'gc, T> {
    type Item = &'gc T;

    fn next(&mut self) -> Option<&'gc T> { 
        self.next.map(|node|  {
            self.next = node.get_next().map(|node_ptr| {
                // we need to use scoped deref in order for T: 'gc
                // regular deref loses that info
                node_ptr.scoped_deref()
            });

            node.get_val()
        })
    }
}

impl<'gc, T: Trace> LinkedListIter<'gc, T> {
    pub fn new(list: &LinkedList<'gc, T>) -> Self {
        let next = list.start.as_option().map(|gc_mut| {
            gc_mut.scoped_deref()
        });

        Self {
            next
        }
    }
}

#[derive(Trace, Clone)]
pub struct LinkedList<'gc, T: Trace> {
    start: GcNullMut<'gc, Node<'gc, T>>,
    end: GcNullMut<'gc, Node<'gc, T>>,
    len: Cell<usize>,
}

impl<'gc, T: Trace> LinkedList<'gc, T> {
    pub fn new(mu: &'gc Mutator<'gc>) -> Gc<'gc, Self> 
    {
        let new = Self {
            start: GcNullMut::new_null(mu),
            end: GcNullMut::new_null(mu),
            len: Cell::new(0),
        };

        Gc::new(mu, new)
    }

    pub fn iter(&self) -> LinkedListIter<'gc, T> {
        LinkedListIter::new(self)
    }

    fn as_gc(&self) -> Gc<'gc, Self> {
        // SAFETY: b/c the only way to construct a LinkedList is by allocating it in a Gc Arena
        // This unsafe could be avoided by changing the func signatures 
        // from `fn foo(&self)` into `fn foo(this: Gc<'gc, Self>)`
        unsafe { Gc::from_nonnull(NonNull::new_unchecked(self as *const Self as *mut Self)) }
    }

    pub fn push_back(&self, mu: &'gc Mutator<'gc>, val: T) {
        let gc_node = Gc::new(mu, Node::new(mu, val));

        if self.len.get() == 0 {
            self.init_push(mu, gc_node.into());
            return;
        }

        Node::set_prev(mu, gc_node, self.end.clone());
        Node::set_next(mu, self.end.as_option().unwrap().into(), gc_node.into());
        self.set_end(mu, gc_node.into());

        let new_len = self.len() + 1;
        self.len.set(new_len);
    }

    pub fn push_front(&self, mu: &'gc Mutator<'gc>, val: T) {
        let gc_node = Gc::new(mu, Node::new(mu, val));

        if self.len.get() == 0 {
            self.init_push(mu, gc_node.into());
            return;
        }

        Node::set_next(mu, gc_node, self.start.clone());
        Node::set_prev(mu, self.start.as_option().unwrap().into(), gc_node.into());
        self.set_start(mu, gc_node.into());

        let new_len = self.len() + 1;
        self.len.set(new_len);
    }

    pub fn pop_back(&self, mu: &'gc Mutator<'gc>, val: T) {
        todo!()
    }

    pub fn pop_front(&self, mu: &'gc Mutator<'gc>, val: T) {
        todo!()
    }

    pub fn delete(&self, mu: &'gc Mutator<'gc>, index: usize) -> &T {
        todo!()
    }

    pub fn swap(&self, mu: &'gc Mutator<'gc>, index: usize, val: T) {
        if index >= self.len() {
            panic!("atempted to swap index {} on len {} list", index, self.len());
        }

        let new_node = Gc::new(mu, Node::new(mu, val));

        // case 1: list has length 1
        // case 2: index == 0
        // case 3: index == (self.len() - 1)
        // case 4: all above are false

        if self.len() == 1 {
            self.set_end(mu, new_node.into());
            self.set_start(mu, new_node.into());
        } else if index == 0 {
            let next = self.node_at(1).unwrap();

            Node::set_prev(mu, next, new_node.into());
            Node::set_next(mu, new_node, next.into());

            self.set_start(mu, new_node.into())
        } else if index == (self.len() - 1){
            let prev = self.node_at(self.len() - 1).unwrap();

            Node::set_next(mu, prev, new_node.into());
            Node::set_prev(mu, new_node, prev.into());

            self.set_end(mu, new_node.into())
        } else {
            let next = self.node_at(index + 1).unwrap();
            let prev = self.node_at(index - 1).unwrap();

            Node::set_next(mu, prev, new_node.into());
            Node::set_prev(mu, new_node, prev.into());
            Node::set_prev(mu, next, new_node.into());
            Node::set_next(mu, new_node, next.into());
        }
    }

    pub fn insert(&self, mu: &'gc Mutator<'gc>, index: usize, val: T) {
        todo!()
    }

    pub fn at(&self, index: usize) -> Option<&T> {
        let mut iter = LinkedListIter::new(self);
        let mut result = iter.next();

        for _ in 0..index {
            result = iter.next();
        }

        result
    }

    fn node_at(&self, index: usize) -> Option<Gc<'gc, Node<'gc, T>>> {
        let mut node = self.start.clone();
        for _ in 0..index {
            match node.as_option() {
                None => return None,
                Some(gc_node) => {
                    node = gc_node.next.clone()
                }
            }
        }

        node.as_option().map(|gc_node| gc_node.into())
    }

    fn len(&self) -> usize {
        self.len.get()
    }

    fn init_push(&self, mu: &'gc Mutator<'gc>, gc_node: GcNullMut<'gc, Node<'gc, T>>) {
        self.set_start(mu, gc_node.clone());
        self.set_end(mu, gc_node);
        self.len.set(1);
        return;
    }

    fn set_start(&self, mu: &'gc Mutator<'gc>, new: GcNullMut<'gc, Node<'gc, T>>) {
        let gc_self = self.as_gc();

        mu.write_barrier(gc_self, |write_barrier| {
            field!(write_barrier, LinkedList, start).set(new);
        });
    }

    fn set_end(&self, mu: &'gc Mutator<'gc>, new: GcNullMut<'gc, Node<'gc, T>>) {
        let gc_self = self.as_gc();

        mu.write_barrier(gc_self, |write_barrier| {
            field!(write_barrier, LinkedList, end).set(new)
        });
    }
}

#[derive(Trace, Clone)]
pub struct Node<'gc, T: Trace> {
    prev:  GcNullMut<'gc, Node<'gc, T>>,
    next: GcNullMut<'gc, Node<'gc, T>>,
    val:   T,
}

impl<'gc, T: Trace> Node<'gc, T> {
    pub fn new(mu: &'gc Mutator<'gc>, val: T) -> Self 
    {
        Self {
            prev: GcNullMut::new_null(mu),
            next: GcNullMut::new_null(mu),
            val,
        }
    }

    pub fn set_prev(mu: &'gc Mutator<'gc>, this: Gc<'gc, Self>, new: GcNullMut<'gc, Self>) {
        mu.write_barrier(this, |write_barrier| {
            field!(write_barrier, Node, prev).set(new)
        });
    }

    pub fn set_next(mu: &'gc Mutator<'gc>, this: Gc<'gc, Self>, new: GcNullMut<'gc, Self>) {
        mu.write_barrier(this, |write_barrier| {
            field!(write_barrier, Node, next).set(new)
        });
    }

    pub fn get_val(&self) -> &T {
        &self.val
    }

    pub fn get_next(&self) -> Option<GcMut<'gc, Node<'gc, T>>> {
        self.next.as_option()
    }
    
    pub fn get_prev(&self) -> Option<GcMut<'gc, Node<'gc, T>>> {
        self.prev.as_option()
    }
}

// TESTS BELOW

#[test]
fn empty_list_arena() {
    let arena: Arena<Root![Gc<'_, LinkedList<'_, usize>>]> = Arena::new(|mu| {
        LinkedList::new(mu)
    });

    arena.mutate(|_mu, root| {
        assert!(root.len() == 0);
    });
}

#[test]
fn list_survives_major_collect() {
    env_logger::init();

    log::debug!("TESTING");
    let arena: Arena<Root![Gc<'_, LinkedList<'_, usize>>]> = Arena::new(|mu| {
        LinkedList::new(mu)
    });

    arena.mutate(|mu, root| {
        for i in 0..100 {
            root.push_back(mu, i);
        }

        assert!(root.len() == 100);
    });

    arena.major_collect();

    arena.mutate(|_mu, root| {
        for (idx, val) in root.iter().enumerate() {
            assert_eq!(idx, *val);
        }
    });
}

#[test]
fn list_survives_minor_collect() {
    let arena: Arena<Root![Gc<'_, LinkedList<'_, usize>>]> = Arena::new(|mu| {
        LinkedList::new(mu)
    });

    arena.mutate(|mu, root| {
        for i in 0..100 {
            root.push_back(mu, i);
        }

        assert!(root.len() == 100);
    });

    arena.minor_collect();

    arena.mutate(|_mu, root| {
        for (idx, val) in root.iter().enumerate() {
            assert_eq!(idx, *val);
        }
    });
}

#[test]
fn list_survives_multiple_collects() {
    let arena: Arena<Root![Gc<'_, LinkedList<'_, usize>>]> = Arena::new(|mu| {
        LinkedList::new(mu)
    });

    arena.mutate(|mu, root| {
        for i in 0..100 {
            root.push_back(mu, i);
        }

        assert!(root.len() == 100);
    });

    for _ in 0..10 {
        arena.minor_collect();
        arena.major_collect();
    }

    arena.mutate(|_mu, root| {
        for (idx, val) in root.iter().enumerate() {
            assert_eq!(idx, *val);
        }
    });
}

#[test]
fn linked_list_at() {
    let arena: Arena<Root![Gc<'_, LinkedList<'_, usize>>]> = Arena::new(|mu| {
        LinkedList::new(mu)
    });

    arena.mutate(|mu, root| {
        for i in 0..100 {
            root.push_back(mu, i);
        }

        for i in 0..100 {
            assert!(*(root.at(i).unwrap()) == i);
        }
    });
}

#[test]
fn list_swap_len_one() {
    let arena: Arena<Root![Gc<'_, LinkedList<'_, usize>>]> = Arena::new(|mu| {
        let list = LinkedList::new(mu);
        list.push_back(mu, 1);
        list
    });

    arena.mutate(|mu, root| {
        root.swap(mu, 0, 69);
    });

    arena.major_collect();

    arena.mutate(|_mu, root| {
        assert!(root.at(0) == Some(&69));
    });
}

#[test]
fn list_swap() {
    let arena: Arena<Root![Gc<'_, LinkedList<'_, usize>>]> = Arena::new(|mu| {
        let list = LinkedList::new(mu);
        for i in 0..5 {
            list.push_back(mu, i);
        }
        list
    });

    arena.major_collect();

    arena.mutate(|mu, root| {
        root.swap(mu, 3, 69);
    });

    arena.major_collect();

    arena.mutate(|_mu, root| {
        for i in 0..5 {
            if i != 3 {
                assert!(*(root.at(i).unwrap()) == i);
            } else {
                assert!(*(root.at(i).unwrap()) == 69);
            }
        }
    });
}

/*

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
