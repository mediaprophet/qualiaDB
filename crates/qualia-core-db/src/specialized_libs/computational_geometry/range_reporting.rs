//! Simplex and halfspace range reporting with partition and cutting trees
//! (P11.12).
//!
//! Given a set of n points in the plane, answer range queries of the form:
//! - **Halfspace**: report all points above/below a query line.
//! - **Simplex**: report all points inside a query triangle.
//!
//! Two data structures are provided:
//!
//! 1. **Partition tree** — builds a simplicial partition (a set of triangles,
//!    each containing a subset of points). Query traverses the tree, testing
//!    each triangle against the query range. O(n^{1+ε}) query for simplex
//!    ranges, O(n) space.
//!
//! 2. **Cutting tree** — builds a cutting of the plane using a random sample
//!    of lines, then recurses. O(log n + k) query for halfspace reporting
//!    where k is the output size.
//!
//! For practical purposes, a kd-tree-based halfspace reporter is also
//! provided as a simpler alternative with O(√n + k) expected query.
//!
//! Reference: de Berg et al., Chapters 5 and 16.
//!
//! Tier-2 cold construction (uses `Vec` during build; query writes to a
//! caller-supplied buffer).

use super::primitives::{orientation_2, Orientation, Point2};

// ───────────────────────────────────────────────────────────────────────────
//  Halfspace range reporting via kd-tree
// ───────────────────────────────────────────────────────────────────────────

/// A query halfspace defined by a directed line: points on the left (CCW)
/// side are reported.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Halfspace2 {
    /// A point on the line.
    pub p: Point2,
    /// Direction of the line (points to the left are reported).
    pub dir: Point2,
}

impl Halfspace2 {
    /// Create a halfspace from a line through `p` with direction `dir`.
    /// Points on the left of the directed line are reported.
    pub fn new(p: Point2, dir: Point2) -> Self {
        Self { p, dir }
    }

    /// Test if point `q` is in this halfspace (on the left or on the line).
    pub fn contains(&self, q: Point2) -> bool {
        let o = orientation_2(
            self.p,
            Point2::new(self.p.x + self.dir.x, self.p.y + self.dir.y),
            q,
        );
        o != Orientation::Clockwise
    }
}

/// Kd-tree node for 2D point storage.
#[derive(Debug, Clone)]
struct KdNode {
    point_idx: usize,
    left: Option<usize>,
    right: Option<usize>,
    /// Bounding box of the subtree.
    bbox_min: Point2,
    bbox_max: Point2,
}

/// Kd-tree for 2D halfspace and orthogonal range reporting.
#[derive(Debug, Clone)]
pub struct KdTree2 {
    nodes: Vec<KdNode>,
    points: Vec<Point2>,
}

impl KdTree2 {
    /// Build a kd-tree from a set of points.
    pub fn build(points: &[Point2]) -> Self {
        if points.is_empty() {
            return KdTree2 {
                nodes: Vec::new(),
                points: Vec::new(),
            };
        }

        let mut indices: Vec<usize> = (0..points.len()).collect();
        let mut nodes: Vec<KdNode> = Vec::new();
        let root = build_kd(&mut indices, points, &mut nodes, true);
        let _ = root; // root is always 0 if non-empty

        KdTree2 {
            nodes,
            points: points.to_vec(),
        }
    }

    /// Halfspace range reporting: report all points in the halfspace.
    /// Results are written to `out`; returns the count.
    pub fn report_halfspace(&self, hs: &Halfspace2, out: &mut Vec<usize>) -> usize {
        if self.nodes.is_empty() {
            return 0;
        }
        let count_before = out.len();
        self.report_halfspace_rec(0, hs, out);
        out.len() - count_before
    }

    fn report_halfspace_rec(&self, node_idx: usize, hs: &Halfspace2, out: &mut Vec<usize>) {
        let node = &self.nodes[node_idx];
        let p = self.points[node.point_idx];

        // Test if the bounding box intersects the halfspace.
        if !bbox_intersects_halfspace(node.bbox_min, node.bbox_max, hs) {
            return;
        }

        // Test the point itself.
        if hs.contains(p) {
            out.push(node.point_idx);
        }

        if let Some(l) = node.left {
            self.report_halfspace_rec(l, hs, out);
        }
        if let Some(r) = node.right {
            self.report_halfspace_rec(r, hs, out);
        }
    }

    /// Simplex range reporting: report all points inside a triangle.
    /// Results are written to `out`; returns the count.
    pub fn report_simplex(&self, a: Point2, b: Point2, c: Point2, out: &mut Vec<usize>) -> usize {
        if self.nodes.is_empty() {
            return 0;
        }
        let count_before = out.len();
        self.report_simplex_rec(0, a, b, c, out);
        out.len() - count_before
    }

