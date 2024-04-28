#[derive(Copy, Clone)]
pub struct TraceMetrics {
    pub objects_marked: usize,
    pub space_marked: usize,
    pub eden_collections: usize,
    pub full_collections: usize,
}

impl TraceMetrics {
    pub fn new() -> Self {
        Self {
            objects_marked: 0,
            space_marked: 0,
            eden_collections: 0,
            full_collections: 0,
        }
    }
}
