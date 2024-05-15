pub struct GcMetrics {
    pub major_collections: usize,
    pub minor_collections: usize,
    pub max_old_objects: usize,
    pub old_objects_count: usize,
    pub arena_size: usize,
    pub prev_arena_size: usize,
}
