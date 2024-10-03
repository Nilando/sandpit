# Sandpit [![Tests](https://github.com/Nilando/sandpit/actions/workflows/rust.yml/badge.svg)](https://github.com/Nilando/sandpit/actions/workflows/rust.yml)
Sandpit exposes a safe API for multi-threaded, generational, trace and sweep garbage collection.

## Trace and Sweep Garbage Collection (GC)
Trace and sweep GC is a memory management technique used to reclaim unused memory in programs. It works by first performing a "trace" phase, where the GC starts from a set of root references (e.g., global variables or the execution stack) and recursively follows all reachable objects, marking them as live. In Sandpit the set of root references are declared on the `Arena<R>` where R represents the root type.
```rust
    // Create an arena with a single garbage collected Foo type as the root.
    let arena: Arena<Root![Gc<'_, Foo<'_>>]> = Arena::new(|mu| Gc::new(mu, Foo::new()));
```
In order for the tracers to be able to accurately mark all objects, objects allocated in the GC arena must implement the `Trace` trait. This trait can safely be derived by a macro which creates a method `trace` which recursively calls trace on all its inner values. 
```rust
#[derive(Trace)]
struct Foo {
    important_data: usize,
    bar: Bar // Bar must be Trace too!
}
```
In the subsequent "sweep" phase, the collector scans through memory, identifying unmarked objects as unreachable (garbage) and reclaiming their memory for future use. This method ensures that only actively used objects remain in memory, reducing fragmentation and memory leaks. 
```rust
    // Enter a mutation which has access to the root of the arena and a mutator.
    arena.mutate(|mutator, root| {
        // we can allocate a garbage collected usize!
        let temp_garbage = Gc::new(mutator, 123);

        // Gc<T> can safely deref into &T
        assert!(*temp_garbage == 123);
    });

    // Garbage that is not reachable from the root will be freed!
    arena.major_collect();
    
    // Everything reachable from the root stays put.
    arena.mutate(|mutator, root| assert_eq!(**root, 69));
```

## The Mutation Context
A mutation context refers to the scenario in a program where the state of the heap (memory) is being modified, typically by altering object references or allocating new objects. This is significant for garbage collectors because mutations can create new references or break old ones, which must be tracked accurately to ensure that the garbage collection process does not mistakenly collect live objects or leave unreachable objects in memory. In the context of write barriers, the mutation context often triggers the need to record or account for such changes.
```rust
    // enter a mutation which has access to the root of the arena and a mutator
    arena.mutate(|mutator, root| {
        let garbage = Gc::new(mutator, 123); // we can allocate new things!

        // Or we can use a write barrier to update existing values
        root.write_barrier(mutator, |barrier| {
            // special care needs to be taken on how barriers are accessed...more on this later
            field!(root, Foo, bar).set(Bar::new());
        })
    });
```

## Safepoints
Safepoints are specific points during program execution where the program can safely pause to allow the garbage collector or other runtime system tasks (like thread suspension) to occur without corrupting the program’s state. At a safepoint, all threads in a program are either stopped or synchronized, ensuring that memory management tasks like garbage collection can be performed without the risk of the program altering the state of memory during the process. Safepoints are strategically placed, often at the beginning or end of method calls, loops, or certain instructions, to minimize performance impact while ensuring the program can be paused safely when needed.

In Sandpit, memory cannot be freed while a mutation is happening. The mutators will recieve a signal from the GC letting the user know that memory is ready to be freed, and that the mutation should exit.
```rust
    // enter a mutation which has access to the root of the arena and a mutator
    arena.mutate(|mutator, root| loop {
        // during this function it is likely the the GC will concurrently begin tracing!
        allocate_a_whole_bunch_of_garbage(mutator, root);

        if mutator.gc_yield() {
            // the mutator is signaling to us that memory is ready to be freed so we should leave the mutation context
            break;
        } else {
            // if the mutator isn't signaling for us to yield then we
            // are fine to go on allocating more garbage
        }
    });

    // memory can automatically be freed once we leave the mutation

    arena.major_collect(); // or manually freed
```

Because memory can be freed outisde of a mutation context, it is critical that references into the GC arena cannot be held outside of a mutation context. If they were, the GC may free their underlying memory, leading to dangling pointers. This is instead prevented by branding all GC values with a lifetime of `'gc`, which is that of the mutation context.
```rust
    let mut bad_thief: &usize = ... ;

    arena.mutate(|mutator, root| {
        let no_escaping = Gc::new(mutator, 69);

        // this will error due to lifetime scope of 'gc being that of the mutation
context
        thief = *no_escaping;
    });
```

## Write Barriers
Write barriers are mechanisms used in garbage collection to track changes to memory that could affect the state of the heap, particularly in generational or incremental garbage collectors. Since such collectors often divide the heap into different regions (e.g., young and old generations), write barriers help ensure that when objects in one region reference objects in another, these references are correctly noted. This ensures that the garbage collector can handle intergenerational pointers and other memory interactions without missing any references during its collection process, maintaining program correctness.

TODO

## Credits
This project was originally inspired from [Writing Interpreters in Rust: a guide](https://rust-hosted-langs.github.io/book/) by Peter Liniker. After initially following the guide,
I branched off to start working on Sandpit by closely following the code in Katherine West's [gc-arena crate](https://github.com/kyren/gc-arena). I would not have been able to compelte this project
without Peter and Katherine's work. I am deeply grateful for their well documented, and insightful open source contributions.

## License
Everything in this repository is licensed under either of:
- MIT license LICENSE-MIT or http://opensource.org/licenses/MIT
- Creative Commons CC0 1.0 Universal Public Domain Dedication LICENSE-CC0 or https://creativecommons.org/publicdomain/zero/1.0/ at your option.
