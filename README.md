VERSION 0.1.1
    - fuzzer test suite
    - Handling mutator panics? I think this may actually work already
    - GcError
    - Change Block to be Box<[u8]>
    - purge of unsafe
    - Defragmentation?
    - GcArray of TraceLeaf
    - allow for bringing your own allocator
    - add allocator config
    - editing config while gc is running?
    - review the trace and traceleaf derives
        - they dont work for things like unit/unamed structs

COMPLETED
    - #[derive(TraceLeaf)]
    - Mutation Input and Output
    - tracer channels
    - add GcConfig
    - free large objects
    - Trigger yield from space/time limits
