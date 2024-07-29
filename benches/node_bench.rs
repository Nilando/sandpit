use criterion::{criterion_group, criterion_main, Criterion};

#[path = "../tests_old/node.rs"]
mod node;

use node::Node;
use sandpit::Gc;

const TREE_SIZE: usize = 10_000;

fn sync_trees() {
    let gc = Gc::build((), |mu, _| Node::alloc(mu, 0).unwrap());

    gc.stop_monitor();

    gc.mutate((), |root, mu, _| {
        for _ in 0..100 {
            Node::create_balanced_tree(root, mu, TREE_SIZE);
        }
    });

    gc.major_collect();

    gc.mutate((), |root, _, _| {
        let actual: Vec<usize> = Node::collect(root);
        let expected: Vec<usize> = (0..TREE_SIZE).collect();
        assert_eq!(actual, expected)
    });
}

fn concurrent_trees() {
    let gc = Gc::build((), |mu, _| Node::alloc(mu, 0).unwrap());

    gc.mutate((), |root, mu, _| {
        for _ in 0..100 {
            Node::create_balanced_tree(root, mu, TREE_SIZE);
        }
    });

    gc.mutate((), |root, _, _| {
        let actual: Vec<usize> = Node::collect(root);
        let expected: Vec<usize> = (0..TREE_SIZE).collect();
        assert_eq!(actual, expected)
    });
}

fn node_benchmark(c: &mut Criterion) {
    c.bench_function("sync_trees", |b| b.iter(sync_trees));
    c.bench_function("concurrent_trees", |b| b.iter(concurrent_trees));
}

criterion_group!(benches, node_benchmark);
criterion_main!(benches);
