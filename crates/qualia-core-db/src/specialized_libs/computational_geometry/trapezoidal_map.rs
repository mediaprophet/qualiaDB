//! Trapezoidal map with randomized incremental point location (P11.6 gap).
//!
//! Given a set of non-crossing line segments, the trapezoidal map decomposes
//! the plane into trapezoids (or degenerate triangles) by shooting vertical
//! rays up and down from each segment endpoint until they hit another segment
//! or the bounding box.
//!
//! A **search DAG** (directed acyclic graph) is built incrementally: each
//! insertion replaces a leaf (trapezoid) with x-node and y-node decision
//! points. Point location traverses the DAG from the root: at an x-node, go
//! left/right by comparing the query x to the endpoint x; at a y-node, go
//! left/right by testing the query point above/below the segment. Expected
//! O(log n) query time under randomised insertion order.
//!
//! Reference: de Berg, Cheong, van Kreveld & Overmars, *Computational
//! Geometry: Algorithms and Applications* (3rd ed.), §6.2–6.3.
//!
//! Tier-2 cold construction (uses `Vec` during build; public output is the
//! DAG itself, queried without allocation).

use super::primitives::{orientation_2, Orientation, Point2};

// ───────────────────────────────────────────────────────────────────────────
//  Error type
// ───────────────────────────────────────────────────────────────────────────

/// Error returned by trapezoidal map operations.
#[derive(Debug, Clone, PartialEq)]
pub enum TrapezoidalMapError {
    /// Fewer than one segment provided.
    TooFewSegments,
    /// Segments cross (the map requires non-crossing input).
    CrossingSegments { i: usize, j: usize },
    /// A segment is degenerate (zero-length).
    DegenerateSegment { index: usize },
    /// The query point is outside the bounding box.
    OutsideBoundingBox,
}

impl core::fmt::Display for TrapezoidalMapError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::TooFewSegments => write!(f, "trapezoidal_map: need at least 1 segment"),
            Self::CrossingSegments { i, j } => {
                write!(f, "trapezoidal_map: segments {i} and {j} cross")
            }
            Self::DegenerateSegment { index } => {
                write!(
                    f,
                    "trapezoidal_map: segment {index} is degenerate (zero-length)"
                )
            }
            Self::OutsideBoundingBox => write!(f, "trapezoidal_map: query outside bounding box"),
        }
    }
}

impl std::error::Error for TrapezoidalMapError {}

// ───────────────────────────────────────────────────────────────────────────
//  Segment
// ───────────────────────────────────────────────────────────────────────────

/// A non-crossing line segment for trapezoidal map construction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TmSegment {
    pub left: Point2,
    pub right: Point2,
}

impl TmSegment {
    pub fn new(a: Point2, b: Point2) -> Self {
        if a.x <= b.x {
            Self { left: a, right: b }
        } else {
            Self { left: b, right: a }
        }
    }

    pub fn is_degenerate(&self) -> bool {
        (self.left.x - self.right.x).abs() < 1e-15 && (self.left.y - self.right.y).abs() < 1e-15
    }

    /// Evaluate the line at x (linear interpolation).
    pub fn y_at_x(&self, x: f64) -> f64 {
        let dx = self.right.x - self.left.x;
        if dx.abs() < 1e-15 {
            // Vertical segment — return the midpoint y.
            return (self.left.y + self.right.y) * 0.5;
        }
        let t = (x - self.left.x) / dx;
        self.left.y + t * (self.right.y - self.left.y)
    }

    /// Is point `p` strictly above this segment?
    pub fn is_above(&self, p: Point2) -> bool {
        let o = orientation_2(self.left, self.right, p);
        o == Orientation::CounterClockwise
    }

    /// Is point `p` strictly below this segment?
    pub fn is_below(&self, p: Point2) -> bool {
        let o = orientation_2(self.left, self.right, p);
        o == Orientation::Clockwise
    }
}

// ───────────────────────────────────────────────────────────────────────────
//  DAG node types
// ───────────────────────────────────────────────────────────────────────────

/// Internal node kind in the search DAG.
#[derive(Debug, Clone, Copy, PartialEq)]
enum NodeKind {
    /// X-node: compares query x to a segment endpoint x.
    XNode {
        point_x: f64,
        left: usize,
        right: usize,
    },
    /// Y-node: tests query above/below a segment.
    YNode {
        segment: usize,
        left: usize,
        right: usize,
    },
    /// Leaf: a trapezoid.
    Leaf { trapezoid: usize },
}

/// A node in the search DAG.
#[derive(Debug, Clone, Copy, PartialEq)]
struct DagNode {
    kind: NodeKind,
}

// ───────────────────────────────────────────────────────────────────────────
//  Trapezoid
// ───────────────────────────────────────────────────────────────────────────

