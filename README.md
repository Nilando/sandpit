VERSION 0.1.1
    - Trigger yield from space/time limits
    - #[derive(TraceLeaf)]
    - Mutation Input and Output
    - fuzzer test suite
    - free large objects
    - Handling mutator panics? I think this may actually work already
    - GcError
    - Change Block to be Box<[u8]>
    - purge of unsafe
    - Defragmentation?
    - GcArray of TraceLeaf

COMPLETED
    - GcConfig
        - TODO: allow for bringing your own allocator
    - tracer channels
