use criterion::{criterion_group, criterion_main, Criterion};
use gc::{Gc, Mutator};
use tests::Node;

const TREE_SIZE: usize = 10_000;

fn major_collection() {
    let gc = Gc::build(|m| {
        let root = Node::alloc(m, 0).unwrap();

        for _ in 0..10 {
            Node::create_balanced_tree(&root, m, TREE_SIZE);
        }

        root
    });

    gc.major_collect();

    gc.mutate(|root, _| {
        let actual: Vec<usize> = Node::collect(root);
        let expected: Vec<usize> = (0..TREE_SIZE).collect();
        assert_eq!(actual, expected)
    });

}

fn minor_collection() {
    let gc = Gc::build(|mutator| {
        let root = Node::alloc(mutator, 0).unwrap();

        Node::create_balanced_tree(&root, mutator, TREE_SIZE);

        root
    });

    gc.minor_collect();

    gc.mutate(|root, _| {
        let actual: Vec<usize> = Node::collect(root);
        let expected: Vec<usize> = (0..TREE_SIZE).collect();
        assert_eq!(actual, expected)
    });
}

fn sync_collection() {
    let gc = Gc::build(|m| Node::alloc(m, 0).unwrap() );

    gc.mutate(|root, m| {
        Node::create_balanced_tree(root, m, TREE_SIZE);
    });

    gc.major_collect();

    gc.mutate(|root, _| {
        let actual: Vec<usize> = Node::collect(root);
        let expected: Vec<usize> = (0..TREE_SIZE).collect();
        assert_eq!(actual, expected)
    });
}

fn concurrent_collection() {
    let gc = Gc::build(|m| Node::alloc(m, 0).unwrap() );

    gc.start_monitor();

    gc.mutate(|root, m| {
        for _ in 0..10 {
            Node::create_balanced_tree(root, m, TREE_SIZE);
        }
    });

    gc.major_collect();

    gc.mutate(|root, _| {
        let actual: Vec<usize> = Node::collect(root);
        let expected: Vec<usize> = (0..TREE_SIZE).collect();
        assert_eq!(actual, expected)
    });
}

fn node_benchmark(c: &mut Criterion) {
    c.bench_function("full collection", |b| b.iter(|| major_collection()));
    c.bench_function("eden collection", |b| b.iter(|| minor_collection()));
    c.bench_function("sync collection", |b| b.iter(|| sync_collection()));
    c.bench_function("concurrent collection", |b| b.iter(|| concurrent_collection()));
}

criterion_group!(benches, node_benchmark);
criterion_main!(benches);