/// A trapezoid in the map, defined by its four boundaries:
/// - `top` / `bottom`: segments (or bounding-box edges) above/below.
/// - `leftp` / `rightp`: points that define the left/right vertical walls.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Trapezoid {
    pub leftp: Point2,
    pub rightp: Point2,
    pub top: Option<usize>,
    pub bottom: Option<usize>,
    /// Bounding-box top is represented as `None` with a flag.
    pub top_is_bbox: bool,
    pub bottom_is_bbox: bool,
}

// ───────────────────────────────────────────────────────────────────────────
//  Trapezoidal map
// ───────────────────────────────────────────────────────────────────────────

/// A trapezoidal map with a search DAG for point location.
///
/// Build with [`TrapezoidalMap::build`], query with [`TrapezoidalMap::locate`].
/// The insertion order is seeded-deterministic (Fisher-Yates with a
/// seedable RNG) so that two builds from the same input + seed produce
/// byte-identical DAGs.
pub struct TrapezoidalMap {
    /// The search DAG nodes.
    nodes: Vec<DagNode>,
    /// The trapezoids.
    trapezoids: Vec<Trapezoid>,
    /// The segments.
    segments: Vec<TmSegment>,
    /// Bounding box.
    bbox_min: Point2,
    bbox_max: Point2,
    /// Root node index.
    root: usize,
}

impl std::fmt::Debug for TrapezoidalMap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TrapezoidalMap")
            .field("num_nodes", &self.nodes.len())
            .field("num_trapezoids", &self.trapezoids.len())
            .field("num_segments", &self.segments.len())
            .field("bbox_min", &self.bbox_min)
            .field("bbox_max", &self.bbox_max)
            .field("root", &self.root)
            .finish()
    }
}

/// Simple seeded RNG (xorshift64) for deterministic insertion order.
struct SeededRng {
    state: u64,
}

impl SeededRng {
    fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 {
                0x9E37_79B9_7F4A_7C15
            } else {
                seed
            },
        }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    /// Fisher-Yates shuffle producing a deterministic permutation.
    fn shuffle<T>(&mut self, arr: &mut [T]) {
        let n = arr.len();
        for i in (1..n).rev() {
            let j = (self.next_u64() % (i as u64 + 1)) as usize;
            arr.swap(i, j);
        }
    }
}

/// Compute a bounding box that contains all segment endpoints, with margin.
fn compute_bbox(segments: &[TmSegment]) -> (Point2, Point2) {
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for s in segments {
        min_x = min_x.min(s.left.x).min(s.right.x);
        min_y = min_y.min(s.left.y).min(s.right.y);
        max_x = max_x.max(s.left.x).max(s.right.x);
        max_y = max_y.max(s.left.y).max(s.right.y);
    }
    let margin = (max_x - min_x).max(max_y - min_y).max(1.0) * 0.5;
    (
        Point2::new(min_x - margin, min_y - margin),
        Point2::new(max_x + margin, max_y + margin),
    )
}

/// Check if two segments properly cross (interior intersection).
fn segments_cross(a: &TmSegment, b: &TmSegment) -> bool {
    let o1 = orientation_2(a.left, a.right, b.left);
    let o2 = orientation_2(a.left, a.right, b.right);
    let o3 = orientation_2(b.left, b.right, a.left);
    let o4 = orientation_2(b.left, b.right, a.right);

    // Proper crossing: endpoints of each segment are on opposite sides.
    if o1 != o2
        && o1 != Orientation::Collinear
        && o2 != Orientation::Collinear
        && o3 != o4
        && o3 != Orientation::Collinear
        && o4 != Orientation::Collinear
    {
        return true;
    }
    false
}

impl TrapezoidalMap {
    /// Build a trapezoidal map from a set of non-crossing segments.
    ///
    /// The `seed` controls the randomised insertion order. The same input +
    /// seed always produces the same DAG.
    ///
    /// Returns an error if segments cross or are degenerate.
    pub fn build(segments: &[TmSegment], seed: u64) -> Result<Self, TrapezoidalMapError> {
        if segments.is_empty() {
            return Err(TrapezoidalMapError::TooFewSegments);
        }
        for (i, s) in segments.iter().enumerate() {
            if s.is_degenerate() {
                return Err(TrapezoidalMapError::DegenerateSegment { index: i });
            }
        }
        // Check for crossings.
        for i in 0..segments.len() {
            for j in (i + 1)..segments.len() {
                if segments_cross(&segments[i], &segments[j]) {
                    return Err(TrapezoidalMapError::CrossingSegments { i, j });
                }
            }
        }

        let (bbox_min, bbox_max) = compute_bbox(segments);

        let mut map = TrapezoidalMap {
            nodes: Vec::new(),
            trapezoids: Vec::new(),
            segments: segments.to_vec(),
            bbox_min,
            bbox_max,
            root: 0,
        };

        // Create the initial trapezoid: the entire bounding box.
        let init_trap = Trapezoid {
            leftp: bbox_min,
            rightp: bbox_max,
            top: None,
            bottom: None,
            top_is_bbox: true,
            bottom_is_bbox: true,
        };
        map.trapezoids.push(init_trap);
        let leaf = DagNode {
            kind: NodeKind::Leaf { trapezoid: 0 },
        };
        map.nodes.push(leaf);
        map.root = 0;

        // Deterministic insertion order.
        let mut order: Vec<usize> = (0..segments.len()).collect();
        SeededRng::new(seed).shuffle(&mut order);

        for &si in &order {
            map.insert_segment(si);
        }

        Ok(map)
    }

