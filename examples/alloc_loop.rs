use sandpit::{
    gc::Gc,
    Arena, Root,
};

fn main() {
    let arena: Arena<Root![Gc<'_, usize>]> = Arena::new(|mu| Gc::new(mu, 69));
    let mut alloc_counter = 0usize;

    for _ in 0..100 {
        arena.mutate(|mu, _| loop {
            Gc::new(mu, 42);

            alloc_counter += std::mem::size_of::<usize>();

            if mu.yield_requested() {
                break;
            }
        });

        let config = arena.metrics();
        let arena_size_mb = config.arena_size as f64 / (1024 * 1024) as f64;
        let allocated_mb = alloc_counter as f64 / (1024 * 1024) as f64;

        assert!(5.0 > arena_size_mb);
        println!("Arena MB(s): {}", arena_size_mb);
        println!("Allocated MB(s): {}", allocated_mb);
    }
}
