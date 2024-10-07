use criterion::{criterion_group, criterion_main, Criterion};

#[path = "../tests/linked_list.rs"]
mod linked_list;

use linked_list::LinkedList;
use sandpit::{gc::Gc, Arena, Root};

const LIST_SIZE: usize = 10_000;
const COLLECT_COUNT: usize = 20;

fn major_collect_list() {
    let arena: Arena<Root![Gc<'_, LinkedList<'_, usize>>]> = Arena::new(|mu| LinkedList::new(mu));

    arena.mutate(|mu, root| {
        for i in 0..LIST_SIZE {
            LinkedList::push_back(*root, mu, i);
        }

        assert!(root.len() == LIST_SIZE);
    });

    for _ in 0..COLLECT_COUNT {
        arena.major_collect();
    }
}

fn minor_collect_list() {
    let arena: Arena<Root![Gc<'_, LinkedList<'_, usize>>]> = Arena::new(|mu| LinkedList::new(mu));

    arena.mutate(|mu, root| {
        for i in 0..LIST_SIZE {
            LinkedList::push_back(*root, mu, i);
        }

        assert!(root.len() == LIST_SIZE);
    });

    for _ in 0..COLLECT_COUNT {
        arena.minor_collect();
    }
}

fn bench_group(c: &mut Criterion) {
    c.bench_function("major collect list", |b| b.iter(major_collect_list));
    c.bench_function("minor collect list", |b| b.iter(minor_collect_list));
}

criterion_group!(benches, bench_group);
criterion_main!(benches);