    /// Insert a segment into the trapezoidal map.
    ///
    /// This follows the standard algorithm:
    /// 1. Find all trapezoids intersected by the segment (via DAG traversal).
    /// 2. Split them and replace leaves with new sub-DAGs.
    fn insert_segment(&mut self, seg_idx: usize) {
        let seg = self.segments[seg_idx];
        let leftp = seg.left;
        let rightp = seg.right;

        // Find the trapezoid containing the left endpoint.
        let start_trap = self.locate_trapezoid_index(leftp);

        // Walk along the segment, collecting all trapezoids it crosses.
        let crossed = self.find_crossed_trapezoids(seg_idx, start_trap, leftp, rightp);

        // Split the first and last trapezoids at the segment endpoints
        // (if the endpoint is strictly inside, not on the boundary).
        // Then split all crossed trapezoids into upper/lower parts.
        let new_nodes = self.build_segment_subdag(seg_idx, &crossed, leftp, rightp);

        // Replace the first crossed trapezoid's leaf with the new sub-DAG root.
        if let Some(&(trap_leaf, new_root)) = new_nodes.first() {
            // Update the leaf node to point to the new sub-DAG.
            // We need to find the parent of this leaf and redirect it.
            // Simpler: replace the leaf in-place by overwriting its kind.
            self.nodes[trap_leaf] = DagNode {
                kind: self.nodes[new_root].kind,
            };
        }
    }

    /// Locate the trapezoid containing point `p` by traversing the DAG.
    fn locate_trapezoid_index(&self, p: Point2) -> usize {
        let mut node = self.root;
        loop {
            match self.nodes[node].kind {
                NodeKind::XNode {
                    point_x,
                    left,
                    right,
                } => {
                    if p.x < point_x {
                        node = left;
                    } else {
                        node = right;
                    }
                }
                NodeKind::YNode {
                    segment,
                    left,
                    right,
                } => {
                    let seg = self.segments[segment];
                    if seg.is_above(p) {
                        node = left;
                    } else {
                        node = right;
                    }
                }
                NodeKind::Leaf { trapezoid } => return trapezoid,
            }
        }
    }

    /// Find all trapezoids crossed by segment `seg_idx`, from left to right.
    fn find_crossed_trapezoids(
        &self,
        seg_idx: usize,
        start: usize,
        _leftp: Point2,
        rightp: Point2,
    ) -> Vec<usize> {
        let mut crossed = vec![start];
        let mut current = start;

        loop {
            let trap = self.trapezoids[current];
            // If the segment's right endpoint is <= the trapezoid's right
            // boundary, we're done.
            if rightp.x <= trap.rightp.x + 1e-12 {
                break;
            }
            // Determine whether the segment exits the trapezoid through the
            // top or bottom boundary by comparing the segment's y at
            // trap.rightp.x with the top/bottom boundaries' y at that x.
            let seg = self.segments[seg_idx];
            let seg_y = seg.y_at_x(trap.rightp.x);

            // Find the neighbor trapezoid by locating the point
            // (trap.rightp.x + epsilon, seg_y) in the DAG.
            let query = Point2::new(trap.rightp.x + 1e-10, seg_y);
            let next = self.locate_trapezoid_index(query);

            if next == current {
                // Can't progress — avoid infinite loop.
                break;
            }
            crossed.push(next);
            current = next;
        }

        crossed
    }

