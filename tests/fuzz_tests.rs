mod fuzz_lib;

use fuzz_lib::*;

// ============================================================================
// Basic Fuzzing Tests
// ============================================================================

#[test]
fn fuzz_never_collect() {
    println!("\n=== Testing: Never Collect ===");
    let config = FuzzConfig::never_collect();
    let stats = fuzz_gc_with_node_verification(config);
    stats.print();
    assert!(stats.total_allocations > 0);
    assert_eq!(stats.collections_performed, 1);
}

#[test]
fn fuzz_collect_always() {
    println!("\n=== Testing: Collect After Every Allocation ===");
    let config = FuzzConfig::collect_always();
    let stats = fuzz_gc_with_node_verification(config);
    stats.print();
    assert!(stats.total_allocations > 0);
    assert!(stats.collections_performed > config.iterations / 2);
}

#[test]
fn fuzz_collect_very_frequent() {
    println!("\n=== Testing: Very Frequent Collections ===");
    let config = FuzzConfig::collect_very_frequent();
    let stats = fuzz_gc_with_node_verification(config);
    stats.print();
    assert!(stats.total_allocations > 0);
    assert!(stats.collections_performed > 10);
}

#[test]
fn fuzz_collect_infrequent() {
    println!("\n=== Testing: Infrequent Collections ===");
    let config = FuzzConfig::collect_infrequent();
    let stats = fuzz_gc_with_node_verification(config);
    stats.print();
    assert!(stats.total_allocations > 0);
}

#[test]
fn fuzz_collect_random_low() {
    println!("\n=== Testing: Random Collections (Low Probability) ===");
    let config = FuzzConfig::collect_random(0.1);
    let stats = fuzz_gc_with_node_verification(config);
    stats.print();
    assert!(stats.total_allocations > 0);
}

#[test]
fn fuzz_collect_random_high() {
    println!("\n=== Testing: Random Collections (High Probability) ===");
    let config = FuzzConfig::collect_random(0.5);
    let stats = fuzz_gc_with_node_verification(config);
    stats.print();
    assert!(stats.total_allocations > 0);
}

#[test]
fn fuzz_collect_mixed() {
    println!("\n=== Testing: Mixed Major/Minor Collections ===");
    let config = FuzzConfig::collect_mixed();
    let stats = fuzz_gc_with_node_verification(config);
    stats.print();
    assert!(stats.total_allocations > 0);
    assert!(stats.major_collections > 0);
    assert!(stats.minor_collections > 0);
}

// ============================================================================
// Stress Tests
// ============================================================================

#[test]
fn fuzz_stress_test() {
    println!("\n=== Testing: Stress Test ===");
    let config = FuzzConfig::stress_test();
    let stats = fuzz_gc_with_node_verification(config);
    stats.print();
    assert!(stats.total_allocations > 1000);
}

#[test]
fn fuzz_stress_test_never_collect() {
    println!("\n=== Testing: Stress Test (No Collections) ===");
    let mut config = FuzzConfig::stress_test();
    config.collection_strategy = CollectionStrategy::Never;
    let stats = fuzz_gc_with_node_verification(config);
    stats.print();
    assert!(stats.total_allocations > 1000);
}

// ============================================================================
// Size and Alignment Variations
// ============================================================================

#[test]
fn fuzz_tiny_objects() {
    println!("\n=== Testing: Tiny Objects ===");
    let config = FuzzConfig {
        object_size_range: (1, 16),
        alignment_range: (0, 1),
        slice_length_range: (0, 10),
        ..FuzzConfig::default()
    };
    let stats = fuzz_gc_with_node_verification(config);
    stats.print();
    assert!(stats.total_allocations > 0);
}

#[test]
fn fuzz_large_objects() {
    println!("\n=== Testing: Large Objects ===");
    let config = FuzzConfig {
        object_size_range: (1024, 8192),
        slice_length_range: (1000, 10000),
        iterations: 50,
        allocs_per_iteration: (5, 20),
        ..FuzzConfig::default()
    };
    let stats = fuzz_gc_with_node_verification(config);
    stats.print();
    assert!(stats.total_allocations > 0);
}

