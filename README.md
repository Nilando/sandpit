# Sandpit [![Tests](https://github.com/Nilando/sandpit/actions/workflows/rust.yml/badge.svg)](https://github.com/Nilando/sandpit/actions/workflows/rust.yml)
Sandpit exposes a safe API for multi-threaded, generational, trace and sweep garbage collection.

This document provides a high level overview of Sandpit and garbage collection in general, for a more detailed explanation see the documentation.

## Contents
* [Trace And Sweep GC](#toc-trace-and-sweep-gc)
* [Mutation Context](#toc-mutation-context)
* [Safepoints](#toc-safepoints)
* [Write Barriers](#toc-write-barriers)
* [Credits](#toc-credits)
* [License](#toc-license)

<a name="toc-trace-and-sweep-gc"></a>
## Trace and Sweep Garbage Collection (GC)
Trace and sweep GC is a memory management technique used to reclaim unused memory in programs. It works by first performing a "trace" phase, where the GC starts from a set of root references (e.g., global variables or the execution stack) and recursively follows all reachable objects, marking them as live. In Sandpit the set of root references are declared on the `Arena<R>` where R represents the root type.
```rust
    // Create an arena with a single garbage collected Foo type as the root.
    let arena: Arena<Root![Gc<'_, VM<'_>>]> = Arena::new(|mutator| {
        Gc::new(mutator, VM::new(mutator)));
    })
```
In order for the tracers to be able to accurately mark all objects, objects allocated in the GC arena must implement the `Trace` trait. This trait can safely be derived by a macro which creates a method `trace` which recursively calls trace on all its inner values. There are 3 types that represent edges within the GC arena `Gc<'gc, T`, `GcMut<'gc, T>` and `GcOpt<'gc, T>`.
```rust
    #[derive(Trace)]
    enum Value {
        // There are 3 types of pointers to GC'ed values.
        A(Gc<'gc, A>), // Immutable pointer, essentially a &'gc T.
        B(GcMut<'gc, B>), // Mutable pointer, can be updated to point at something else via a write barrier.
        C(GcOpt<'gc, C>), // Optionally null pointer that is also mutable. Can be unwrapped into a GcMut.

        // All inner values must be trace, therfore types A, B, and T must impl Trace as well!
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
<a name="toc-mutation-context"></a>
## The Mutation Context
A mutation context refers to the scenario in a program where the state of the heap (memory) is being modified, typically by altering object references or allocating new objects. This is significant for garbage collectors because mutations can create new references or break old ones, which must be tracked accurately to ensure that the garbage collection process does not mistakenly collect live objects or leave unreachable objects in memory. In the context of write barriers, the mutation context often triggers the need to record or account for such changes.
```rust
    // enter a mutation context which has access to the root of the arena and a mutator
    arena.mutate(|mutator, root| {
        let garbage = Gc::new(mutator, 123); // we can allocate new things!

        // Or we can use a write barrier to update existing values
        root.write_barrier(mutator, |barrier| {
            // special care needs to be taken on how barriers are accessed...more on this later
            field!(root, Foo, bar).set(Bar::new());
        })
    });
```
<a name="toc-safepoints"></a>
## Safepoints
Safepoints are specific points during program execution where the program can safely pause to allow the garbage collector or other runtime system tasks (like thread suspension) to occur without corrupting the program’s state. 

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
    let mut thief: &usize = ... ;

    arena.mutate(|mutator, root| {
        let gc_data = Gc::new(mutator, 69);

        // this will error due to lifetime scope of 'gc being that of the mutation context
        thief = *gc_data;
    });
```

<a name="toc-write-barriers"></a>
## Write Barriers
Write barriers are mechanisms used in garbage collection to track changes to memory that could affect the state of the heap, particularly in generational and concurrent garbage collectors. Since such collectors often divide the heap into different regions (e.g., young and old generations), write barriers help ensure that when objects in one region reference objects in another, these references are correctly noted. This ensures that the garbage collector can handle intergenerational pointers and other memory interactions without missing any references during its collection process, maintaining program correctness.

In Sandpit write barriers can be obtained via the `GcMut<'gc, T>` and `GcOpt<'gc, T>` types.
```rust
    arena.mutate(|mutator, root| {
        let gc_mut = GcMut::new(mutator, true);

        // The fn write_barrier takes a callback which accepts a barrier type
        // that wraps the GcMut allowing it to be updated.
        root.write_barrier(mutator, |barrier| {
            barrier.set(GcMut::new(mutator, false));
        })
        // When the callback exits, the mutator will ensure that any updates to the root GcMut
        // will be tracked.
    });
```


<a name="toc-credits"></a>
## Credits
This project was originally inspired from [Writing Interpreters in Rust: a guide](https://rust-hosted-langs.github.io/book/) by Peter Liniker. After initially following the guide,
I branched off to start working on Sandpit by closely following the code in Katherine West's [gc-arena crate](https://github.com/kyren/gc-arena). I would not have been able to compelte this project
without Peter and Katherine's work. I am deeply grateful for their well documented, and insightful open source contributions.

<a name="toc-license"></a>
## License
Everything in this repository is licensed under either of:
- MIT license LICENSE-MIT or http://opensource.org/licenses/MIT
- Creative Commons CC0 1.0 Universal Public Domain Dedication LICENSE-CC0 or https://creativecommons.org/publicdomain/zero/1.0/ at your option.
