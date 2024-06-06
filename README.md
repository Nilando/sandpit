VERSION 0.1.1
    - fuzzer test suite
    - GcError Overview
    - purge of unsafe
    - Defragmentation?
    - allow for bringing your own allocator
    - add allocator config
    - editing config while gc is running?
    - review the trace and traceleaf derives
        - they dont work for things like unit/unamed structs
    - track collection times
    - add a lot more trace impls
    - memory profiling functionality

COMPLETED
    - #[derive(TraceLeaf)]
    - Mutation Input and Output
    - tracer channels
    - add GcConfig
    - free large objects
    - Trigger yield from space/time limits
    - using derives inside of /src