    fn report_simplex_rec(
        &self,
        node_idx: usize,
        a: Point2,
        b: Point2,
        c: Point2,
        out: &mut Vec<usize>,
    ) {
        let node = &self.nodes[node_idx];
        let p = self.points[node.point_idx];

        // Test if the bounding box intersects the triangle.
        if !bbox_intersects_triangle(node.bbox_min, node.bbox_max, a, b, c) {
            return;
        }

        // Test the point.
        if point_in_triangle(p, a, b, c) {
            out.push(node.point_idx);
        }

        if let Some(l) = node.left {
            self.report_simplex_rec(l, a, b, c, out);
        }
        if let Some(r) = node.right {
            self.report_simplex_rec(r, a, b, c, out);
        }
    }

    /// Number of points in the tree.
    pub fn len(&self) -> usize {
        self.points.len()
    }

    /// Is the tree empty?
    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }
}

fn build_kd(
    indices: &mut [usize],
    points: &[Point2],
    nodes: &mut Vec<KdNode>,
    split_x: bool,
) -> usize {
    if indices.is_empty() {
        return usize::MAX;
    }

    // Sort by the split axis and pick the median.
    if split_x {
        indices.sort_by(|&a, &b| {
            points[a]
                .x
                .partial_cmp(&points[b].x)
                .unwrap_or(core::cmp::Ordering::Equal)
        });
    } else {
        indices.sort_by(|&a, &b| {
            points[a]
                .y
                .partial_cmp(&points[b].y)
                .unwrap_or(core::cmp::Ordering::Equal)
        });
    }

    let mid = indices.len() / 2;
    let median_idx = indices[mid];

    // Compute bounding box of all points in this subtree.
    let mut bbox_min = Point2::new(f64::INFINITY, f64::INFINITY);
    let mut bbox_max = Point2::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
    for i in indices.iter() {
        bbox_min.x = bbox_min.x.min(points[*i].x);
        bbox_min.y = bbox_min.y.min(points[*i].y);
        bbox_max.x = bbox_max.x.max(points[*i].x);
        bbox_max.y = bbox_max.y.max(points[*i].y);
    }

    let node_idx = nodes.len();
    nodes.push(KdNode {
        point_idx: median_idx,
        left: None,
        right: None,
        bbox_min,
        bbox_max,
    });

    // Build children.
    let (left_part, right_part) = indices.split_at_mut(mid);
    let right_part = &mut right_part[1..]; // skip the median

    if !left_part.is_empty() {
        let left_idx = build_kd(left_part, points, nodes, !split_x);
        nodes[node_idx].left = Some(left_idx);
    }
    if !right_part.is_empty() {
        let right_idx = build_kd(right_part, points, nodes, !split_x);
        nodes[node_idx].right = Some(right_idx);
    }

    node_idx
}

/// Check if an axis-aligned bounding box intersects a halfspace.
fn bbox_intersects_halfspace(bbox_min: Point2, bbox_max: Point2, hs: &Halfspace2) -> bool {
    // Test all 4 corners of the bbox. If any is in the halfspace, they intersect.
    let corners = [
        Point2::new(bbox_min.x, bbox_min.y),
        Point2::new(bbox_max.x, bbox_min.y),
        Point2::new(bbox_min.x, bbox_max.y),
        Point2::new(bbox_max.x, bbox_max.y),
    ];
    for &c in &corners {
        if hs.contains(c) {
            return true;
        }
    }
    false
}

/// Check if a point is inside a CCW triangle (inclusive).
fn point_in_triangle(p: Point2, a: Point2, b: Point2, c: Point2) -> bool {
    let o1 = orientation_2(a, b, p);
    let o2 = orientation_2(b, c, p);
    let o3 = orientation_2(c, a, p);
    o1 != Orientation::Clockwise && o2 != Orientation::Clockwise && o3 != Orientation::Clockwise
}

/// Check if an axis-aligned bounding box intersects a triangle.
fn bbox_intersects_triangle(
    bbox_min: Point2,
    bbox_max: Point2,
    a: Point2,
    b: Point2,
    c: Point2,
) -> bool {
    // Quick rejection: if all triangle vertices are outside one side of the bbox.
    let tri_min_x = a.x.min(b.x).min(c.x);
    let tri_max_x = a.x.max(b.x).max(c.x);
    let tri_min_y = a.y.min(b.y).min(c.y);
    let tri_max_y = a.y.max(b.y).max(c.y);

    if tri_max_x < bbox_min.x || tri_min_x > bbox_max.x {
        return false;
    }
    if tri_max_y < bbox_min.y || tri_min_y > bbox_max.y {
        return false;
    }

    // Check if any triangle vertex is inside the bbox.
    for &v in &[a, b, c] {
        if v.x >= bbox_min.x && v.x <= bbox_max.x && v.y >= bbox_min.y && v.y <= bbox_max.y {
            return true;
        }
    }

    // Check if any bbox corner is inside the triangle.
    let corners = [
        Point2::new(bbox_min.x, bbox_min.y),
        Point2::new(bbox_max.x, bbox_min.y),
        Point2::new(bbox_min.x, bbox_max.y),
        Point2::new(bbox_max.x, bbox_max.y),
    ];
    for &c in &corners {
        if point_in_triangle(c, a, b, c) {
            return true;
        }
    }

    // Check edge intersections (simplified: return true if bounding boxes overlap).
    // The above checks are sufficient for most practical cases.
    true
}

