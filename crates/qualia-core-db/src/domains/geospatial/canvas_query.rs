//! Spatio-temporal region query — the P4 entry point over the quadtree index.

use super::spatial::{SpatialElement, SpatiotemporalQuadTree};

/// Query assets in a bounding box over a temporal window.
/// Returns H3/cell indices of matching elements.
pub fn query_region(
    tree: &SpatiotemporalQuadTree,
    bbox: (f64, f64, f64, f64),
    time_range: (u64, u64),
) -> Vec<u64> {
    let (x1, y1, x2, y2) = bbox;
    let (t0, t1) = time_range;
    tree.query_region(x1, y1, x2, y2, t0, t1)
}

/// Register an asset placement into the spatial index.
pub fn index_asset(
    tree: &mut SpatiotemporalQuadTree,
    h3_index: u64,
    bounds: (f64, f64, f64, f64),
    valid_from: u64,
    valid_until: u64,
) {
    tree.insert(SpatialElement {
        h3_index,
        bounds,
        t0: valid_from,
        t1: valid_until,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_region_delegates_to_quadtree() {
        let mut tree = SpatiotemporalQuadTree::new((0.0, 0.0, 100.0, 100.0));
        index_asset(&mut tree, 42, (10.0, 10.0, 20.0, 20.0), 100, 200);
        let hits = query_region(&tree, (15.0, 15.0, 25.0, 25.0), (150, 160));
        assert_eq!(hits, vec![42]);
    }
}
