use gc::{Gc, GcCell, GcCellPtr, GcError, GcPtr, Mutator};
use gc_derive::Trace;
use std::ptr::NonNull;
use rand::Rng;

unsafe impl Send for Node {}
unsafe impl Sync for Node {}

#[derive(Trace)]
pub struct Node {
    left: GcCellPtr<Node>,
    right: GcCellPtr<Node>,
    val: GcCell<usize>,
}

impl Node {
    pub fn alloc<M: Mutator>(mutator: &M, val: usize) -> Result<GcPtr<Self>, GcError> {
        mutator.alloc(Node::new(val))
    }

    pub fn new(val: usize) -> Self {
        Self {
            left: GcCellPtr::new_null(),
            right: GcCellPtr::new_null(),
            val: GcCell::new(val),
        }
    }

    pub fn kill_children(this: GcPtr<Node>) {
        this.left.set_null();
        this.right.set_null();
    }

    pub fn set_left<M: Mutator>(this: GcPtr<Node>, mutator: &mut M, new_left: GcPtr<Node>) {
        this.write_barrier(mutator, new_left, |this| &this.left);
    }

    pub fn set_right<M: Mutator>(this: GcPtr<Node>, mutator: &mut M, new_right: GcPtr<Node>) {
        this.write_barrier(mutator, new_right, |this| &this.right);
    }

    pub fn insert_rand<M: Mutator>(this: GcPtr<Node>, mutator: &mut M) {
        let x = rand::thread_rng().gen_range(0..10_000_000);
        Node::insert(this, mutator, x);
    }

    pub fn insert<M: Mutator>(this: GcPtr<Node>, mutator: &mut M, new_val: usize) {
        if new_val > this.val.get() {
            if this.left.is_null() {
                // create a new node and set it as left
                let node_ptr = Node::alloc(mutator, new_val).unwrap();
                Node::set_left(this, mutator, node_ptr);
            } else {
                // recure on the left node
                let left = this.left.as_ref().unwrap();
                Node::insert(*left, mutator, new_val);
            }
        } else {
            if this.right.is_null() {
                // create a new node and set it as left
                let node_ptr = Node::alloc(mutator, new_val).unwrap();
                Node::set_right(this, mutator, node_ptr);
            } else {
                // insert to right
                let right = this.right.as_ref().unwrap();
                Node::insert(*right, mutator, new_val);
            }
        }
    }

    pub fn collect(this: GcPtr<Node>) -> Vec<usize> {
        let mut result = vec![];
        Self::traverse(this, &mut result);

        result
    }

    pub fn traverse(this: GcPtr<Node>, vals: &mut Vec<usize>) {
        if !this.right.is_null() {
            Self::traverse(*this.right.as_ref().unwrap(), vals)
        }
        vals.push(this.val.get());
        if !this.left.is_null() {
            Self::traverse(*this.left.as_ref().unwrap(), vals)
        }
    }

    pub fn find(this: GcPtr<Node>, val: usize) -> Option<GcPtr<Node>> {
        let current_val = this.val.get();

        if current_val > val && !this.right.is_null() {
            Self::find(*this.right.as_ref().unwrap(), val)
        } else if current_val < val && !this.left.is_null() {
            Self::find(*this.left.as_ref().unwrap(), val)
        } else if current_val == val {
            Some(this)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
                Node::insert(*root, mutator, i);
            }
        });

        gc.mutate(|root, _| {
            let vals = Node::collect(*root);
            let result: Vec<usize> = (0..1_000).collect();

            assert_eq!(vals, result);
        });
    }

    #[test]
    fn find() {
        let gc = Gc::build(|mutator| {
            let root = Node::alloc(mutator, 0).unwrap();
            for _ in 0..1_000 {
                Node::insert_rand(root, mutator);
            }

            Node::insert(root, mutator, 420);

            return root;
        });

        gc.collect();

        gc.mutate(|root, _| {
            let node = Node::find(*root, 420).unwrap();

            assert_eq!(node.val.get(), 420);

            let node = Node::find(node, 1_001);

            assert!(node.is_none());

            Node::kill_children(*root);
        });
    }

    #[test]
    fn multiple_collects() {
        let gc: Gc<GcPtr<Node>> = Gc::build(|mutator| {
            let root = Node::alloc(mutator, 0).unwrap();
            for _ in 0..1_000 {
                Node::insert_rand(root, mutator);
            }
            Node::insert(root, mutator, 69);
            return root;
        });


        std::thread::scope(|s| {
            for _ in 0..10 {
                s.spawn(|| {
                    for i in 0..10 {
                        if i % 2 == 0 {
                            gc.eden_collect();
                        } else {
                            gc.collect();
                        }
                    }
                });
            }
        });

        gc.mutate(|root, _mutator| {
            assert!(Node::find(*root, 69).is_some());
        });
    }

    #[test]
    fn monitor_requests_yield() {
        let gc = Gc::build(|mutator| Node::alloc(mutator, 0).unwrap());

        gc.start_monitor();

        gc.mutate(|root, mutator| {
            loop {
                Node::insert_rand(*root, mutator);

                if mutator.yield_requested() { 
                    Node::kill_children(*root);
                    break;
                }
            }
        });
    }
}
