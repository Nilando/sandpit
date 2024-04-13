use criterion::{criterion_group, criterion_main, Criterion};
use gc::Gc;
use tests::Node;

fn full_collection() {
    let gc = Gc::build(|mutator| {
        let root = Node::alloc(mutator, 0).unwrap();
        for _ in 0..100_000 {
            Node::insert_rand(root, mutator);
        }

        root
    });

    for _ in 0..200 {
        gc.collect();
    }
}

fn eden_collection() {
    let gc = Gc::build(|mutator| {
        let root = Node::alloc(mutator, 0).unwrap();
        for _ in 0..100_000 {
            Node::insert_rand(root, mutator);
        }

        root
    });

    for _ in 0..200 {
        gc.eden_collect();
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("full collection", |b| b.iter(|| full_collection()));
    c.bench_function("eden collection", |b| b.iter(|| eden_collection()));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
