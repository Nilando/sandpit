use sandpit::{
    field,
    gc::{Gc, GcMut, GcOpt},
    Arena, Mutator, Root, Trace, TraceLeaf, WriteBarrier,
};
use std::cell::{Cell, Ref, RefCell, RefMut};
use std::ptr::NonNull;
pub struct LinkedListIter<'gc, T: Trace> {
    next: Option<&'gc Node<'gc, T>>,
}

impl<'gc, T: Trace> Iterator for LinkedListIter<'gc, T> {
    type Item = &'gc T;

    fn next(&mut self) -> Option<&'gc T> {
        self.next.map(|node| {
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
        let next = list.start.as_option().map(|gc_mut| gc_mut.scoped_deref());

        Self { next }
    }
}

#[derive(Trace, Clone)]
pub struct LinkedList<'gc, T: Trace> {
    start: GcOpt<'gc, Node<'gc, T>>,
    end: GcOpt<'gc, Node<'gc, T>>,
    len: Cell<usize>,
}

impl<'gc, T: Trace> LinkedList<'gc, T> {
    pub fn new(mu: &'gc Mutator<'gc>) -> Gc<'gc, Self> {
        let new = Self {
            start: GcOpt::new_none(mu),
            end: GcOpt::new_none(mu),
            len: Cell::new(0),
        };

