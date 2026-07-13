#[derive(Debug, Clone)]
pub struct SpatialElement {
    pub h3_index: u64,
    pub bounds: (f64, f64, f64, f64), // (min_x, min_y, max_x, max_y)
    pub t0: u64,
    pub t1: u64,
}

#[derive(Debug, Clone)]
enum QuadTreeNode {
    Leaf {
        elements: Vec<SpatialElement>,
    },
    Internal {
        children: Box<[QuadTreeNode; 4]>,
    },
}

#[derive(Debug, Clone)]
pub struct SpatiotemporalQuadTree {
    pub root_bounds: (f64, f64, f64, f64),
    root: QuadTreeNode,
    max_elements: usize,
    max_depth: u32,
}

impl SpatiotemporalQuadTree {
    pub fn new(bounds: (f64, f64, f64, f64)) -> Self {
        Self {
            root_bounds: bounds,
            root: QuadTreeNode::Leaf { elements: Vec::new() },
            max_elements: 16,
            max_depth: 8,
        }
    }

    pub fn insert(&mut self, element: SpatialElement) {
        Self::insert_into(&mut self.root, self.root_bounds, element, 0, self.max_elements, self.max_depth);
    }

    fn insert_into(
        node: &mut QuadTreeNode,
        bounds: (f64, f64, f64, f64),
        element: SpatialElement,
        depth: u32,
        max_elements: usize,
        max_depth: u32,
    ) {
        match node {
            QuadTreeNode::Leaf { elements } => {
                elements.push(element);
                if elements.len() > max_elements && depth < max_depth {
                    // Split the node
                    let (min_x, min_y, max_x, max_y) = bounds;
                    let mid_x = (min_x + max_x) / 2.0;
                    let mid_y = (min_y + max_y) / 2.0;

                    let mut top_left = Vec::new();
                    let mut top_right = Vec::new();
                    let mut bottom_left = Vec::new();
                    let mut bottom_right = Vec::new();

                    for el in elements.drain(..) {
                        let (el_min_x, el_min_y, el_max_x, el_max_y) = el.bounds;
                        // Determine which quadrants the element belongs to
                        let left = el_min_x <= mid_x;
                        let right = el_max_x >= mid_x;
                        let bottom = el_min_y <= mid_y;
                        let top = el_max_y >= mid_y;

                        if left && top { top_left.push(el.clone()); }
                        if right && top { top_right.push(el.clone()); }
                        if left && bottom { bottom_left.push(el.clone()); }
                        if right && bottom { bottom_right.push(el.clone()); }
                    }

                    *node = QuadTreeNode::Internal {
                        children: Box::new([
                            QuadTreeNode::Leaf { elements: top_left },
                            QuadTreeNode::Leaf { elements: top_right },
                            QuadTreeNode::Leaf { elements: bottom_left },
                            QuadTreeNode::Leaf { elements: bottom_right },
                        ]),
                    };
                }
            }
            QuadTreeNode::Internal { children } => {
                let (min_x, min_y, max_x, max_y) = bounds;
                let mid_x = (min_x + max_x) / 2.0;
                let mid_y = (min_y + max_y) / 2.0;

                let (el_min_x, el_min_y, el_max_x, el_max_y) = element.bounds;
                let left = el_min_x <= mid_x;
                let right = el_max_x >= mid_x;
                let bottom = el_min_y <= mid_y;
                let top = el_max_y >= mid_y;

                if left && top {
                    Self::insert_into(&mut children[0], (min_x, mid_y, mid_x, max_y), element.clone(), depth + 1, max_elements, max_depth);
                }
                if right && top {
                    Self::insert_into(&mut children[1], (mid_x, mid_y, max_x, max_y), element.clone(), depth + 1, max_elements, max_depth);
                }
                if left && bottom {
                    Self::insert_into(&mut children[2], (min_x, min_y, mid_x, mid_y), element.clone(), depth + 1, max_elements, max_depth);
                }
                if right && bottom {
                    Self::insert_into(&mut children[3], (mid_x, min_y, max_x, mid_y), element.clone(), depth + 1, max_elements, max_depth);
                }
            }
        }
    }

    /// Queries the quadtree for elements that intersect the given spatial bounding box
    /// and are valid within the given temporal range [t0, t1].
    pub fn query_region(&self, x1: f64, y1: f64, x2: f64, y2: f64, t0: u64, t1: u64) -> Vec<u64> {
        let mut results = Vec::new();
        Self::query_node(&self.root, self.root_bounds, (x1, y1, x2, y2), t0, t1, &mut results);
        
        // Remove duplicates because elements might span multiple quadrants
        results.sort_unstable();
        results.dedup();
        results
    }

    fn query_node(
        node: &QuadTreeNode,
        bounds: (f64, f64, f64, f64),
        query_bounds: (f64, f64, f64, f64),
        t0: u64,
        t1: u64,
        results: &mut Vec<u64>,
    ) {
        let (qx1, qy1, qx2, qy2) = query_bounds;
        let (bx1, by1, bx2, by2) = bounds;

        // If the query region doesn't intersect the node's bounds, skip
        if qx1 > bx2 || qx2 < bx1 || qy1 > by2 || qy2 < by1 {
            return;
        }

        match node {
            QuadTreeNode::Leaf { elements } => {
                for el in elements {
                    let overlap_x = el.bounds.0 <= qx2 && el.bounds.2 >= qx1;
                    let overlap_y = el.bounds.1 <= qy2 && el.bounds.3 >= qy1;
                    let overlap_t = el.t0 <= t1 && el.t1 >= t0;

                    if overlap_x && overlap_y && overlap_t {
                        results.push(el.h3_index);
                    }
                }
            }
            QuadTreeNode::Internal { children } => {
                let mid_x = (bx1 + bx2) / 2.0;
                let mid_y = (by1 + by2) / 2.0;

                Self::query_node(&children[0], (bx1, mid_y, mid_x, by2), query_bounds, t0, t1, results);
                Self::query_node(&children[1], (mid_x, mid_y, bx2, by2), query_bounds, t0, t1, results);
                Self::query_node(&children[2], (bx1, by1, mid_x, mid_y), query_bounds, t0, t1, results);
                Self::query_node(&children[3], (mid_x, by1, bx2, mid_y), query_bounds, t0, t1, results);
            }
        }
    }
}

/// Embeds an H3 index into a unified 64-bit coordinate space index, commonly used in MORTON codes
/// or spatial hashing systems where bit packing includes resolution or other contextual data.
pub fn embed_h3_context(index: u64, resolution: u8, base_cell: u8) -> u64 {
    // 64-bit layout: [1 bit reserved][4 bits resolution][7 bits base_cell][52 bits H3 internal]
    let res_bits = ((resolution & 0x0F) as u64) << 59;
    let base_bits = ((base_cell & 0x7F) as u64) << 52;
    let index_bits = index & 0x000F_FFFF_FFFF_FFFF;
    res_bits | base_bits | index_bits
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

    #[test]
    fn test_quadtree_split() {
        let mut qt = SpatiotemporalQuadTree::new((0.0, 0.0, 100.0, 100.0));
        qt.max_elements = 2; // Force early split

        for i in 0..5 {
            qt.insert(SpatialElement {
                h3_index: i,
                bounds: (10.0 + (i as f64), 10.0 + (i as f64), 15.0 + (i as f64), 15.0 + (i as f64)),
                t0: 100,
                t1: 200,
            });
        }
        
        let res = qt.query_region(0.0, 0.0, 50.0, 50.0, 50, 250);
        assert_eq!(res.len(), 5);
    }
}