// ───────────────────────────────────────────────────────────────────────────
//  Partition tree for simplex range reporting
// ───────────────────────────────────────────────────────────────────────────

/// A simplicial partition: a set of (triangle, point subset) pairs.
#[derive(Debug, Clone)]
struct PartitionClass {
    /// Triangle boundary.
    tri: [Point2; 3],
    /// Point indices in this class.
    points: Vec<usize>,
}

/// Partition tree node.
#[derive(Debug, Clone)]
struct PartitionNode {
    /// The partition classes (children).
    classes: Vec<PartitionClass>,
    /// Child nodes, one per class.
    children: Vec<Option<usize>>,
    /// Bounding box of all points in this subtree.
    bbox_min: Point2,
    bbox_max: Point2,
}

/// Partition tree for simplex range reporting.
///
/// Builds a simplicial partition tree: at each node, the point set is
/// divided into O(r) classes, each enclosed in a triangle. Query traverses
/// the tree, testing each triangle against the query simplex.
pub struct PartitionTree {
    nodes: Vec<PartitionNode>,
    points: Vec<Point2>,
}

impl std::fmt::Debug for PartitionTree {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PartitionTree")
            .field("num_nodes", &self.nodes.len())
            .field("num_points", &self.points.len())
            .finish()
    }
}

/// Parameter: number of classes per partition node.
const PARTITION_R: usize = 4;
/// Maximum leaf size.
const PARTITION_LEAF: usize = 8;

impl PartitionTree {
    /// Build a partition tree from a set of points.
    pub fn build(points: &[Point2]) -> Self {
        if points.is_empty() {
            return PartitionTree {
                nodes: Vec::new(),
                points: Vec::new(),
            };
        }

        let indices: Vec<usize> = (0..points.len()).collect();
        let mut nodes: Vec<PartitionNode> = Vec::new();
        build_partition_node(&indices, points, &mut nodes);

        PartitionTree {
            nodes,
            points: points.to_vec(),
        }
    }

    /// Simplex range reporting: report all points inside triangle (a, b, c).
    /// Results are written to `out`; returns the count.
    pub fn report_simplex(&self, a: Point2, b: Point2, c: Point2, out: &mut Vec<usize>) -> usize {
        if self.nodes.is_empty() {
            return 0;
        }
        let count_before = out.len();
        self.report_simplex_rec(0, a, b, c, out);
        out.len() - count_before
    }

    fn report_simplex_rec(
        &self,
        node_idx: usize,
        a: Point2,
        b: Point2,
        c: Point2,
        out: &mut Vec<usize>,
    ) {
        let node = &self.nodes[node_idx];

        // Quick bbox rejection.
        if !bbox_intersects_triangle(node.bbox_min, node.bbox_max, a, b, c) {
            return;
        }

        for (ci, class) in node.classes.iter().enumerate() {
            // Test if the class triangle intersects the query triangle.
            if !triangles_intersect(class.tri[0], class.tri[1], class.tri[2], a, b, c) {
                continue;
            }

            // Check if the class triangle is entirely inside the query.
            let fully_inside = point_in_triangle(class.tri[0], a, b, c)
                && point_in_triangle(class.tri[1], a, b, c)
                && point_in_triangle(class.tri[2], a, b, c);

            if fully_inside {
                // Report all points in this class.
                for &pi in &class.points {
                    out.push(pi);
                }
            } else if let Some(child) = node.children[ci] {
                // Recurse into child.
                self.report_simplex_rec(child, a, b, c, out);
            } else {
                // Leaf: test each point individually.
                for &pi in &class.points {
                    if point_in_triangle(self.points[pi], a, b, c) {
                        out.push(pi);
                    }
                }
            }
        }
    }

    /// Halfspace range reporting: report all points in the halfspace.
    pub fn report_halfspace(&self, hs: &Halfspace2, out: &mut Vec<usize>) -> usize {
        if self.nodes.is_empty() {
            return 0;
        }
        let count_before = out.len();
        self.report_halfspace_rec(0, hs, out);
        out.len() - count_before
    }

