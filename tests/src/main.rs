use gc::{Gc, Mutator};
use tests::Node;

fn main() {
    let gc = Gc::build(|m| Node::alloc(m, 0).unwrap() );

    gc.start_monitor();

    for _ in 0..10 {
        gc.mutate(|root, m| {
            loop {
                Node::create_balanced_tree(root, m, 1_000);

                if m.yield_requested() {
                    break;
                }
            }
        });

        gc.mutate(|root, _| {
            let actual: Vec<usize> = Node::collect(root);
            let expected: Vec<usize> = (0..1_000).collect();
            assert_eq!(actual, expected)
        });

        //assert_eq!(*gc.metrics().get("prev_marked_objects").unwrap(), 1_000);
    }
}