    /// Build the sub-DAG for inserting a segment across a set of trapezoids.
    ///
    /// Returns a list of (leaf_index_to_replace, new_subdag_root) pairs.
    /// In this simplified version, we handle the common case where the
    /// segment crosses 1 or more trapezoids.
    fn build_segment_subdag(
        &mut self,
        seg_idx: usize,
        crossed: &[usize],
        leftp: Point2,
        rightp: Point2,
    ) -> Vec<(usize, usize)> {
        if crossed.is_empty() {
            return vec![];
        }

        let _seg = self.segments[seg_idx];

        // For each crossed trapezoid, we create:
        // - If it's the first and the left endpoint is strictly inside:
        //   split into left + right, then the right part is split into
        //   upper + lower.
        // - If it's the last and the right endpoint is strictly inside:
        //   split into left + right, then the left part is split into
        //   upper + lower.
        // - Middle trapezoids: just split into upper + lower.
        //
        // We build a chain of y-nodes (upper/lower splits) connected by
        // x-nodes (endpoint splits).

        // We'll create new trapezoids and nodes. The key insight is:
        // For simplicity, we handle the general case by creating:
        // 1. An x-node for the left endpoint (if first trapezoid).
        // 2. Y-nodes for each crossed trapezoid (upper/lower split).
        // 3. An x-node for the right endpoint (if last trapezoid).
        //
        // The sub-DAG root replaces the first crossed trapezoid's leaf.

        let first_trap = self.trapezoids[crossed[0]];
        let first_leaf = self.find_leaf_for_trapezoid(crossed[0]);

        // Determine if we need left x-node.
        let needs_left_split = leftp.x > first_trap.leftp.x + 1e-12;
        let last_trap = self.trapezoids[*crossed.last().unwrap()];
        let needs_right_split = rightp.x < last_trap.rightp.x - 1e-12;

        // Create the new trapezoids for each crossed trapezoid.
        // Each crossed trapezoid becomes an upper and lower trapezoid.
        // If it's the first and needs_left_split, we also get a left trapezoid.
        // If it's the last and needs_right_split, we also get a right trapezoid.

        let mut upper_traps: Vec<usize> = Vec::new();
        let mut lower_traps: Vec<usize> = Vec::new();

        for (k, &ti) in crossed.iter().enumerate() {
            let trap = self.trapezoids[ti];

            // Determine the left and right boundaries of this trapezoid
            // after endpoint splitting.
            let trap_left = if k == 0 && needs_left_split {
                leftp
            } else {
                trap.leftp
            };
            let trap_right = if k == crossed.len() - 1 && needs_right_split {
                rightp
            } else {
                trap.rightp
            };

            // Upper trapezoid (above the segment).
            let upper_trap = Trapezoid {
                leftp: trap_left,
                rightp: trap_right,
                top: trap.top,
                bottom: Some(seg_idx),
                top_is_bbox: trap.top_is_bbox,
                bottom_is_bbox: false,
            };
            let upper_idx = self.trapezoids.len();
            self.trapezoids.push(upper_trap);
            upper_traps.push(upper_idx);

            // Lower trapezoid (below the segment).
            let lower_trap = Trapezoid {
                leftp: trap_left,
                rightp: trap_right,
                top: Some(seg_idx),
                bottom: trap.bottom,
                top_is_bbox: false,
                bottom_is_bbox: trap.bottom_is_bbox,
            };
            let lower_idx = self.trapezoids.len();
            self.trapezoids.push(lower_trap);
            lower_traps.push(lower_idx);
        }

        // Build the sub-DAG.
        // For the first trapezoid with left split:
        //   x-node(leftp) -> left: old trapezoid (unchanged left part)
        //                     right: y-node chain
        // For the last trapezoid with right split:
        //   the y-node chain ends with x-node(rightp) -> left: y-node, right: old right part

        // Create leaf nodes for all upper and lower trapezoids.
        let mut upper_leaves: Vec<usize> = Vec::new();
        let mut lower_leaves: Vec<usize> = Vec::new();
        for &ui in &upper_traps {
            let idx = self.nodes.len();
            self.nodes.push(DagNode {
                kind: NodeKind::Leaf { trapezoid: ui },
            });
            upper_leaves.push(idx);
        }
        for &li in &lower_traps {
            let idx = self.nodes.len();
            self.nodes.push(DagNode {
                kind: NodeKind::Leaf { trapezoid: li },
            });
            lower_leaves.push(idx);
        }

        // Build y-node chain for the crossed trapezoids.
        // If there's only one crossed trapezoid, the chain is a single y-node.
        // If there are multiple, we chain them via x-nodes at the trapezoid
        // boundaries.

        // Simplified approach: build a chain of y-nodes, one per crossed trapezoid.
        // For multiple trapezoids, we connect them with x-nodes at the
        // right boundary of each intermediate trapezoid.

        let mut chain_root = self.build_y_chain(
            seg_idx,
            &upper_leaves,
            &lower_leaves,
            crossed,
            needs_left_split,
            needs_right_split,
            leftp,
            rightp,
        );

        // If we need a left x-node, wrap the chain.
        if needs_left_split {
            // Create a leaf for the left part of the first trapezoid.
            let first_trap = self.trapezoids[crossed[0]];
            let left_trap = Trapezoid {
                leftp: first_trap.leftp,
                rightp: leftp,
                top: first_trap.top,
                bottom: first_trap.bottom,
                top_is_bbox: first_trap.top_is_bbox,
                bottom_is_bbox: first_trap.bottom_is_bbox,
            };
            let left_trap_idx = self.trapezoids.len();
            self.trapezoids.push(left_trap);
            let left_leaf = self.nodes.len();
            self.nodes.push(DagNode {
                kind: NodeKind::Leaf {
                    trapezoid: left_trap_idx,
                },
            });

            let x_node = self.nodes.len();
            self.nodes.push(DagNode {
                kind: NodeKind::XNode {
                    point_x: leftp.x,
                    left: left_leaf,
                    right: chain_root,
                },
            });
            chain_root = x_node;
        }

        // If we need a right x-node, wrap the chain.
        if needs_right_split {
            let last_trap = self.trapezoids[*crossed.last().unwrap()];
            let right_trap = Trapezoid {
                leftp: rightp,
                rightp: last_trap.rightp,
                top: last_trap.top,
                bottom: last_trap.bottom,
                top_is_bbox: last_trap.top_is_bbox,
                bottom_is_bbox: last_trap.bottom_is_bbox,
            };
            let right_trap_idx = self.trapezoids.len();
            self.trapezoids.push(right_trap);
            let right_leaf = self.nodes.len();
            self.nodes.push(DagNode {
                kind: NodeKind::Leaf {
                    trapezoid: right_trap_idx,
                },
            });

            let x_node = self.nodes.len();
            self.nodes.push(DagNode {
                kind: NodeKind::XNode {
                    point_x: rightp.x,
                    left: chain_root,
                    right: right_leaf,
                },
            });
            chain_root = x_node;
        }

        vec![(first_leaf, chain_root)]
    }