#[test]
fn fuzz_super_large_objects() {
    println!("\n=== Testing: Large Objects ===");
    let config = FuzzConfig {
        object_size_range: (1024 * 16 + 1, 1024 * 20),
        slice_length_range: (1000, 10000),
        iterations: 10,
        allocs_per_iteration: (5, 20),
        ..FuzzConfig::default()
    };
    let stats = fuzz_gc_with_node_verification(config);
    stats.print();
    assert!(stats.total_allocations > 0);
}


#[test]
fn fuzz_max_alignment() {
    println!("\n=== Testing: Maximum Alignment ===");
    let config = FuzzConfig {
        alignment_range: (3, 4),
        collection_strategy: CollectionStrategy::VeryFrequent(10),
        ..FuzzConfig::default()
    };
    let stats = fuzz_gc_with_node_verification(config);
    stats.print();
    assert!(stats.total_allocations > 0);
}

// ============================================================================
// Slice-Heavy Tests
// ============================================================================

#[test]
fn fuzz_mostly_slices() {
    println!("\n=== Testing: Mostly Slices ===");
    let config = FuzzConfig {
        slice_probability: 0.8,
        string_probability: 0.0,
        slice_length_range: (0, 5000),
        ..FuzzConfig::default()
    };
    let stats = fuzz_gc_with_node_verification(config);
    stats.print();
    assert!(stats.total_allocations > 0);
}

#[test]
fn fuzz_empty_slices() {
    println!("\n=== Testing: Empty and Small Slices ===");
    let config = FuzzConfig {
        slice_probability: 0.9,
        string_probability: 0.0,
        slice_length_range: (0, 5),
        ..FuzzConfig::default()
    };
    let stats = fuzz_gc_with_node_verification(config);
    stats.print();
}

#[test]
fn fuzz_huge_slices() {
    println!("\n=== Testing: Huge Slices ===");
    let config = FuzzConfig {
        slice_probability: 0.8,
        string_probability: 0.0,
        slice_length_range: (5000, 20000),
        iterations: 20,
        allocs_per_iteration: (1, 5),
        collection_strategy: CollectionStrategy::VeryFrequent(2),
        ..FuzzConfig::default()
    };
    let stats = fuzz_gc_with_node_verification(config);
    stats.print();
}

// ============================================================================
// String Tests
// ============================================================================

#[test]
fn fuzz_mostly_strings() {
    println!("\n=== Testing: Mostly Strings ===");
    let config = FuzzConfig {
        string_probability: 0.7,
        slice_probability: 0.1,
        object_size_range: (0, 500),
        ..FuzzConfig::default()
    };
    let stats = fuzz_gc_with_node_verification(config);
    stats.print();
    assert!(stats.total_allocations > 0);
}

#[test]
fn fuzz_empty_strings() {
    println!("\n=== Testing: Empty Strings ===");
    let config = FuzzConfig {
        string_probability: 1.0,
        slice_probability: 0.0,
        object_size_range: (0, 5),
        ..FuzzConfig::default()
    };
    let stats = fuzz_gc_with_node_verification(config);
    stats.print();
}

#[test]
fn fuzz_large_strings() {
    println!("\n=== Testing: Large Strings ===");
    let config = FuzzConfig {
        string_probability: 0.8,
        slice_probability: 0.0,
        object_size_range: (500, 2000),
        iterations: 50,
        allocs_per_iteration: (5, 15),
        ..FuzzConfig::default()
    };
    let stats = fuzz_gc_with_node_verification(config);
    stats.print();
}

#[test]
fn fuzz_balanced_workload() {
    println!("\n=== Testing: Balanced Workload ===");
    let config = FuzzConfig {
        slice_probability: 0.33,
        string_probability: 0.33,
        collection_strategy: CollectionStrategy::Mixed {
            major_every: 30,
            minor_every: 10,
        },
        ..FuzzConfig::default()
    };
    let stats = fuzz_gc_with_node_verification(config);
    stats.print();
}

#[test]
fn fuzz_variable_sizes() {
    println!("\n=== Testing: Variable Size Workload ===");
    let config = FuzzConfig {
        object_size_range: (1, 10000),
        slice_length_range: (0, 10000),
        allocs_per_iteration: (1, 200),
        collection_strategy: CollectionStrategy::Random(0.2),
        ..FuzzConfig::default()
    };
    let stats = fuzz_gc_with_node_verification(config);
    stats.print();
    assert!(stats.total_allocations > 0);
}