        Gc::new(mu, new)
    }

    pub fn iter(&self) -> LinkedListIter<'gc, T> {
        LinkedListIter::new(self)
    }

    pub fn push_back(this: Gc<'gc, Self>, mu: &'gc Mutator<'gc>, val: T) {
        let gc_node = Gc::new(mu, Node::new(mu, val));

        if this.len.get() == 0 {
            LinkedList::init_push(this, mu, gc_node.into());
            return;
        }

        Node::set_prev(mu, gc_node, this.end.clone());
        Node::set_next(mu, this.end.as_option().unwrap().into(), gc_node.into());
        LinkedList::set_end(this, mu, gc_node.into());

        let new_len = this.len() + 1;
        this.len.set(new_len);
    }

    pub fn push_front(this: Gc<'gc, Self>, mu: &'gc Mutator<'gc>, val: T) {
        let gc_node = Gc::new(mu, Node::new(mu, val));

        if this.len.get() == 0 {
            LinkedList::init_push(this, mu, gc_node.into());
            return;
        }

        Node::set_next(mu, gc_node, this.start.clone());
        Node::set_prev(mu, this.start.as_option().unwrap().into(), gc_node.into());
        LinkedList::set_start(this, mu, gc_node.into());

        let new_len = this.len() + 1;
        this.len.set(new_len);
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

    pub fn swap(this: Gc<'gc, Self>, mu: &'gc Mutator<'gc>, index: usize, val: T) {
        if index >= this.len() {
            panic!(
                "atempted to swap index {} on len {} list",
                index,
                this.len()
            );
        }

        let new_node = Gc::new(mu, Node::new(mu, val));

        // case 1: list has length 1
        // case 2: index == 0
        // case 3: index == (self.len() - 1)
        // case 4: all above are false

        if this.len() == 1 {
            LinkedList::set_end(this, mu, new_node.into());
            LinkedList::set_start(this, mu, new_node.into());
        } else if index == 0 {
            let next = this.node_at(1).unwrap();

            Node::set_prev(mu, next, new_node.into());
            Node::set_next(mu, new_node, next.into());

            LinkedList::set_start(this, mu, new_node.into())
        } else if index == (this.len() - 1) {
            let prev = this.node_at(this.len() - 1).unwrap();

            Node::set_next(mu, prev, new_node.into());
            Node::set_prev(mu, new_node, prev.into());

            LinkedList::set_end(this, mu, new_node.into())
        } else {
            let next = this.node_at(index + 1).unwrap();
            let prev = this.node_at(index - 1).unwrap();

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

    pub fn len(&self) -> usize {
        self.len.get()
    }

    fn node_at(&self, index: usize) -> Option<Gc<'gc, Node<'gc, T>>> {
        let mut node = self.start.clone();
        for _ in 0..index {
            match node.as_option() {
                None => return None,
                Some(gc_node) => node = gc_node.next.clone(),
            }
        }

        node.as_option().map(|gc_node| gc_node.into())
    }


    fn init_push(this: Gc<'gc, Self>, mu: &'gc Mutator<'gc>, gc_node: GcOpt<'gc, Node<'gc, T>>) {
        LinkedList::set_start(this, mu, gc_node.clone());
        LinkedList::set_end(this, mu, gc_node);
        this.len.set(1);
        return;
    }

    fn set_start(this: Gc<'gc, Self>, mu: &'gc Mutator<'gc>, new: GcOpt<'gc, Node<'gc, T>>) {
        this.write_barrier(mu, |write_barrier| {
            field!(write_barrier, LinkedList, start).set(new);
        });
    }

    fn set_end(this: Gc<'gc, Self>, mu: &'gc Mutator<'gc>, new: GcOpt<'gc, Node<'gc, T>>) {
        this.write_barrier(mu, |write_barrier| {
            field!(write_barrier, LinkedList, end).set(new)
        });
    }
}

#[derive(Trace, Clone)]
pub struct Node<'gc, T: Trace> {
    prev: GcOpt<'gc, Node<'gc, T>>,
    next: GcOpt<'gc, Node<'gc, T>>,
    val: T,
}

impl<'gc, T: Trace> Node<'gc, T> {
    pub fn new(mu: &'gc Mutator<'gc>, val: T) -> Self {
        Self {
            prev: GcOpt::new_none(mu),
            next: GcOpt::new_none(mu),
            val,
        }
    }

    pub fn set_prev(mu: &'gc Mutator<'gc>, this: Gc<'gc, Self>, new: GcOpt<'gc, Self>) {
        this.write_barrier(mu, |write_barrier| {
            field!(write_barrier, Node, prev).set(new)
        });
    }

    pub fn set_next(mu: &'gc Mutator<'gc>, this: Gc<'gc, Self>, new: GcOpt<'gc, Self>) {
        this.write_barrier(mu, |write_barrier| {
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
    let arena: Arena<Root![Gc<'_, LinkedList<'_, usize>>]> = Arena::new(|mu| LinkedList::new(mu));

    arena.mutate(|_mu, root| {
        assert!(root.len() == 0);
    });
}

#[test]
fn list_survives_major_collect() {
    let arena: Arena<Root![Gc<'_, LinkedList<'_, usize>>]> = Arena::new(|mu| LinkedList::new(mu));

    arena.mutate(|mu, root| {
        for i in 0..100 {
            LinkedList::push_back(root.clone(), mu, i);
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
    let arena: Arena<Root![Gc<'_, LinkedList<'_, usize>>]> = Arena::new(|mu| LinkedList::new(mu));

    arena.mutate(|mu, root| {
        for i in 0..100 {
            LinkedList::push_back(root.clone(), mu, i);
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
    let arena: Arena<Root![Gc<'_, LinkedList<'_, usize>>]> = Arena::new(|mu| LinkedList::new(mu));

    arena.mutate(|mu, root| {
        for i in 0..100 {
            LinkedList::push_back(root.clone(), mu, i);
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
    let arena: Arena<Root![Gc<'_, LinkedList<'_, usize>>]> = Arena::new(|mu| LinkedList::new(mu));

    arena.mutate(|mu, root| {
        for i in 0..100 {
            LinkedList::push_back(root.clone(), mu, i);
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
        LinkedList::push_back(list.clone(), mu, 1);
        list
    });

    arena.mutate(|mu, root| {
        LinkedList::swap(root.clone(), mu, 0, 69);
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
            LinkedList::push_back(list.clone(), mu, i);
        }
        list
    });

    arena.major_collect();

    arena.mutate(|mu, root| {
        LinkedList::swap(root.clone(), mu, 3, 69);
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

// this test uses a lot of memory
#[test]
#[ignore]
fn push_until_yield() {
    let arena: Arena<Root![Gc<'_, LinkedList<'_, usize>>]> = Arena::new(|mu| LinkedList::new(mu));
    arena.major_collect();

    let mut i = 0;

    arena.mutate(|mu, root| {
        while !mu.yield_requested() {
            LinkedList::push_back(root.clone(), mu, i);
            i += 1;
        }
    });

    arena.mutate(|_mu, root| {
        for k in 0..i {
            assert!(*(root.at(k).unwrap()) == k);
        }
    });
}

#[test]
fn objects_marked_metric() {
    let arena: Arena<Root![Gc<'_, LinkedList<'_, usize>>]> =
        Arena::new(|mu| LinkedList::new(mu));

    for i in 0..100 {
        arena.major_collect();
        arena.major_collect(); // TODO: why isn't old_objects count being set right
        assert_eq!(arena.metrics().old_objects_count, (i + 1));
        arena.mutate(|mu, root| {
            LinkedList::push_back(root.clone(), mu, i);
        });
    }
}