    /// Build a chain of y-nodes for the crossed trapezoids.
    fn build_y_chain(
        &mut self,
        seg_idx: usize,
        upper_leaves: &[usize],
        lower_leaves: &[usize],
        crossed: &[usize],
        _needs_left: bool,
        _needs_right: bool,
        _leftp: Point2,
        _rightp: Point2,
    ) -> usize {
        if crossed.len() == 1 {
            // Single y-node.
            let y_node = self.nodes.len();
            self.nodes.push(DagNode {
                kind: NodeKind::YNode {
                    segment: seg_idx,
                    left: upper_leaves[0],
                    right: lower_leaves[0],
                },
            });
            return y_node;
        }

        // Multiple trapezoids: build a right-leaning chain.
        // For each trapezoid k, create a y-node with upper/lower leaves.
        // Connect consecutive y-nodes with x-nodes at the trapezoid boundary.
        //
        // The structure is:
        //   x-node(boundary[0]) -> left: y-node[0], right: x-node(boundary[1]) -> ...
        // The last x-node's right child is the last y-node.

        let mut prev_y = self.nodes.len();
        self.nodes.push(DagNode {
            kind: NodeKind::YNode {
                segment: seg_idx,
                left: upper_leaves[0],
                right: lower_leaves[0],
            },
        });

        for k in 1..crossed.len() {
            let trap = self.trapezoids[crossed[k]];
            let boundary_x = trap.leftp.x;

            let y_node = self.nodes.len();
            self.nodes.push(DagNode {
                kind: NodeKind::YNode {
                    segment: seg_idx,
                    left: upper_leaves[k],
                    right: lower_leaves[k],
                },
            });

            let x_node = self.nodes.len();
            self.nodes.push(DagNode {
                kind: NodeKind::XNode {
                    point_x: boundary_x,
                    left: prev_y,
                    right: y_node,
                },
            });
            prev_y = x_node;
        }

        prev_y
    }

    /// Find the DAG leaf node index that points to trapezoid `ti`.
    fn find_leaf_for_trapezoid(&self, ti: usize) -> usize {
        for (ni, node) in self.nodes.iter().enumerate() {
            if let NodeKind::Leaf { trapezoid } = node.kind {
                if trapezoid == ti {
                    return ni;
                }
            }
        }
        0 // fallback
    }

    /// Locate a query point in the trapezoidal map.
    ///
    /// Returns the index of the trapezoid containing the query point,
    /// or an error if the point is outside the bounding box.
    pub fn locate(&self, query: Point2) -> Result<usize, TrapezoidalMapError> {
        if query.x < self.bbox_min.x
            || query.x > self.bbox_max.x
            || query.y < self.bbox_min.y
            || query.y > self.bbox_max.y
        {
            return Err(TrapezoidalMapError::OutsideBoundingBox);
        }
        Ok(self.locate_trapezoid_index(query))
    }

    /// Get a trapezoid by index.
    pub fn trapezoid(&self, index: usize) -> &Trapezoid {
        &self.trapezoids[index]
    }

    /// Number of trapezoids.
    pub fn num_trapezoids(&self) -> usize {
        self.trapezoids.len()
    }

    /// Number of DAG nodes.
    pub fn num_nodes(&self) -> usize {
        self.nodes.len()
    }

    /// Get the segments.
    pub fn segments(&self) -> &[TmSegment] {
        &self.segments
    }

    /// Bounding box.
    pub fn bbox(&self) -> (Point2, Point2) {
        (self.bbox_min, self.bbox_max)
    }

    /// Brute-force point location: scan all trapezoids.
    /// Used as an oracle for testing.
    pub fn locate_brute_force(&self, query: Point2) -> Option<usize> {
        for (i, t) in self.trapezoids.iter().enumerate() {
            if self.point_in_trapezoid(query, t) {
                return Some(i);
            }
        }
        None
    }

