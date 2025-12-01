use rand::prelude::*;
use sandpit::{Arena, Gc, GcOpt, Root, Trace};

#[derive(Debug, Clone, Copy)]
pub enum CollectionStrategy {
    Never,
    AfterEveryAllocation,
    VeryFrequent(usize),
    Infrequent(usize),
    Random(f64),
    Mixed { major_every: usize, minor_every: usize },
}

/// Configuration for the GC fuzzer
#[derive(Debug, Clone, Copy)]
pub struct FuzzConfig {
    /// Total number of allocation iterations to perform
    pub iterations: usize,

    /// Range for number of allocations per iteration (min, max)
    pub allocs_per_iteration: (usize, usize),

    /// Range for object sizes (min, max)
    pub object_size_range: (usize, usize),

    /// Range for object alignment (as power of 2: min, max)
    /// E.g., (0, 4) means alignments from 2^0=1 to 2^4=16
    pub alignment_range: (u32, u32),

    /// Range for slice lengths (min, max)
    pub slice_length_range: (usize, usize),

    /// Probability of allocating a slice vs a sized object (0.0 to 1.0)
    pub slice_probability: f64,

    /// Probability of allocating a string (0.0 to 1.0)
    pub string_probability: f64,

    /// Collection strategy to use
    pub collection_strategy: CollectionStrategy,

    /// Whether to verify allocations (slower but catches bugs)
    pub verify_allocations: bool,

    /// Random seed (None for random seed)
    pub seed: Option<u64>,
}

impl Default for FuzzConfig {
    fn default() -> Self {
        Self {
            iterations: 1000,
            allocs_per_iteration: (10, 100),
            object_size_range: (1, 1024),
            alignment_range: (0, 4),
            slice_length_range: (0, 1000),
            slice_probability: 0.3,
            string_probability: 0.2,
            collection_strategy: CollectionStrategy::VeryFrequent(10),
            verify_allocations: true,
            seed: None,
        }
    }
}

impl FuzzConfig {
    pub fn never_collect() -> Self {
        Self {
            collection_strategy: CollectionStrategy::Never,
            ..Default::default()
        }
    }

    pub fn collect_always() -> Self {
        Self {
            collection_strategy: CollectionStrategy::AfterEveryAllocation,
            iterations: 50,
            allocs_per_iteration: (5, 20),
            ..Default::default()
        }
    }

    pub fn collect_very_frequent() -> Self {
        Self {
            collection_strategy: CollectionStrategy::VeryFrequent(5),
            ..Default::default()
        }
    }

    pub fn collect_infrequent() -> Self {
        Self {
            collection_strategy: CollectionStrategy::Infrequent(100),
            iterations: 200,
            ..Default::default()
        }
    }

    pub fn collect_random(probability: f64) -> Self {
        Self {
            collection_strategy: CollectionStrategy::Random(probability),
            ..Default::default()
        }
    }

    pub fn collect_mixed() -> Self {
        Self {
            collection_strategy: CollectionStrategy::Mixed {
                major_every: 50,
                minor_every: 10,
            },
            ..Default::default()
        }
    }

    pub fn stress_test() -> Self {
        Self {
            iterations: 1000,
            allocs_per_iteration: (50, 200),
            object_size_range: (1, 4096),
            slice_length_range: (0, 10000),
            collection_strategy: CollectionStrategy::VeryFrequent(25),
            ..Default::default()
        }
    }
}

#[derive(Debug, Default)]
pub struct FuzzStats {
    pub total_allocations: usize,
    pub sized_allocations: usize,
    pub slice_allocations: usize,
    pub string_allocations: usize,
    pub collections_performed: usize,
    pub major_collections: usize,
    pub minor_collections: usize,
}

impl FuzzStats {
    pub fn print(&self) {
        println!("=== Fuzz Statistics ===");
        println!("Total allocations: {}", self.total_allocations);
        println!("  Sized: {}", self.sized_allocations);
        println!("  Slices: {}", self.slice_allocations);
        println!("  Strings: {}", self.string_allocations);
        println!("Collections performed: {}", self.collections_performed);
        println!("  Major: {}", self.major_collections);
        println!("  Minor: {}", self.minor_collections);
    }
}


fn should_collect(
    strategy: &CollectionStrategy,
    iteration: usize,
    alloc_count: usize,
    rng: &mut impl Rng,
) -> (bool, bool) {
    match strategy {
        CollectionStrategy::Never => (false, false),
        CollectionStrategy::AfterEveryAllocation => (true, iteration % 2 == 0),
        CollectionStrategy::VeryFrequent(n) => {
            if alloc_count % n == 0 {
                (true, alloc_count % (n * 2) == 0)
            } else {
                (false, false)
            }
        }
        CollectionStrategy::Infrequent(n) => {
            if alloc_count % n == 0 {
                (true, alloc_count % (n * 3) == 0)
            } else {
                (false, false)
            }
        }
        CollectionStrategy::Random(prob) => {
            if rng.gen::<f64>() < *prob {
                (true, rng.gen::<bool>())
            } else {
                (false, false)
            }
        }
        CollectionStrategy::Mixed { major_every, minor_every } => {
            if alloc_count % major_every == 0 {
                (true, true)
            } else if alloc_count % minor_every == 0 {
                (true, false)
            } else {
                (false, false)
            }
        }
    }
}

