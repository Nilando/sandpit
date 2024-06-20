VERSION 0.3.0
    - Alloc Blocks into Regions
    - Defragmentation
    - grow, shrink, and layout alloc options
    - switch to major during minor collection
    - fix GcError todos! find way to convert AllocError to GcError
    - Add more bench tests
    - Add A Fuzzing Test suite

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
    - Review of Atomic Operations
