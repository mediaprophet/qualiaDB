pub struct SpatiotemporalQuadTree {
    pub root_bounds: (f64, f64, f64, f64),
}
impl SpatiotemporalQuadTree {
    pub fn query_region(&self, _x1: f64, _y1: f64, _x2: f64, _y2: f64) -> Vec<u64> {
        Vec::new()
    }
}
pub fn embed_h3_context(index: u64) -> u64 {
    index
}
