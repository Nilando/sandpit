[![Tests](https://github.com/Nilando/sandpit/actions/workflows/rust.yml/badge.svg)](https://github.com/Nilando/sandpit/actions/workflows/rust.yml)

## A Concurrent GC Arena
Sandpit is a concurrent, generational, trace and sweep garbage collected arena 
meant for use in a VM. Access is given into the arena via a mutation callback 
which allows allocating of objects that impl the Trace trait. Allocating into 
the arena will eventually trigger a concurrent collection, in which tracing 
begins from the root of the arena and goes until all reachable objects have 
been marked. All unreachable objects will be freed after the mutators have been 
signaled to yield and all mutation contexts have exited.

### Yield Request
Users of this GC are responsible for monitoring the mutation yield signal, and 
upon receiving it, exiting the mutation scope. Without exiting the scope, 
memory cannot be freed, and memory may run out.

### Write Barriers
Due to being a concurrent & generational collector, write barriers are 
needed to ensure any GcPtr's that are updated to point at new objects are still
traced. Write barrier's can be implemented by the user by using a combination 
of the mutator's two methods `is_marked` and `retrace`.

### Trace and TraceLeaf Traits
Tracers find all references an object has by using the Trace trait's trace fn.
This trait can be auto derived for a type using Derive(Trace) which generates
a call to trace each GcPtr contained within the object. A type being TraceLeaf
indicates that the objects contains no references and in that sense is a 'leaf'
in the graph of reachable objects.

### Synchronization with Tracers
Synchronization with tracers must be ensured when mutating Gc types. For 
example Cell<T> is trace only if T is TraceLeaf. This is because tracers do not
need to read any pointers from TraceLeafs, so mutating a leaf without any 
synchronization is usually fine. However having a Cell<T> where T is Trace 
would be problematic in that there could potentially be a data race in which 
a tracer thread is reading GcPtrs from T while it is being mutated.

Synchronized mutation of Trace types can be achieved by creating custom Trace
impls for those types which 

## Roadmap Stuff

- BUGS

- VERSION 0.3.0
    - Swap out of root type
    - Evacuation of fragmented Blocks
    - separate allocator into another crate?
        - crate would need to be pulled in to allow for testing
        - move allocate_api to /src
    - make sweeping concurrent
    - grow, shrink, and layout alloc options

- OPEN ISSUES
    - Should the config be editable while gc is running?
    - Mutator Context Trait?
    - add non blocking versions of the gc mutate functions?
