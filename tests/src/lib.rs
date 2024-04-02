#[cfg(test)]
mod tests {

    use gc::{Gc, GcCell, GcCellPtr, GcError, GcPtr, Mutator};
    use gc_derive::Trace;
    use std::ptr::NonNull;

    #[derive(Trace)]
    struct Node {
        left: GcCellPtr<Node>,
        right: GcCellPtr<Node>,
        val: GcCell<usize>,
    }

    impl Node {
        fn alloc<M: Mutator>(mutator: &M, val: usize) -> Result<GcPtr<Self>, GcError> {
            mutator.alloc(Node::new(val))
        }

        fn new(val: usize) -> Self {
            Self {
                left: GcCellPtr::new_null(),
                right: GcCellPtr::new_null(),
                val: GcCell::new(val),
            }
        }

        fn set_left<M: Mutator>(this: GcPtr<Node>, mutator: &M, new_left: GcPtr<Node>) {
            this.write_barrier(mutator, new_left, |this| &this.left);
        }

        fn set_right<M: Mutator>(this: GcPtr<Node>, mutator: &M, new_right: GcPtr<Node>) {
            this.write_barrier(mutator, new_right, |this| &this.right);
        }

        fn insert<M: Mutator>(this: GcPtr<Node>, mutator: &M, new_val: usize) {
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

        fn collect(this: GcPtr<Node>) -> Vec<usize> {
            let mut result = vec![];
            Self::traverse(this, &mut result);

            result
        }

        fn traverse(this: GcPtr<Node>, vals: &mut Vec<usize>) {
            if !this.right.is_null() {
                Self::traverse(*this.right.as_ref().unwrap(), vals)
            }
            vals.push(this.val.get());
            if !this.left.is_null() {
                Self::traverse(*this.left.as_ref().unwrap(), vals)
            }
        }

        fn find(this: GcPtr<Node>, val: usize) -> Option<GcPtr<Node>> {
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

    #[test]
    fn root_node() {
        let mut gc: Gc<Node> = Gc::build(|mutator| Node::alloc(mutator, 69).unwrap());

        gc.mutate(|root, _| {
            root.val.set(69);
            assert_eq!(root.val.get(), 69);

            root.val.set(420);
            assert_eq!(root.val.get(), 420);
        });
    }

    #[test]
    fn insert() {
        let mut gc: Gc<Node> = Gc::build(|mutator| Node::alloc(mutator, 0).unwrap());

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
        let mut gc: Gc<Node> = Gc::build(|mutator| Node::alloc(mutator, 0).unwrap());

        gc.mutate(|root, mutator| {
            for i in 1..1_000 {
                Node::insert(*root, mutator, i);
            }
        });

        gc.mutate(|root, _| {
            let node = Node::find(*root, 420).unwrap();

            assert_eq!(node.val.get(), 420);

            let node = Node::find(*root, 1_001);
            assert!(node.is_none());
        });
    }

    #[test]
    fn delete_and_collect() {
        let mut gc: Gc<Node> = Gc::build(|mutator| Node::alloc(mutator, 0).unwrap());

        // delete some stuff

        gc.collect();
    }
}