// ============================================================================
// Deterministic Tests (with seeds)
// ============================================================================

#[test]
fn fuzz_deterministic_1() {
    println!("\n=== Testing: Deterministic Run #1 ===");
    let config = FuzzConfig {
        seed: Some(12345),
        ..FuzzConfig::default()
    };
    let stats = fuzz_gc_with_node_verification(config);
    stats.print();
    assert!(stats.total_allocations > 0);
}

#[test]
fn fuzz_deterministic_2() {
    println!("\n=== Testing: Deterministic Run #2 ===");
    let config = FuzzConfig {
        seed: Some(67890),
        collection_strategy: CollectionStrategy::Random(0.3),
        ..FuzzConfig::default()
    };
    let stats = fuzz_gc_with_node_verification(config);
    stats.print();
    assert!(stats.total_allocations > 0);
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn fuzz_single_allocation() {
    println!("\n=== Testing: Single Allocation Per Iteration ===");
    let config = FuzzConfig {
        iterations: 100,
        allocs_per_iteration: (1, 1),
        collection_strategy: CollectionStrategy::VeryFrequent(5),
        ..FuzzConfig::default()
    };
    let stats = fuzz_gc_with_node_verification(config);
    stats.print();
    assert!(stats.total_allocations > 0);
}

#[test]
fn fuzz_many_small_iterations() {
    println!("\n=== Testing: Many Small Iterations ===");
    let config = FuzzConfig {
        iterations: 1000,
        allocs_per_iteration: (1, 5),
        object_size_range: (1, 32),
        slice_length_range: (0, 10),
        collection_strategy: CollectionStrategy::Infrequent(200),
        ..FuzzConfig::default()
    };
    let stats = fuzz_gc_with_node_verification(config);
    stats.print();
    assert!(stats.total_allocations > 0);
}

#[test]
fn fuzz_few_large_iterations() {
    println!("\n=== Testing: Few Large Iterations ===");
    let config = FuzzConfig {
        iterations: 10,
        allocs_per_iteration: (100, 500),
        collection_strategy: CollectionStrategy::AfterEveryAllocation,
        ..FuzzConfig::default()
    };
    let stats = fuzz_gc_with_node_verification(config);
    stats.print();
    assert!(stats.total_allocations > 0);
}

// ============================================================================
// Multi-threaded Stress Tests
// ============================================================================

#[cfg(feature = "multi_threaded")]
#[test]
fn fuzz_mt_stress_test_frequent_collect() {
    println!("\n=== Testing (MT): Stress Test with Frequent Collections ===");
    let mut config = FuzzConfig::stress_test();
    config.collection_strategy = CollectionStrategy::VeryFrequent(10);
    let stats = fuzz_gc_with_node_verification(config);
    stats.print();
    assert!(stats.total_allocations > 1000);
    assert!(stats.collections_performed > 50);
}

#[cfg(feature = "multi_threaded")]
#[test]
fn fuzz_mt_stress_test_random_collect() {
    println!("\n=== Testing (MT): Stress Test with Random Collections ===");
    let mut config = FuzzConfig::stress_test();
    config.collection_strategy = CollectionStrategy::Random(0.3);
    let stats = fuzz_gc_with_node_verification(config);
    stats.print();
    assert!(stats.total_allocations > 1000);
}

#[cfg(feature = "multi_threaded")]
#[test]
fn fuzz_mt_extreme_stress() {
    println!("\n=== Testing (MT): Extreme Stress Test ===");
    let config = FuzzConfig {
        iterations: 2000,
        allocs_per_iteration: (20, 100),
        object_size_range: (1, 8192),
        slice_length_range: (0, 10000),
        slice_probability: 0.4,
        string_probability: 0.3,
        collection_strategy: CollectionStrategy::Mixed {
            major_every: 100,
            minor_every: 20,
        },
        ..FuzzConfig::default()
    };
    let stats = fuzz_gc_with_node_verification(config);
    stats.print();
    assert!(stats.total_allocations > 10000);
}
