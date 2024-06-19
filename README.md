[![Tests](https://github.com/Nilando/sandpit/actions/workflows/rust.yml/badge.svg)](https://github.com/Nilando/sandpit/actions/workflows/rust.yml)

## A Concurrent GC Arena
Sandpit is a concurrent, generational, trace and sweep garbage collected arena meant for use in a VM. Access is given into the arena via a mutation callback which allows allocating of objects that impl the Trace trait. Allocating into the arena will eventually trigger a concurrent collection, in which tracing begins from the root of the arena and goes until all reachable objects have been marked. All unreachable objects will be freed after the mutators have been signaled to yield and all mutation contexts have exited.

##### *IMPORTANT*
Users of this GC are responsible for monitoring the mutation yield signal, and upon receiving it, exiting the mutation scope. Without exiting the scope, memory cannot be freed, and memory may run out.

## Write Barrier
Due to being a concurrent & generational collector, a write barrier must be implemented to ensure any updates to previously traced objects are not missed by the tracers. This is done by enforcing that GcPtr's be updated via a write barrier callback, which ensures a retrace is done if needed. Write barrier can also be implemented by hand, but requires using unsafe functions.



## Roadmap Stuff

- BUGS
    - write barrier can be misused
    - Miri hangs on node tests

- VERSION 0.3.0
    - Alloc Blocks into Regions
    - Add more bench tests
    - Add A Fuzzing Test suite
    - fix GcError todos!
    - Defragmentation
    - switch to major during minor collection
    - grow, shrink, and layout alloc options?
    - Complete trace and traceleaf derives
    - add a lot more trace impls

- OPEN ISSUES
    - Better Config
        - add allocator config?
        - easy swap allocator?
        - editing config while gc is running?
    - Mutator Context?
        - the presence of a GcPtr, GcArray, or Mutator all indicate we are in a mutator context
    - add non blocking versions of the gc mutate functions?
    - allow swapping out of root type?
    - Review of Atomic Operations
