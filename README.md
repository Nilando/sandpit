VERSION 0.1.1
    - Fuzzer Test Suite
        - graph memory history
            - how much memory is allocated over time
            - trigger values over time
        - things to fuzz
            - config vars
            - alloc types align and size
            - write barriers
            - 
    - GcError Overview
        - what are all the errors we want to report?
        - is OOM even worth returning?
    - purge of unsafe
        - unsafe is evil it must be exterminated
    - Defragmentation
        - this is huge/would require a lot work 
    - Better Config
        - add allocator config
        - easy swap allocator?
        - editing config while gc is running?
    - review the trace and traceleaf derives
        - they dont work for things like unit/unamed structs
        - add a lot more trace impls
    - Better Profiling
        - track gaps between blocks
        - how much of this can be done with a cli tool?
    - GcArrays
        - GcArrays should be implemented so that they have const size
        - should making a vec(dyn len array) fall on the user of this library?
    - Mutator Context?
        - the presence of a GcPtr, GcArray, or Mutator all indicate we are in a mutator context

            

COMPLETED
    - #[derive(TraceLeaf)]
    - Mutation Input and Output
    - tracer channels
    - add GcConfig
    - free large objects
    - Trigger yield from space/time limits
    - using derives inside of /src