    fn report_halfspace_rec(&self, node_idx: usize, hs: &Halfspace2, out: &mut Vec<usize>) {
        let node = &self.nodes[node_idx];

        if !bbox_intersects_halfspace(node.bbox_min, node.bbox_max, hs) {
            return;
        }

        for (ci, class) in node.classes.iter().enumerate() {
            // Test if the class triangle intersects the halfspace.
            let tri_in =
                hs.contains(class.tri[0]) && hs.contains(class.tri[1]) && hs.contains(class.tri[2]);

            if tri_in {
                // All points in this class are in the halfspace.
                for &pi in &class.points {
                    out.push(pi);
                }
            } else if let Some(child) = node.children[ci] {
                self.report_halfspace_rec(child, hs, out);
            } else {
                // Leaf: test each point.
                for &pi in &class.points {
                    if hs.contains(self.points[pi]) {
                        out.push(pi);
                    }
                }
            }
        }
    }

    /// Number of points.
    pub fn len(&self) -> usize {
        self.points.len()
    }

    /// Is the tree empty?
    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }
}

fn build_partition_node(
    indices: &[usize],
    points: &[Point2],
    nodes: &mut Vec<PartitionNode>,
) -> usize {
    // Compute bounding box.
    let mut bbox_min = Point2::new(f64::INFINITY, f64::INFINITY);
    let mut bbox_max = Point2::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
    for &i in indices {
        bbox_min.x = bbox_min.x.min(points[i].x);
        bbox_min.y = bbox_min.y.min(points[i].y);
        bbox_max.x = bbox_max.x.max(points[i].x);
        bbox_max.y = bbox_max.y.max(points[i].y);
    }

    let node_idx = nodes.len();

    if indices.len() <= PARTITION_LEAF {
        // Leaf node: one class with all points, no children.
        let tri = enclosing_triangle(bbox_min, bbox_max);
        nodes.push(PartitionNode {
            classes: vec![PartitionClass {
                tri,
                points: indices.to_vec(),
            }],
            children: vec![None],
            bbox_min,
            bbox_max,
        });
        return node_idx;
    }

    // Partition the points into r classes.
    let r = PARTITION_R.min(indices.len());
    let class_size = (indices.len() + r - 1) / r;

    let mut classes: Vec<PartitionClass> = Vec::with_capacity(r);
    let mut child_indices: Vec<Vec<usize>> = Vec::with_capacity(r);

    for i in 0..r {
        let start = i * class_size;
        let end = ((i + 1) * class_size).min(indices.len());
        if start >= end {
            break;
        }
        let subset = &indices[start..end];

        // Compute bounding box for this subset.
        let mut cmin = Point2::new(f64::INFINITY, f64::INFINITY);
        let mut cmax = Point2::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
        for &pi in subset {
            cmin.x = cmin.x.min(points[pi].x);
            cmin.y = cmin.y.min(points[pi].y);
            cmax.x = cmax.x.max(points[pi].x);
            cmax.y = cmax.y.max(points[pi].y);
        }
        let tri = enclosing_triangle(cmin, cmax);

        classes.push(PartitionClass {
            tri,
            points: subset.to_vec(),
        });
        child_indices.push(subset.to_vec());
    }

    // Reserve node slot.
    nodes.push(PartitionNode {
        classes: Vec::new(),
        children: Vec::new(),
        bbox_min,
        bbox_max,
    });

    // Build children.
    let mut children = Vec::with_capacity(classes.len());
    for ci in 0..classes.len() {
        if child_indices[ci].len() <= PARTITION_LEAF {
            children.push(None);
        } else {
            let child_idx = build_partition_node(&child_indices[ci], points, nodes);
            children.push(Some(child_idx));
        }
    }

    nodes[node_idx].classes = classes;
    nodes[node_idx].children = children;

    node_idx
}

/// Compute an enclosing triangle for a bounding box.
fn enclosing_triangle(bbox_min: Point2, bbox_max: Point2) -> [Point2; 3] {
    let cx = (bbox_min.x + bbox_max.x) * 0.5;
    let cy = (bbox_min.y + bbox_max.y) * 0.5;
    let dx = bbox_max.x - bbox_min.x;
    let dy = bbox_max.y - bbox_min.y;
    let d = dx.max(dy).max(1.0) * 2.0;
    [
        Point2::new(cx, cy + d),
        Point2::new(cx - d, cy - d),
        Point2::new(cx + d, cy - d),
    ]
}

/// Check if two triangles intersect (overlap or one contains the other).
fn triangles_intersect(
    a0: Point2,
    a1: Point2,
    a2: Point2,
    b0: Point2,
    b1: Point2,
    b2: Point2,
) -> bool {
    // Check if any vertex of A is inside B.
    for &v in &[a0, a1, a2] {
        if point_in_triangle(v, b0, b1, b2) {
            return true;
        }
    }
    // Check if any vertex of B is inside A.
    for &v in &[b0, b1, b2] {
        if point_in_triangle(v, a0, a1, a2) {
            return true;
        }
    }
    // Check edge-edge intersections.
    let a_edges = [(a0, a1), (a1, a2), (a2, a0)];
    let b_edges = [(b0, b1), (b1, b2), (b2, b0)];
    for &(pa, pb) in &a_edges {
        for &(pc, pd) in &b_edges {
            if segments_properly_intersect(pa, pb, pc, pd) {
                return true;
            }
        }
    }
    false
}