// Types for node verification fuzzing
use sandpit::GcVec;
use core::cell::Cell;

#[derive(Trace)]
enum NodeContent<'gc> {
    Sized(Gc<'gc, usize>),
    Slice(Gc<'gc, [usize]>),
    String(Gc<'gc, str>),
    VecValues(GcVec<'gc, Cell<usize>>),
    VecNodes(GcVec<'gc, Gc<'gc, VerifiedNode<'gc>>>),
}

#[derive(Trace)]
struct VerifiedNode<'gc> {
    id: Cell<usize>,
    content: NodeContent<'gc>,
    next: GcOpt<'gc, VerifiedNode<'gc>>,
}

// Helper: Create node content based on config probabilities
fn create_node_content<'gc, 'a>(
    mu: &'a sandpit::Mutator<'gc>,
    id: usize,
    config: &FuzzConfig,
    rng: &mut StdRng,
    stats: &mut FuzzStats,
) -> NodeContent<'gc>
where 'a: 'gc {
    let rand_val: f64 = rng.gen();

    if rand_val < config.string_probability {
        // String allocation
        let min_len = config.object_size_range.0;
        let max_len = config.object_size_range.1.min(1000).max(min_len);
        let string_len = if min_len == max_len {
            min_len
        } else {
            rng.gen_range(min_len..=max_len)
        };
        let string_content: String = (0..string_len)
            .map(|_| rng.sample(rand::distributions::Alphanumeric) as char)
            .collect();
        let gc_str = mu.alloc_str(&string_content);
        stats.string_allocations += 1;
        NodeContent::String(gc_str)
    } else if rand_val < config.string_probability + config.slice_probability {
        // Slice allocation
        let min_len = config.slice_length_range.0;
        let max_len = config.slice_length_range.1.min(50).max(min_len);
        let slice_len = if min_len == max_len {
            min_len
        } else {
            rng.gen_range(min_len..=max_len)
        };
        let slice = mu.alloc_array_from_fn(slice_len, |i| id * 1000 + i);
        stats.slice_allocations += 1;
        NodeContent::Slice(slice)
    } else {
        // Split remaining probability between sized, vec_values, vec_nodes
        let remaining_prob = 1.0 - config.string_probability - config.slice_probability;
        let adjusted_val = (rand_val - config.string_probability - config.slice_probability) / remaining_prob;

        if adjusted_val < 0.33 {
            // Sized allocation
            let data = Gc::new(mu, id * 777);
            stats.sized_allocations += 1;
            NodeContent::Sized(data)
        } else if adjusted_val < 0.66 {
            // GcVec of values
            let vec_len = rng.gen_range(0..=20);
            let vec = GcVec::new(mu);
            for i in 0..vec_len {
                vec.push(mu, Cell::new(id * 100 + i));
            }
            stats.sized_allocations += 1;
            NodeContent::VecValues(vec)
        } else {
            // GcVec of nodes (keep it small to avoid deep recursion)
            let vec_len = rng.gen_range(0..=5);
            let vec = GcVec::new(mu);
            // Create simple sized nodes to avoid infinite recursion
            for _ in 0..vec_len {
                let simple_node = Gc::new(mu, VerifiedNode {
                    id: Cell::new(id),
                    content: NodeContent::Sized(Gc::new(mu, id * 999)),
                    next: GcOpt::new_none(),
                });
                vec.push(mu, simple_node);
            }
            stats.sized_allocations += 1;
            NodeContent::VecNodes(vec)
        }
    }
}

// Helper: Verify a single node's content
fn verify_node_content<'gc>(node: &VerifiedNode<'gc>, id: usize, prefix: &str) {
    match &node.content {
        NodeContent::Sized(data_gc) => {
            let data = data_gc.scoped_deref();
            assert_eq!(*data, id * 777,
                "{}Node data corruption! Expected {}, got {} for id {}",
                prefix, id * 777, *data, id);
        }
        NodeContent::Slice(slice_gc) => {
            let slice = slice_gc.scoped_deref();
            for (idx, &slice_val) in slice.iter().enumerate() {
                let expected = id * 1000 + idx;
                assert_eq!(slice_val, expected,
                    "{}Node slice corruption! Expected {}, got {} for id {} at index {}",
                    prefix, expected, slice_val, id, idx);
            }
        }
        NodeContent::String(string_gc) => {
            let _string = string_gc.scoped_deref();
            // String content is random, so we just verify it's still accessible
        }
        NodeContent::VecValues(vec) => {
            for i in 0..vec.len() {
                if let Some(cell) = vec.get_idx(i) {
                    let val = cell.get();
                    let expected = id * 100 + i;
                    assert_eq!(val, expected,
                        "{}Vec value corruption! Expected {}, got {} for id {} at index {}",
                        prefix, expected, val, id, i);
                }
            }
        }
        NodeContent::VecNodes(vec) => {
            for i in 0..vec.len() {
                if let Some(node_gc) = vec.get_idx(i) {
                    let inner_node = node_gc.scoped_deref();
                    let _ = inner_node.id.get();
                    // Just verify we can access the node
                }
            }
        }
    }
}

// Helper: Verify all nodes in the list
fn verify_all_nodes<'gc>(
        head: GcOpt<'gc, VerifiedNode<'gc>>,
        expected_count: usize,
        prefix: &str,
    ) {
    let mut current = head;
    let mut verified_count = 0;

    while let Some(node_gc) = current.as_option() {
        let node = node_gc.scoped_deref();
        let id = node.id.get();
        verify_node_content(&node, id, prefix);
        verified_count += 1;
        current = node.next.clone();
    }

    assert_eq!(verified_count, expected_count,
        "{}Node count mismatch! Expected {}, found {} reachable nodes",
        prefix, expected_count, verified_count);
}

// Helper: Remove nodes from the front of the list
fn prune_nodes<'gc>(
        mu: &'gc sandpit::Mutator<'gc>,
        root: &'gc sandpit::InnerBarrier<GcOpt<'gc, VerifiedNode<'gc>>>,
        num_removes: usize,
        node_count: &mut usize,
    ) {
    for _ in 0..num_removes {
        if let Some(head_gc) = root.inner().as_option() {
            let head = head_gc.scoped_deref();
            let next = head.next.clone();
            root.write_barrier(mu, |barrier| {
                barrier.set(next);
            });
            *node_count -= 1;
        } else {
            break;
        }
    }
}

