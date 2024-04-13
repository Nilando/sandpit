#[derive(Copy, Clone)]
pub struct TraceMetrics {
    pub objects_marked: usize,
    pub space_marked: usize,
}

impl TraceMetrics {
    pub fn new() -> Self {
        Self {
            objects_marked: 0,
            space_marked: 0,
            //space_freed: 0
        }
    }
}
