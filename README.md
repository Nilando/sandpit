VERSION 0.1.1
    - Better Profiling
        - track gaps between blocks
            - FIX: allocate by regions and then divide the regions into blocks
        - how much of this can be done with a cli tool?
    - Fuzzer Test Suite
        - graph memory history
            - how much memory is allocated over time
            - trigger values over time
        - things to fuzz
            - config vars
            - alloc types align and size
            - write barriers
    - GcError Overview
        - what are all the errors we want to report?
        - is OOM even worth returning?
    - purge of unsafe
        - unsafe is evil it must be exterminated
    - Defragmentation
        - this is huge/would require a lot work 
            1. there would need to be some kind of defragmentation trigger
                - this wouold presumably require some way of calculating fragmentation
            2. to compact an object there would need to be a tracing algorithm
            that updates all ptrs to the object and updates those ptrs
    - Better Config
        - add allocator config
        - easy swap allocator?
        - editing config while gc is running?
    - review the trace and traceleaf derives
        - they dont work for a lot of types, things like unit/unamed structs
        - add a lot more trace impls
    - Mutator Context?
        - the presence of a GcPtr, GcArray, or Mutator all indicate we are in a mutator context
    - add non blocking versions of the gc mutate functions?
            

COMPLETED
    - #[derive(TraceLeaf)]
    - Mutation Input and Output
    - tracer channels
    - add GcConfig
    - free large objects
    - Trigger yield from space/time limits
    - using derives inside of /src