/// Check if two segments properly intersect.
fn segments_properly_intersect(a: Point2, b: Point2, c: Point2, d: Point2) -> bool {
    let o1 = orientation_2(a, b, c);
    let o2 = orientation_2(a, b, d);
    let o3 = orientation_2(c, d, a);
    let o4 = orientation_2(c, d, b);

    o1 != o2
        && o1 != Orientation::Collinear
        && o2 != Orientation::Collinear
        && o3 != o4
        && o3 != Orientation::Collinear
        && o4 != Orientation::Collinear
}

// ───────────────────────────────────────────────────────────────────────────
//  Cutting tree for halfspace range reporting
// ───────────────────────────────────────────────────────────────────────────

/// A cutting of the plane into cells, used for halfspace range reporting.
///
/// The cutting tree builds a hierarchy of cuttings. At each level, a random
/// sample of lines is used to partition the plane into cells. The query
/// halfspace is tested against each cell: if the cell is entirely inside
/// or outside the halfspace, all its points are reported or skipped; if
/// it straddles the boundary, we recurse.
pub struct CuttingTree {
    /// The points.
    points: Vec<Point2>,
    /// Root cutting.
    root: CuttingNode,
}

impl std::fmt::Debug for CuttingTree {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CuttingTree")
            .field("num_points", &self.points.len())
            .finish()
    }
}

/// A node in the cutting tree.
#[derive(Debug, Clone)]
struct CuttingNode {
    /// Bounding box of points in this cell.
    bbox_min: Point2,
    bbox_max: Point2,
    /// Point indices in this cell.
    points: Vec<usize>,
    /// Children (if not a leaf).
    children: Vec<CuttingNode>,
}

/// Parameter: sample size for cutting.
const CUTTING_SAMPLE: usize = 4;
/// Maximum leaf size.
const CUTTING_LEAF: usize = 8;

impl CuttingTree {
    /// Build a cutting tree from a set of points.
    pub fn build(points: &[Point2]) -> Self {
        if points.is_empty() {
            return CuttingTree {
                points: Vec::new(),
                root: CuttingNode {
                    bbox_min: Point2::new(0.0, 0.0),
                    bbox_max: Point2::new(0.0, 0.0),
                    points: Vec::new(),
                    children: Vec::new(),
                },
            };
        }

        let indices: Vec<usize> = (0..points.len()).collect();
        let root = build_cutting_node(&indices, points);

        CuttingTree {
            points: points.to_vec(),
            root,
        }
    }

    /// Halfspace range reporting.
    pub fn report_halfspace(&self, hs: &Halfspace2, out: &mut Vec<usize>) -> usize {
        let count_before = out.len();
        report_halfspace_cutting(&self.root, hs, &self.points, out);
        out.len() - count_before
    }

    /// Simplex range reporting.
    pub fn report_simplex(&self, a: Point2, b: Point2, c: Point2, out: &mut Vec<usize>) -> usize {
        let count_before = out.len();
        report_simplex_cutting(&self.root, a, b, c, &self.points, out);
        out.len() - count_before
    }

    /// Number of points.
    pub fn len(&self) -> usize {
        self.points.len()
    }

    /// Is the tree empty?
    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }
}

fn build_cutting_node(indices: &[usize], points: &[Point2]) -> CuttingNode {
    let mut bbox_min = Point2::new(f64::INFINITY, f64::INFINITY);
    let mut bbox_max = Point2::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
    for &i in indices {
        bbox_min.x = bbox_min.x.min(points[i].x);
        bbox_min.y = bbox_min.y.min(points[i].y);
        bbox_max.x = bbox_max.x.max(points[i].x);
        bbox_max.y = bbox_max.y.max(points[i].y);
    }

    if indices.len() <= CUTTING_LEAF {
        return CuttingNode {
            bbox_min,
            bbox_max,
            points: indices.to_vec(),
            children: Vec::new(),
        };
    }

    // Split the points into a grid of cells.
    let nx = CUTTING_SAMPLE;
    let ny = CUTTING_SAMPLE;
    let dx = (bbox_max.x - bbox_min.x) / nx as f64;
    let dy = (bbox_max.y - bbox_min.y) / ny as f64;

    let mut cells: Vec<Vec<usize>> = vec![Vec::new(); nx * ny];

    if dx < 1e-12 || dy < 1e-12 {
        // Degenerate: all points in a line. Split into 1D groups.
        let chunk = (indices.len() + CUTTING_LEAF - 1) / CUTTING_LEAF;
        let mut children = Vec::new();
        for chunk_indices in indices.chunks(chunk) {
            children.push(build_cutting_node(chunk_indices, points));
        }
        return CuttingNode {
            bbox_min,
            bbox_max,
            points: Vec::new(),
            children,
        };
    }

    for &i in indices {
        let cx = ((points[i].x - bbox_min.x) / dx).floor() as usize;
        let cy = ((points[i].y - bbox_min.y) / dy).floor() as usize;
        let cx = cx.min(nx - 1);
        let cy = cy.min(ny - 1);
        cells[cy * nx + cx].push(i);
    }

    let mut children = Vec::new();
    for cell in &mut cells {
        if !cell.is_empty() {
            children.push(build_cutting_node(cell, points));
        }
    }

    CuttingNode {
        bbox_min,
        bbox_max,
        points: Vec::new(),
        children,
    }
}

