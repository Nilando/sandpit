use gc::Gc;
use tests::Node;
use criterion::{criterion_group, criterion_main, Criterion};

fn bench() {
    let gc: Gc<Node> = Gc::build(|mutator| Node::alloc(mutator, 0).unwrap());

    for _ in 0..10 {
        gc.mutate(|root, mutator| {
            Node::kill_children(root);

            for _ in 0..10_000 {
                Node::insert_rand(root, mutator);
            }
        });

        //gc.collect();
    }
}


fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("node bench", |b| b.iter(|| bench()));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