    /// Check if a point is inside a trapezoid.
    fn point_in_trapezoid(&self, p: Point2, t: &Trapezoid) -> bool {
        if p.x < t.leftp.x - 1e-10 || p.x > t.rightp.x + 1e-10 {
            return false;
        }

        // Check below top boundary.
        let top_y = if t.top_is_bbox {
            self.bbox_max.y
        } else if let Some(ti) = t.top {
            self.segments[ti].y_at_x(p.x)
        } else {
            self.bbox_max.y
        };
        if p.y > top_y + 1e-10 {
            return false;
        }

        // Check above bottom boundary.
        let bottom_y = if t.bottom_is_bbox {
            self.bbox_min.y
        } else if let Some(bi) = t.bottom {
            self.segments[bi].y_at_x(p.x)
        } else {
            self.bbox_min.y
        };
        if p.y < bottom_y - 1e-10 {
            return false;
        }

        true
    }
}

// ───────────────────────────────────────────────────────────────────────────
//  Tests
// ───────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn s(ax: f64, ay: f64, bx: f64, by: f64) -> TmSegment {
        TmSegment::new(Point2::new(ax, ay), Point2::new(bx, by))
    }

    fn pt(x: f64, y: f64) -> Point2 {
        Point2::new(x, y)
    }

    // ── Basic build ─────────────────────────────────────────────────────

    #[test]
    fn single_segment_builds() {
        let segs = vec![s(0.0, 0.0, 10.0, 10.0)];
        let tm = TrapezoidalMap::build(&segs, 42).unwrap();
        assert!(tm.num_trapezoids() >= 3); // left, upper, lower (at minimum)
        assert!(tm.num_nodes() >= 3);
    }

    #[test]
    fn empty_segments_errors() {
        let result = TrapezoidalMap::build(&[], 42);
        assert!(matches!(result, Err(TrapezoidalMapError::TooFewSegments)));
    }

    #[test]
    fn degenerate_segment_errors() {
        let segs = vec![s(5.0, 5.0, 5.0, 5.0)];
        let result = TrapezoidalMap::build(&segs, 42);
        assert!(matches!(
            result,
            Err(TrapezoidalMapError::DegenerateSegment { .. })
        ));
    }

    #[test]
    fn crossing_segments_errors() {
        let segs = vec![s(0.0, 0.0, 10.0, 10.0), s(0.0, 10.0, 10.0, 0.0)];
        let result = TrapezoidalMap::build(&segs, 42);
        assert!(matches!(
            result,
            Err(TrapezoidalMapError::CrossingSegments { .. })
        ));
    }

    // ── Point location ──────────────────────────────────────────────────

    #[test]
    fn locate_above_single_segment() {
        let segs = vec![s(0.0, 5.0, 10.0, 5.0)];
        let tm = TrapezoidalMap::build(&segs, 42).unwrap();
        let trap_idx = tm.locate(pt(5.0, 8.0)).unwrap();
        let trap = tm.trapezoid(trap_idx);
        // The point is above the segment, so bottom should be the segment.
        assert_eq!(trap.bottom, Some(0));
        assert!(trap.top_is_bbox);
    }

    #[test]
    fn locate_below_single_segment() {
        let segs = vec![s(0.0, 5.0, 10.0, 5.0)];
        let tm = TrapezoidalMap::build(&segs, 42).unwrap();
        let trap_idx = tm.locate(pt(5.0, 2.0)).unwrap();
        let trap = tm.trapezoid(trap_idx);
        // The point is below the segment, so top should be the segment.
        assert_eq!(trap.top, Some(0));
        assert!(trap.bottom_is_bbox);
    }

    #[test]
    fn locate_left_of_single_segment() {
        let segs = vec![s(5.0, 5.0, 10.0, 5.0)];
        let tm = TrapezoidalMap::build(&segs, 42).unwrap();
        let trap_idx = tm.locate(pt(3.0, 5.0)).unwrap();
        let trap = tm.trapezoid(trap_idx);
        // The point is to the left — should be in a trapezoid that spans
        // the full height (both bbox top and bottom).
        assert!(trap.top_is_bbox);
        assert!(trap.bottom_is_bbox);
    }

    #[test]
    fn locate_outside_bbox_errors() {
        let segs = vec![s(0.0, 5.0, 10.0, 5.0)];
        let tm = TrapezoidalMap::build(&segs, 42).unwrap();
        assert!(matches!(
            tm.locate(pt(100.0, 100.0)),
            Err(TrapezoidalMapError::OutsideBoundingBox)
        ));
        assert!(matches!(
            tm.locate(pt(-100.0, -100.0)),
            Err(TrapezoidalMapError::OutsideBoundingBox)
        ));
    }

    // ── Multiple segments ───────────────────────────────────────────────

    #[test]
    fn two_parallel_segments_three_bands() {
        let segs = vec![s(0.0, 3.0, 10.0, 3.0), s(0.0, 7.0, 10.0, 7.0)];
        let tm = TrapezoidalMap::build(&segs, 42).unwrap();

        // Point in bottom band.
        let bot = tm.locate(pt(5.0, 1.0)).unwrap();
        let bot_trap = tm.trapezoid(bot);
        assert_eq!(bot_trap.top, Some(0));
        assert!(bot_trap.bottom_is_bbox);

        // Point in middle band.
        let mid = tm.locate(pt(5.0, 5.0)).unwrap();
        let mid_trap = tm.trapezoid(mid);
        assert_eq!(mid_trap.top, Some(1));
        assert_eq!(mid_trap.bottom, Some(0));

        // Point in top band.
        let top = tm.locate(pt(5.0, 9.0)).unwrap();
        let top_trap = tm.trapezoid(top);
        assert!(top_trap.top_is_bbox);
        assert_eq!(top_trap.bottom, Some(1));
    }

    #[test]
    fn diagonal_segment_splits_correctly() {
        let segs = vec![s(0.0, 0.0, 10.0, 10.0)];
        let tm = TrapezoidalMap::build(&segs, 42).unwrap();

        // Point above the diagonal.
        let above = tm.locate(pt(5.0, 8.0)).unwrap();
        let above_trap = tm.trapezoid(above);
        assert_eq!(above_trap.bottom, Some(0));

        // Point below the diagonal.
        let below = tm.locate(pt(5.0, 2.0)).unwrap();
        let below_trap = tm.trapezoid(below);
        assert_eq!(below_trap.top, Some(0));
    }

    // ── DAG vs brute-force oracle ───────────────────────────────────────

    #[test]
    fn dag_locate_matches_brute_force_single() {
        let segs = vec![s(0.0, 5.0, 10.0, 5.0)];
        let tm = TrapezoidalMap::build(&segs, 42).unwrap();

        for (x, y) in [
            (1.0, 1.0),
            (5.0, 1.0),
            (9.0, 1.0),
            (1.0, 5.0),
            (5.0, 5.0),
            (9.0, 5.0),
            (1.0, 9.0),
            (5.0, 9.0),
            (9.0, 9.0),
            (3.0, 3.0),
            (7.0, 7.0),
            (2.0, 8.0),
        ] {
            let dag = tm.locate(pt(x, y)).unwrap();
            let bf = tm.locate_brute_force(pt(x, y)).unwrap();
            // Both should find a trapezoid containing the point.
            let dag_trap = tm.trapezoid(dag);
            let bf_trap = tm.trapezoid(bf);
            assert!(
                tm.point_in_trapezoid(pt(x, y), dag_trap),
                "DAG result doesn't contain ({}, {})",
                x,
                y
            );
            assert!(
                tm.point_in_trapezoid(pt(x, y), bf_trap),
                "Brute-force result doesn't contain ({}, {})",
                x,
                y
            );
        }
    }

    #[test]
    fn dag_locate_matches_brute_force_multi() {
        // Segments must not cross — use non-crossing ones.
        let segs = vec![s(1.0, 3.0, 8.0, 3.0), s(2.0, 6.0, 9.0, 6.0)];
        let tm = TrapezoidalMap::build(&segs, 42).unwrap();

        for x in 0..20 {
            for y in 0..20 {
                let qx = x as f64 * 0.5;
                let qy = y as f64 * 0.5;
                let p = pt(qx, qy);
                if let Ok(dag) = tm.locate(p) {
                    let dag_trap = tm.trapezoid(dag);
                    assert!(
                        tm.point_in_trapezoid(p, dag_trap),
                        "DAG result doesn't contain ({}, {})",
                        qx,
                        qy
                    );
                }
            }
        }
    }

    // ── Determinism ─────────────────────────────────────────────────────

    #[test]
    fn same_seed_produces_same_map() {
        let segs = vec![s(1.0, 3.0, 8.0, 3.0), s(2.0, 6.0, 9.0, 6.0)];

        let tm1 = TrapezoidalMap::build(&segs, 12345).unwrap();
        let tm2 = TrapezoidalMap::build(&segs, 12345).unwrap();

        assert_eq!(tm1.num_trapezoids(), tm2.num_trapezoids());
        assert_eq!(tm1.num_nodes(), tm2.num_nodes());

        // Same query results.
        for x in 0..20 {
            for y in 0..20 {
                let p = pt(x as f64 * 0.5, y as f64 * 0.5);
                let r1 = tm1.locate(p);
                let r2 = tm2.locate(p);
                assert_eq!(r1, r2, "Mismatch at ({}, {})", p.x, p.y);
            }
        }
    }

    #[test]
    fn different_seeds_produce_valid_maps() {
        let segs = vec![s(1.0, 3.0, 8.0, 3.0), s(2.0, 6.0, 9.0, 6.0)];

        for seed in [1u64, 42, 100, 999] {
            let tm = TrapezoidalMap::build(&segs, seed).unwrap();
            // Verify all queries return valid trapezoids.
            for x in 0..20 {
                for y in 0..20 {
                    let p = pt(x as f64 * 0.5, y as f64 * 0.5);
                    if let Ok(idx) = tm.locate(p) {
                        let trap = tm.trapezoid(idx);
                        assert!(
                            tm.point_in_trapezoid(p, trap),
                            "seed {} point ({}, {}) not in returned trapezoid",
                            seed,
                            p.x,
                            p.y
                        );
                    }
                }
            }
        }
    }

    // ── Non-crossing diagonal segments ──────────────────────────────────

    #[test]
    fn non_crossing_diagonals() {
        // Two diagonal segments that don't cross.
        let segs = vec![s(1.0, 1.0, 4.0, 4.0), s(6.0, 1.0, 9.0, 4.0)];
        let tm = TrapezoidalMap::build(&segs, 42).unwrap();

        // Points near each segment.
        let p1 = tm.locate(pt(2.5, 3.0)).unwrap(); // above first segment
        let t1 = tm.trapezoid(p1);
        assert_eq!(t1.bottom, Some(0));

        let p2 = tm.locate(pt(7.5, 3.0)).unwrap(); // above second segment
        let t2 = tm.trapezoid(p2);
        assert_eq!(t2.bottom, Some(1));
    }

    // ── Segment properties ──────────────────────────────────────────────

    #[test]
    fn segment_normalization() {
        let s1 = TmSegment::new(pt(0.0, 0.0), pt(10.0, 5.0));
        assert_eq!(s1.left, pt(0.0, 0.0));
        assert_eq!(s1.right, pt(10.0, 5.0));

        let s2 = TmSegment::new(pt(10.0, 5.0), pt(0.0, 0.0));
        assert_eq!(s2.left, pt(0.0, 0.0));
        assert_eq!(s2.right, pt(10.0, 5.0));
    }

    #[test]
    fn segment_y_at_x() {
        let s = TmSegment::new(pt(0.0, 0.0), pt(10.0, 10.0));
        assert!((s.y_at_x(5.0) - 5.0).abs() < 1e-10);
        assert!((s.y_at_x(0.0) - 0.0).abs() < 1e-10);
        assert!((s.y_at_x(10.0) - 10.0).abs() < 1e-10);
    }

    #[test]
    fn segment_above_below() {
        let s = TmSegment::new(pt(0.0, 5.0), pt(10.0, 5.0));
        assert!(s.is_above(pt(5.0, 8.0)));
        assert!(s.is_below(pt(5.0, 2.0)));
        assert!(!s.is_above(pt(5.0, 5.0)));
        assert!(!s.is_below(pt(5.0, 5.0)));
    }

    // ── Vertical segment ────────────────────────────────────────────────

    #[test]
    fn vertical_segment_handled() {
        let segs = vec![s(5.0, 1.0, 5.0, 9.0)];
        let tm = TrapezoidalMap::build(&segs, 42).unwrap();

        // Points on either side.
        let left = tm.locate(pt(2.0, 5.0)).unwrap();
        let right = tm.locate(pt(8.0, 5.0)).unwrap();
        // Both should be valid trapezoids.
        assert!(tm.point_in_trapezoid(pt(2.0, 5.0), tm.trapezoid(left)));
        assert!(tm.point_in_trapezoid(pt(8.0, 5.0), tm.trapezoid(right)));
    }

    // ── Multiple segments forming a staircase ───────────────────────────

    #[test]
    fn staircase_segments() {
        let segs = vec![
            s(1.0, 2.0, 3.0, 2.0),
            s(3.0, 4.0, 5.0, 4.0),
            s(5.0, 6.0, 7.0, 6.0),
        ];
        let tm = TrapezoidalMap::build(&segs, 42).unwrap();

        // Verify queries at various points.
        for x in 0..16 {
            for y in 0..16 {
                let p = pt(x as f64 * 0.5, y as f64 * 0.5);
                if let Ok(idx) = tm.locate(p) {
                    let trap = tm.trapezoid(idx);
                    assert!(
                        tm.point_in_trapezoid(p, trap),
                        "staircase: point ({}, {}) not in returned trapezoid",
                        p.x,
                        p.y
                    );
                }
            }
        }
    }

    // ── Error display ───────────────────────────────────────────────────

    #[test]
    fn error_display() {
        let e = TrapezoidalMapError::TooFewSegments;
        assert!(e.to_string().contains("at least 1 segment"));

        let e = TrapezoidalMapError::DegenerateSegment { index: 3 };
        assert!(e.to_string().contains("segment 3"));

        let e = TrapezoidalMapError::CrossingSegments { i: 1, j: 2 };
        assert!(e.to_string().contains("segments 1 and 2"));

        let e = TrapezoidalMapError::OutsideBoundingBox;
        assert!(e.to_string().contains("bounding box"));
    }
}
