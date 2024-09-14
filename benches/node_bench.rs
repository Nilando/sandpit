use criterion::{criterion_group, criterion_main, Criterion};

#[path = "../tests/node.rs"]
mod node;

use node::Node;
use sandpit::{Arena, Root};

const TREE_LAYERS: usize = 20;

fn balanced_binary_trees() {
    /*
    let arena: Arena<Root![Node<'_>]> = Arena::new(|mu| Node::create_balanced_tree(mu, TREE_LAYERS));

    arena.major_collect();

    arena.mutate(|mu, _root| { 
        Node::create_balanced_tree(mu, TREE_LAYERS); 
    });

    arena.major_collect();

    arena.mutate(|_mu, root| {
        let actual: Vec<usize> = Node::collect(root);
        let expected: Vec<usize> = (0..(2usize.pow(TREE_LAYERS as u32 - 1))).collect();

        assert_eq!(actual, expected)
    });
    */
}


fn node_group(c: &mut Criterion) {
    c.bench_function("sync_trees", |b| b.iter(balanced_binary_trees));
}

criterion_group!(benches, node_group);
criterion_main!(benches);