/// Fuzz test with nodes that verifies all allocated values remain valid
/// This tests graph structures (linked lists) with verification
pub fn fuzz_gc_with_node_verification(config: FuzzConfig) -> FuzzStats {
    use sandpit::InnerBarrier;

    let mut stats = FuzzStats::default();
    let mut rng: StdRng = if let Some(seed) = config.seed {
        StdRng::seed_from_u64(seed)
    } else {
        StdRng::from_entropy()
    };

    // Root points to the head of the linked list
    let arena: Arena<Root![InnerBarrier<GcOpt<'_, VerifiedNode<'_>>>]> = Arena::new(|mu| {
        InnerBarrier::new(mu, GcOpt::new_none())
    });

    let mut alloc_count = 0;
    let mut next_id = 0;
    let mut node_count = 0;

    for iteration in 0..config.iterations {
        arena.mutate(|mu, root| {
            let num_allocs = rng.gen_range(config.allocs_per_iteration.0..=config.allocs_per_iteration.1);

            // Add new nodes to the front of the list
            for _ in 0..num_allocs {
                let id = next_id;
                next_id += 1;

                let content = create_node_content(mu, id, &config, &mut rng, &mut stats);

                // Create new node pointing to current head
                let new_node = Gc::new(mu, VerifiedNode {
                    id: Cell::new(id),
                    content,
                    next: root.inner().clone(),
                });

                // Update head to point to new node
                root.write_barrier(mu, |barrier| {
                    barrier.set(GcOpt::from(new_node));
                });

                node_count += 1;
                stats.total_allocations += 1;
            }

            // Verify all nodes in the list
            if config.verify_allocations {
                verify_all_nodes(root.inner().clone(), node_count, "");
            }

            // Occasionally remove some nodes from the front to keep memory reasonable
            if node_count > 500 && rng.gen::<bool>() {
                let num_removes = rng.gen_range(1..=50);
                prune_nodes(mu, root, num_removes, &mut node_count);
            }

            alloc_count += 1;
        });

        let (should_collect, is_major) = should_collect(
            &config.collection_strategy,
            iteration,
            alloc_count,
            &mut rng,
        );

        if should_collect {
            if is_major {
                arena.major_collect();
                stats.major_collections += 1;
            } else {
                arena.minor_collect();
                stats.minor_collections += 1;
            }
            stats.collections_performed += 1;
        }
    }

    // Final verification pass - walk the entire list
    arena.mutate(|_mu, root| {
        verify_all_nodes(root.inner().clone(), node_count, "Final: ");
    });

    arena.major_collect();
    stats.collections_performed += 1;
    stats.major_collections += 1;

    stats
}
