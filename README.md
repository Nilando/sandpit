VERSION 0.3.0
    - Alloc Blocks into Regions
    - Add more bench tests
    - Add A Fuzzing Test suite
    - fix GcError todos!
    - Defragmentation
    - switch to major during minor collection
    - grow, shrink, and layout alloc options

VERSION 0.2.0
    - #[derive(TraceLeaf)]
    - Mutation Input and Output
    - tracer channels
    - add GcConfig
    - free large objects
    - Trigger yield from space/time limits
    - using derives inside of /src

ISSUES
    - Better Config
        - add allocator config?
        - easy swap allocator?
        - editing config while gc is running?
    - review the trace and traceleaf derives
        - they dont work for a lot of types, things like unit/unamed structs
        - add a lot more trace impls
    - Mutator Context?
        - the presence of a GcPtr, GcArray, or Mutator all indicate we are in a mutator context
    - add non blocking versions of the gc mutate functions?
