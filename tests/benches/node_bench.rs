use criterion::{criterion_group, criterion_main, Criterion};
use gc::Gc;
use tests::Node;

const TREE_SIZE: usize = 10_000;

fn full_collection() {
    let gc = Gc::build(|mutator| {
        let root = Node::alloc(mutator, 0).unwrap();

        Node::create_balanced_tree(&root, mutator, TREE_SIZE);

        root
    });

    for _ in 0..200 {
        gc.collect();
    }

    gc.mutate(|root, _| {
        let actual: Vec<usize> = Node::collect(root);
        let expected: Vec<usize> = (0..TREE_SIZE).collect();
        assert_eq!(actual, expected)
    });

}

fn eden_collection() {
    let gc = Gc::build(|mutator| {
        let root = Node::alloc(mutator, 0).unwrap();

        Node::create_balanced_tree(&root, mutator, TREE_SIZE);

        root
    });

    for _ in 0..200 {
        gc.eden_collect();
    }

    gc.mutate(|root, _| {
        let actual: Vec<usize> = Node::collect(root);
        let expected: Vec<usize> = (0..TREE_SIZE).collect();
        assert_eq!(actual, expected)
    });
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("full collection", |b| b.iter(|| full_collection()));
    c.bench_function("eden collection", |b| b.iter(|| eden_collection()));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
