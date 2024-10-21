use criterion::{
    criterion_group, 
    criterion_main, 
    Criterion, 
};

use sandpit::{Arena, gc::Gc, Root};

fn mutate_and_alloc(c: &mut Criterion) {
    let arena: Arena<Root![Gc<'_, usize>]> = Arena::new(|mu| {
        let root = Gc::new(mu, 69);

        root
    });

    arena.mutate(|mu, _| {
        c.bench_function("alloc usize", |b| {
                b.iter(|| Gc::new(mu, 123usize));
        });
    });

    c.bench_function("enter mutation", |b| {
        b.iter(|| {
            arena.mutate(|_mu, root| {
                assert!(**root == 69);
            });
        });
    });
}

criterion_group!(benches, mutate_and_alloc);
criterion_main!(benches);