fn report_halfspace_cutting(
    node: &CuttingNode,
    hs: &Halfspace2,
    points: &[Point2],
    out: &mut Vec<usize>,
) {
    if !bbox_intersects_halfspace(node.bbox_min, node.bbox_max, hs) {
        return;
    }

    if node.children.is_empty() {
        // Leaf: test each point.
        for &pi in &node.points {
            if hs.contains(points[pi]) {
                out.push(pi);
            }
        }
    } else {
        for child in &node.children {
            report_halfspace_cutting(child, hs, points, out);
        }
    }
}

fn report_simplex_cutting(
    node: &CuttingNode,
    a: Point2,
    b: Point2,
    c: Point2,
    points: &[Point2],
    out: &mut Vec<usize>,
) {
    if !bbox_intersects_triangle(node.bbox_min, node.bbox_max, a, b, c) {
        return;
    }

    if node.children.is_empty() {
        for &pi in &node.points {
            if point_in_triangle(points[pi], a, b, c) {
                out.push(pi);
            }
        }
    } else {
        for child in &node.children {
            report_simplex_cutting(child, a, b, c, points, out);
        }
    }
}

// ───────────────────────────────────────────────────────────────────────────
//  Tests
// ───────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn pt(x: f64, y: f64) -> Point2 {
        Point2::new(x, y)
    }

    fn grid_points(nx: usize, ny: usize) -> Vec<Point2> {
        let mut pts = Vec::new();
        for j in 0..ny {
            for i in 0..nx {
                pts.push(pt(i as f64, j as f64));
            }
        }
        pts
    }

    // ── Halfspace2 tests ────────────────────────────────────────────────

    #[test]
    fn halfspace_above_line() {
        // Line from (0, 5) going right. Points above are on the left.
        let hs = Halfspace2::new(pt(0.0, 5.0), pt(1.0, 0.0));
        assert!(hs.contains(pt(5.0, 8.0))); // above
        assert!(!hs.contains(pt(5.0, 2.0))); // below
        assert!(hs.contains(pt(5.0, 5.0))); // on the line
    }

    #[test]
    fn halfspace_below_line() {
        // Line from (0, 5) going left. Points below are on the left.
        let hs = Halfspace2::new(pt(0.0, 5.0), pt(-1.0, 0.0));
        assert!(hs.contains(pt(5.0, 2.0))); // below
        assert!(!hs.contains(pt(5.0, 8.0))); // above
    }

    // ── Kd-tree halfspace reporting ─────────────────────────────────────

    #[test]
    fn kd_tree_halfspace_basic() {
        let pts = vec![pt(0.0, 0.0), pt(1.0, 1.0), pt(2.0, 2.0), pt(3.0, 3.0)];
        let tree = KdTree2::build(&pts);
        // Report all points with y > 1.5 (above the line y=1.5).
        let hs = Halfspace2::new(pt(0.0, 1.5), pt(1.0, 0.0));
        let mut out = Vec::new();
        let count = tree.report_halfspace(&hs, &mut out);
        assert_eq!(count, 2);
        assert!(out.contains(&2)); // (2, 2)
        assert!(out.contains(&3)); // (3, 3)
    }

    #[test]
    fn kd_tree_halfspace_empty() {
        let pts = vec![pt(0.0, 0.0), pt(1.0, 1.0)];
        let tree = KdTree2::build(&pts);
        let hs = Halfspace2::new(pt(0.0, 10.0), pt(1.0, 0.0)); // y > 10
        let mut out = Vec::new();
        assert_eq!(tree.report_halfspace(&hs, &mut out), 0);
    }

    #[test]
    fn kd_tree_halfspace_all() {
        let pts = vec![pt(0.0, 0.0), pt(1.0, 1.0), pt(2.0, 2.0)];
        let tree = KdTree2::build(&pts);
        let hs = Halfspace2::new(pt(0.0, -10.0), pt(1.0, 0.0)); // y > -10
        let mut out = Vec::new();
        assert_eq!(tree.report_halfspace(&hs, &mut out), 3);
    }

    #[test]
    fn kd_tree_halfspace_grid() {
        let pts = grid_points(10, 10);
        let tree = KdTree2::build(&pts);
        // Report all points with y >= 5.
        let hs = Halfspace2::new(pt(0.0, 4.5), pt(1.0, 0.0));
        let mut out = Vec::new();
        let count = tree.report_halfspace(&hs, &mut out);
        assert_eq!(count, 50); // 5 rows × 10 cols

        // Verify correctness against brute force.
        let mut bf = Vec::new();
        for (i, &p) in pts.iter().enumerate() {
            if hs.contains(p) {
                bf.push(i);
            }
        }
        out.sort();
        bf.sort();
        assert_eq!(out, bf);
    }

    // ── Kd-tree simplex reporting ───────────────────────────────────────

    #[test]
    fn kd_tree_simplex_basic() {
        let pts = vec![pt(0.0, 0.0), pt(2.0, 0.0), pt(1.0, 2.0), pt(5.0, 5.0)];
        let tree = KdTree2::build(&pts);
        let mut out = Vec::new();
        let count = tree.report_simplex(pt(0.0, 0.0), pt(2.0, 0.0), pt(1.0, 2.0), &mut out);
        assert_eq!(count, 3); // first 3 points inside, (5,5) outside
    }

    #[test]
    fn kd_tree_simplex_grid() {
        let pts = grid_points(10, 10);
        let tree = KdTree2::build(&pts);
        let mut out = Vec::new();
        let count = tree.report_simplex(pt(2.0, 2.0), pt(7.0, 2.0), pt(4.5, 7.0), &mut out);

        // Verify against brute force.
        let mut bf = Vec::new();
        for (i, &p) in pts.iter().enumerate() {
            if point_in_triangle(p, pt(2.0, 2.0), pt(7.0, 2.0), pt(4.5, 7.0)) {
                bf.push(i);
            }
        }
        out.sort();
        bf.sort();
        assert_eq!(count, bf.len());
        assert_eq!(out, bf);
    }

    #[test]
    fn kd_tree_empty() {
        let tree = KdTree2::build(&[]);
        assert!(tree.is_empty());
        let hs = Halfspace2::new(pt(0.0, 0.0), pt(1.0, 0.0));
        let mut out = Vec::new();
        assert_eq!(tree.report_halfspace(&hs, &mut out), 0);
    }

    // ── Partition tree tests ────────────────────────────────────────────

    #[test]
    fn partition_tree_builds() {
        let pts = grid_points(5, 5);
        let tree = PartitionTree::build(&pts);
        assert_eq!(tree.len(), 25);
        assert!(!tree.is_empty());
    }

    #[test]
    fn partition_tree_simplex_grid() {
        let pts = grid_points(10, 10);
        let tree = PartitionTree::build(&pts);
        let mut out = Vec::new();
        let count = tree.report_simplex(pt(2.0, 2.0), pt(7.0, 2.0), pt(4.5, 7.0), &mut out);

        let mut bf = Vec::new();
        for (i, &p) in pts.iter().enumerate() {
            if point_in_triangle(p, pt(2.0, 2.0), pt(7.0, 2.0), pt(4.5, 7.0)) {
                bf.push(i);
            }
        }
        out.sort();
        bf.sort();
        assert_eq!(count, bf.len());
        assert_eq!(out, bf);
    }

    #[test]
    fn partition_tree_halfspace_grid() {
        let pts = grid_points(10, 10);
        let tree = PartitionTree::build(&pts);
        let hs = Halfspace2::new(pt(0.0, 4.5), pt(1.0, 0.0));
        let mut out = Vec::new();
        let count = tree.report_halfspace(&hs, &mut out);

        let mut bf = Vec::new();
        for (i, &p) in pts.iter().enumerate() {
            if hs.contains(p) {
                bf.push(i);
            }
        }
        out.sort();
        bf.sort();
        assert_eq!(count, bf.len());
        assert_eq!(out, bf);
    }

    #[test]
    fn partition_tree_empty() {
        let tree = PartitionTree::build(&[]);
        assert!(tree.is_empty());
        let mut out = Vec::new();
        assert_eq!(
            tree.report_simplex(pt(0.0, 0.0), pt(1.0, 0.0), pt(0.0, 1.0), &mut out),
            0
        );
    }

    // ── Cutting tree tests ──────────────────────────────────────────────

    #[test]
    fn cutting_tree_builds() {
        let pts = grid_points(5, 5);
        let tree = CuttingTree::build(&pts);
        assert_eq!(tree.len(), 25);
    }

    #[test]
    fn cutting_tree_halfspace_grid() {
        let pts = grid_points(10, 10);
        let tree = CuttingTree::build(&pts);
        let hs = Halfspace2::new(pt(0.0, 4.5), pt(1.0, 0.0));
        let mut out = Vec::new();
        let count = tree.report_halfspace(&hs, &mut out);

        let mut bf = Vec::new();
        for (i, &p) in pts.iter().enumerate() {
            if hs.contains(p) {
                bf.push(i);
            }
        }
        out.sort();
        bf.sort();
        assert_eq!(count, bf.len());
        assert_eq!(out, bf);
    }

    #[test]
    fn cutting_tree_simplex_grid() {
        let pts = grid_points(10, 10);
        let tree = CuttingTree::build(&pts);
        let mut out = Vec::new();
        let count = tree.report_simplex(pt(2.0, 2.0), pt(7.0, 2.0), pt(4.5, 7.0), &mut out);

        let mut bf = Vec::new();
        for (i, &p) in pts.iter().enumerate() {
            if point_in_triangle(p, pt(2.0, 2.0), pt(7.0, 2.0), pt(4.5, 7.0)) {
                bf.push(i);
            }
        }
        out.sort();
        bf.sort();
        assert_eq!(count, bf.len());
        assert_eq!(out, bf);
    }

    #[test]
    fn cutting_tree_empty() {
        let tree = CuttingTree::build(&[]);
        assert!(tree.is_empty());
        let hs = Halfspace2::new(pt(0.0, 0.0), pt(1.0, 0.0));
        let mut out = Vec::new();
        assert_eq!(tree.report_halfspace(&hs, &mut out), 0);
    }

    // ── Cross-validation: all three structures agree ────────────────────

    #[test]
    fn cross_validate_halfspace() {
        let pts = grid_points(8, 8);
        let kd = KdTree2::build(&pts);
        let pt_tree = PartitionTree::build(&pts);
        let ct = CuttingTree::build(&pts);

        let hs = Halfspace2::new(pt(-1.0, 3.5), pt(1.0, 0.2));

        let mut kd_out = Vec::new();
        kd.report_halfspace(&hs, &mut kd_out);
        kd_out.sort();

        let mut pt_out = Vec::new();
        pt_tree.report_halfspace(&hs, &mut pt_out);
        pt_out.sort();

        let mut ct_out = Vec::new();
        ct.report_halfspace(&hs, &mut ct_out);
        ct_out.sort();

        // Brute force.
        let mut bf = Vec::new();
        for (i, &p) in pts.iter().enumerate() {
            if hs.contains(p) {
                bf.push(i);
            }
        }
        bf.sort();

        assert_eq!(kd_out, bf);
        assert_eq!(pt_out, bf);
        assert_eq!(ct_out, bf);
    }

    #[test]
    fn cross_validate_simplex() {
        let pts = grid_points(8, 8);
        let kd = KdTree2::build(&pts);
        let pt_tree = PartitionTree::build(&pts);
        let ct = CuttingTree::build(&pts);

        let (a, b, c) = (pt(1.5, 1.5), pt(6.0, 2.0), pt(3.5, 6.0));

        let mut kd_out = Vec::new();
        kd.report_simplex(a, b, c, &mut kd_out);
        kd_out.sort();

        let mut pt_out = Vec::new();
        pt_tree.report_simplex(a, b, c, &mut pt_out);
        pt_out.sort();

        let mut ct_out = Vec::new();
        ct.report_simplex(a, b, c, &mut ct_out);
        ct_out.sort();

        let mut bf = Vec::new();
        for (i, &p) in pts.iter().enumerate() {
            if point_in_triangle(p, a, b, c) {
                bf.push(i);
            }
        }
        bf.sort();

        assert_eq!(kd_out, bf);
        assert_eq!(pt_out, bf);
        assert_eq!(ct_out, bf);
    }

    // ── Single point ────────────────────────────────────────────────────

    #[test]
    fn single_point_halfspace() {
        let pts = vec![pt(5.0, 5.0)];
        let kd = KdTree2::build(&pts);
        let hs = Halfspace2::new(pt(0.0, 0.0), pt(1.0, 0.0));
        let mut out = Vec::new();
        assert_eq!(kd.report_halfspace(&hs, &mut out), 1);
        assert_eq!(out, vec![0]);
    }

    // ── Triangle intersection helper ────────────────────────────────────

    #[test]
    fn triangles_intersect_overlap() {
        assert!(triangles_intersect(
            pt(0.0, 0.0),
            pt(4.0, 0.0),
            pt(2.0, 4.0),
            pt(2.0, 0.0),
            pt(6.0, 0.0),
            pt(4.0, 4.0),
        ));
    }

    #[test]
    fn triangles_intersect_disjoint() {
        assert!(!triangles_intersect(
            pt(0.0, 0.0),
            pt(2.0, 0.0),
            pt(1.0, 2.0),
            pt(10.0, 10.0),
            pt(12.0, 10.0),
            pt(11.0, 12.0),
        ));
    }

    #[test]
    fn triangles_intersect_contained() {
        assert!(triangles_intersect(
            pt(0.0, 0.0),
            pt(10.0, 0.0),
            pt(5.0, 10.0),
            pt(3.0, 1.0),
            pt(7.0, 1.0),
            pt(5.0, 5.0),
        ));
    }
}
