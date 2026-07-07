pub struct SpatiotemporalQuadTree {
    pub root_bounds: (f64, f64, f64, f64),
    // A real implementation would store a tree of nodes with elements and temporal intervals.
    // For now, we stub a structure that could be queried by bbox and temporal range.
    pub elements: Vec<SpatialElement>,
}

pub struct SpatialElement {
    pub h3_index: u64,
    pub bounds: (f64, f64, f64, f64),
    pub t0: u64,
    pub t1: u64,
}

impl SpatiotemporalQuadTree {
    pub fn new(bounds: (f64, f64, f64, f64)) -> Self {
        Self {
            root_bounds: bounds,
            elements: Vec::new(),
        }
    }

    pub fn insert(&mut self, element: SpatialElement) {
        self.elements.push(element);
    }

    /// Queries the quadtree for elements that intersect the given spatial bounding box
    /// and are valid within the given temporal range [t0, t1].
    pub fn query_region(&self, x1: f64, y1: f64, x2: f64, y2: f64, t0: u64, t1: u64) -> Vec<u64> {
        let mut results = Vec::new();
        for el in &self.elements {
            // Check spatial intersection
            let overlap_x = el.bounds.0 <= x2 && el.bounds.2 >= x1;
            let overlap_y = el.bounds.1 <= y2 && el.bounds.3 >= y1;
            
            // Check temporal intersection
            let overlap_t = el.t0 <= t1 && el.t1 >= t0;
            
            if overlap_x && overlap_y && overlap_t {
                results.push(el.h3_index);
            }
        }
        results
    }
}

pub fn embed_h3_context(index: u64) -> u64 {
    // In a real implementation, this might embed resolution or base cell into the u64 hash.
    // For now we just pass through.
    index
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_region() {
        let mut qt = SpatiotemporalQuadTree::new((-180.0, -90.0, 180.0, 90.0));
        
        qt.insert(SpatialElement {
            h3_index: 101,
            bounds: (10.0, 10.0, 20.0, 20.0),
            t0: 1000,
            t1: 2000,
        });
        
        qt.insert(SpatialElement {
            h3_index: 202,
            bounds: (50.0, 50.0, 60.0, 60.0),
            t0: 1500,
            t1: 2500,
        });

        // Intersects first element spatially and temporally
        let res1 = qt.query_region(15.0, 15.0, 25.0, 25.0, 1200, 1800);
        assert_eq!(res1, vec![101]);

        // Spatially intersects but temporally disjoint
        let res2 = qt.query_region(15.0, 15.0, 25.0, 25.0, 3000, 4000);
        assert!(res2.is_empty());

        // Intersects both temporally, only second spatially
        let res3 = qt.query_region(40.0, 40.0, 55.0, 55.0, 1500, 2000);
        assert_eq!(res3, vec![202]);
    }
}
