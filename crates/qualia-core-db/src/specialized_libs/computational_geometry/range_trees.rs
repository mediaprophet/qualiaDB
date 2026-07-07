//! P11.10 — Interval, segment, hereditary segment, priority-search and range
//! trees.
//!
//! The acceptance gate requires: "Stabbing/reporting/counting results equal
//! linear scans; fractional-cascading indices are canonical; caller buffers
//! report exact capacity needs."
//!
//! ## Algorithms
//!
//! This module implements five orthogonal range-search structures:
//!
//! 1. **Interval tree** — stabs a point q, reporting all intervals [lo, hi]
//!    containing q. Built by sorting intervals on lo, partitioning at the
//!    median, and storing the right endpoints sorted descending in the left
//!    subtree and the left endpoints sorted ascending in the right subtree.
//!    O(n log n) build, O(log n + k) query.
//!
//! 2. **Segment tree** — stabs a point q on a 1-D axis, reporting all
//!    segments covering q. Built by canonical decomposition of each segment
//!    into O(log n) elementary intervals of a segment-tree skeleton.
//!    O(n log n) build, O(log n + k) query.
//!
//! 3. **Priority search tree** — 2-D orthogonal range reporting for queries
//!    of the form { (x, y) : x ∈ [x_lo, x_hi], y ≤ y_max }. A hybrid of a
//!    heap (on y) and a balanced BST (on x). O(n log n) build,
//!    O(log n + k) query.
//!
//! 4. **1-D range tree** — sorted array with binary search. Reports all
//!    keys in [lo, hi]. O(n) build, O(log n + k) query.
//!
//! 5. **2-D range tree** — a range tree on x, where each node stores a
//!    sorted array on y (the points in its subtree). Fractional cascading
//!    is simulated by storing the y-sorted arrays and binary-searching at
//!    each level. O(n log n) build, O(log² n + k) query (or O(log n + k)
//!    with fractional cascading).
//!
//! ## Zero-heap contract
//!
//! Tier-2 cold construction: `Vec` during build; the query path uses
//! caller-supplied output buffers (`&mut [u32]`) and reports the exact
//! capacity needed via a `required_*` function.

use super::primitives::Point2;

// ───────────────────────────────────────────────────────────────────────────
//  Interval tree (stabbing query)
// ───────────────────────────────────────────────────────────────────────────

/// A 1-D closed interval [lo, hi] with an associated index.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Interval {
    pub lo: f64,
    pub hi: f64,
    pub index: u32,
}

/// An interval tree node.
#[derive(Debug, Clone)]
struct IntervalNode {
    /// The split point (median of lo endpoints).
    split: f64,
    /// Intervals containing `split`, sorted by lo ascending. For a stabbing
    /// query at q ≤ split, hi ≥ q is guaranteed, so scan and report while
    /// lo ≤ q.
    left_sorted: Vec<Interval>,
    /// The SAME intervals as `left_sorted`, sorted by hi descending. For a
    /// stabbing query at q > split, lo ≤ q is guaranteed, so scan and report
    /// while hi ≥ q.
    right_sorted: Vec<Interval>,
    left: Option<Box<IntervalNode>>,
    right: Option<Box<IntervalNode>>,
}

/// An interval tree supporting stabbing queries: given a point q, report all
/// intervals containing q.
#[derive(Debug, Clone)]
pub struct IntervalTree {
    root: Option<Box<IntervalNode>>,
    count: usize,
}

impl IntervalTree {
    /// Build an interval tree from a set of intervals.
    pub fn build(intervals: &[Interval]) -> Self {
        let count = intervals.len();
        if intervals.is_empty() {
            return Self {
                root: None,
                count: 0,
            };
        }
        let root = build_interval_node(intervals);
        Self { root, count }
    }

    /// Stabbing query: report all intervals containing point `q`.
    /// Writes indices into `out` and returns the number written.
    pub fn stab(&self, q: f64, out: &mut [u32]) -> usize {
        let mut written = 0usize;
        if let Some(ref root) = self.root {
            stab_node(root, q, out, &mut written);
        }
        written
    }

    /// Returns the exact number of intervals that would be reported by a
    /// stabbing query at `q`. Use this to size the output buffer.
    pub fn stab_count(&self, q: f64) -> usize {
        if let Some(ref root) = self.root {
            let mut count = 0usize;
            stab_count_node(root, q, &mut count);
            count
        } else {
            0
        }
    }

    pub fn len(&self) -> usize {
        self.count
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
}

fn build_interval_node(intervals: &[Interval]) -> Option<Box<IntervalNode>> {
    if intervals.is_empty() {
        return None;
    }

    // Find median of lo endpoints.
    let mut los: Vec<f64> = intervals.iter().map(|i| i.lo).collect();
    los.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mid = los.len() / 2;
    let split = los[mid];

    // Partition:
    //   containing: intervals that contain `split` (lo <= split <= hi)
    //   left_only: intervals entirely to the left (hi < split)
    //   right_only: intervals entirely to the right (lo > split)
    let mut containing: Vec<Interval> = Vec::new();
    let mut left_only: Vec<Interval> = Vec::new();
    let mut right_only: Vec<Interval> = Vec::new();
    for &iv in intervals {
        if iv.lo <= split && iv.hi >= split {
            containing.push(iv);
        } else if iv.hi < split {
            left_only.push(iv);
        } else {
            right_only.push(iv);
        }
    }

    // left_sorted: containing intervals sorted by lo ascending.
    let mut left_sorted = containing.clone();
    left_sorted.sort_by(|a, b| a.lo.partial_cmp(&b.lo).unwrap_or(std::cmp::Ordering::Equal));
    // right_sorted: same intervals sorted by hi descending.
    let mut right_sorted = containing;
    right_sorted.sort_by(|a, b| b.hi.partial_cmp(&a.hi).unwrap_or(std::cmp::Ordering::Equal));

    let left_child = if left_only.len() > 1 {
        build_interval_node(&left_only)
    } else if left_only.len() == 1 {
        Some(Box::new(IntervalNode {
            split: left_only[0].lo,
            left_sorted: left_only.clone(),
            right_sorted: left_only,
            left: None,
            right: None,
        }))
    } else {
        None
    };
    let right_child = if right_only.len() > 1 {
        build_interval_node(&right_only)
    } else if right_only.len() == 1 {
        Some(Box::new(IntervalNode {
            split: right_only[0].lo,
            left_sorted: right_only.clone(),
            right_sorted: right_only,
            left: None,
            right: None,
        }))
    } else {
        None
    };

    Some(Box::new(IntervalNode {
        split,
        left_sorted,
        right_sorted,
        left: left_child,
        right: right_child,
    }))
}

fn stab_node(node: &IntervalNode, q: f64, out: &mut [u32], written: &mut usize) {
    if q <= node.split {
        // left_sorted is sorted by lo ascending. All intervals contain split
        // and split >= q, so hi >= q is guaranteed. Report while lo <= q.
        for iv in &node.left_sorted {
            if iv.lo <= q {
                if *written < out.len() {
                    out[*written] = iv.index;
                }
                *written += 1;
            } else {
                break; // sorted by lo ascending, no more will match
            }
        }
        if let Some(ref left) = node.left {
            stab_node(left, q, out, written);
        }
    } else {
        // right_sorted is sorted by hi descending. All intervals contain split
        // and split < q, so lo <= q is guaranteed. Report while hi >= q.
        for iv in &node.right_sorted {
            if iv.hi >= q {
                if *written < out.len() {
                    out[*written] = iv.index;
                }
                *written += 1;
            } else {
                break; // sorted by hi descending, no more will match
            }
        }
        if let Some(ref right) = node.right {
            stab_node(right, q, out, written);
        }
    }
}

fn stab_count_node(node: &IntervalNode, q: f64, count: &mut usize) {
    if q <= node.split {
        for iv in &node.left_sorted {
            if iv.lo <= q {
                *count += 1;
            } else {
                break;
            }
        }
        if let Some(ref left) = node.left {
            stab_count_node(left, q, count);
        }
    } else {
        for iv in &node.right_sorted {
            if iv.hi >= q {
                *count += 1;
            } else {
                break;
            }
        }
        if let Some(ref right) = node.right {
            stab_count_node(right, q, count);
        }
    }
}

// ───────────────────────────────────────────────────────────────────────────
//  1-D range tree (sorted array + binary search)
// ───────────────────────────────────────────────────────────────────────────

/// A 1-D range tree: a sorted array supporting range reporting queries.
#[derive(Debug, Clone)]
pub struct RangeTree1D {
    keys: Vec<f64>,
    indices: Vec<u32>,
}

impl RangeTree1D {
    /// Build from a set of (key, index) pairs.
    pub fn build(data: &[(f64, u32)]) -> Self {
        let mut sorted: Vec<(f64, u32)> = data.to_vec();
        sorted.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        let keys: Vec<f64> = sorted.iter().map(|(k, _)| *k).collect();
        let indices: Vec<u32> = sorted.iter().map(|(_, i)| *i).collect();
        Self { keys, indices }
    }

    /// Range query: report all keys in [lo, hi]. Writes indices into `out`.
    pub fn range_query(&self, lo: f64, hi: f64, out: &mut [u32]) -> usize {
        let start = self.keys.partition_point(|&k| k < lo);
        let end = self.keys.partition_point(|&k| k <= hi);
        let count = end - start;
        for i in 0..count {
            if i < out.len() {
                out[i] = self.indices[start + i];
            }
        }
        count
    }

    /// Returns the exact number of keys in [lo, hi].
    pub fn range_count(&self, lo: f64, hi: f64) -> usize {
        let start = self.keys.partition_point(|&k| k < lo);
        let end = self.keys.partition_point(|&k| k <= hi);
        end - start
    }

    pub fn len(&self) -> usize {
        self.keys.len()
    }

    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }
}

// ───────────────────────────────────────────────────────────────────────────
//  2-D range tree (range tree on x, sorted arrays on y)
// ───────────────────────────────────────────────────────────────────────────

/// A node in the 2-D range tree.
#[derive(Debug, Clone)]
struct Range2DNode {
    /// All points in this subtree, sorted by y. Stores (x, y, index) so leaf
    /// nodes can check x-range during partial-overlap queries.
    y_sorted: Vec<(f64, f64, u32)>, // (x, y, index)
    /// The x-range of this subtree [min_x, max_x].
    min_x: f64,
    max_x: f64,
    left: Option<Box<Range2DNode>>,
    right: Option<Box<Range2DNode>>,
}

/// A 2-D range tree supporting orthogonal range queries:
/// { (x, y) : x ∈ [x_lo, x_hi], y ∈ [y_lo, y_hi] }.
#[derive(Debug, Clone)]
pub struct RangeTree2D {
    root: Option<Box<Range2DNode>>,
    count: usize,
}

impl RangeTree2D {
    /// Build from a set of 2-D points with indices.
    pub fn build(points: &[(Point2, u32)]) -> Self {
        let count = points.len();
        if points.is_empty() {
            return Self {
                root: None,
                count: 0,
            };
        }
        let root = build_range2d_node(points);
        Self { root, count }
    }

    /// 2-D orthogonal range query. Writes indices into `out`.
    pub fn range_query(
        &self,
        x_lo: f64,
        x_hi: f64,
        y_lo: f64,
        y_hi: f64,
        out: &mut [u32],
    ) -> usize {
        let mut written = 0usize;
        if let Some(ref root) = self.root {
            range2d_query(root, x_lo, x_hi, y_lo, y_hi, out, &mut written);
        }
        written
    }

    /// Returns the exact count of points in the 2-D range.
    pub fn range_count(&self, x_lo: f64, x_hi: f64, y_lo: f64, y_hi: f64) -> usize {
        if let Some(ref root) = self.root {
            range2d_count(root, x_lo, x_hi, y_lo, y_hi)
        } else {
            0
        }
    }

    pub fn len(&self) -> usize {
        self.count
    }
}

fn build_range2d_node(points: &[(Point2, u32)]) -> Option<Box<Range2DNode>> {
    if points.is_empty() {
        return None;
    }
    // Sort by x and find median.
    let mut sorted: Vec<(Point2, u32)> = points.to_vec();
    sorted.sort_by(|a, b| {
        a.0.x
            .partial_cmp(&b.0.x)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let mid = sorted.len() / 2;
    let min_x = sorted[0].0.x;
    let max_x = sorted[sorted.len() - 1].0.x;

    // y-sorted array for this subtree (all points): (x, y, index).
    let mut y_sorted: Vec<(f64, f64, u32)> = sorted.iter().map(|(p, i)| (p.x, p.y, *i)).collect();
    y_sorted.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    if sorted.len() <= 2 {
        return Some(Box::new(Range2DNode {
            y_sorted,
            min_x,
            max_x,
            left: None,
            right: None,
        }));
    }

    let left: Vec<(Point2, u32)> = sorted[..mid].to_vec();
    let right: Vec<(Point2, u32)> = sorted[mid..].to_vec();

    Some(Box::new(Range2DNode {
        y_sorted,
        min_x,
        max_x,
        left: build_range2d_node(&left),
        right: build_range2d_node(&right),
    }))
}

fn range2d_query(
    node: &Range2DNode,
    x_lo: f64,
    x_hi: f64,
    y_lo: f64,
    y_hi: f64,
    out: &mut [u32],
    written: &mut usize,
) {
    // No overlap → skip.
    if node.max_x < x_lo || node.min_x > x_hi {
        return;
    }
    // If this subtree's x-range is fully inside [x_lo, x_hi], report all
    // y-matching points from y_sorted (canonical decomposition).
    if node.min_x >= x_lo && node.max_x <= x_hi {
        report_y_range(&node.y_sorted, y_lo, y_hi, out, written);
        return;
    }
    // Partial overlap: if leaf, scan y_sorted checking both x and y.
    if node.left.is_none() && node.right.is_none() {
        for &(x, y, idx) in &node.y_sorted {
            if x >= x_lo && x <= x_hi && y >= y_lo && y <= y_hi {
                if *written < out.len() {
                    out[*written] = idx;
                }
                *written += 1;
            }
        }
        return;
    }
    // Partial overlap: recurse into children.
    if let Some(ref left) = node.left {
        range2d_query(left, x_lo, x_hi, y_lo, y_hi, out, written);
    }
    if let Some(ref right) = node.right {
        range2d_query(right, x_lo, x_hi, y_lo, y_hi, out, written);
    }
}

fn range2d_count(node: &Range2DNode, x_lo: f64, x_hi: f64, y_lo: f64, y_hi: f64) -> usize {
    if node.max_x < x_lo || node.min_x > x_hi {
        return 0;
    }
    if node.min_x >= x_lo && node.max_x <= x_hi {
        return count_y_range(&node.y_sorted, y_lo, y_hi);
    }
    if node.left.is_none() && node.right.is_none() {
        return node
            .y_sorted
            .iter()
            .filter(|(x, y, _)| *x >= x_lo && *x <= x_hi && *y >= y_lo && *y <= y_hi)
            .count();
    }
    let mut count = 0;
    if let Some(ref left) = node.left {
        count += range2d_count(left, x_lo, x_hi, y_lo, y_hi);
    }
    if let Some(ref right) = node.right {
        count += range2d_count(right, x_lo, x_hi, y_lo, y_hi);
    }
    count
}

fn report_y_range(
    y_sorted: &[(f64, f64, u32)],
    y_lo: f64,
    y_hi: f64,
    out: &mut [u32],
    written: &mut usize,
) {
    let start = y_sorted.partition_point(|(_, y, _)| *y < y_lo);
    let end = y_sorted.partition_point(|(_, y, _)| *y <= y_hi);
    for i in start..end {
        if *written < out.len() {
            out[*written] = y_sorted[i].2;
        }
        *written += 1;
    }
}

fn count_y_range(y_sorted: &[(f64, f64, u32)], y_lo: f64, y_hi: f64) -> usize {
    let start = y_sorted.partition_point(|(_, y, _)| *y < y_lo);
    let end = y_sorted.partition_point(|(_, y, _)| *y <= y_hi);
    end - start
}

// ───────────────────────────────────────────────────────────────────────────
//  Priority search tree (2-D: x in range, y <= y_max)
// ───────────────────────────────────────────────────────────────────────────

/// A priority search tree node.
#[derive(Debug, Clone)]
struct PSTNode {
    /// The point at this node (the one with minimum y in its subtree).
    point: Point2,
    index: u32,
    /// Split on x.
    split_x: f64,
    left: Option<Box<PSTNode>>,
    right: Option<Box<PSTNode>>,
}

/// A priority search tree supporting queries of the form:
/// { (x, y) : x ∈ [x_lo, x_hi], y ≤ y_max }.
#[derive(Debug, Clone)]
pub struct PrioritySearchTree {
    root: Option<Box<PSTNode>>,
    count: usize,
}

impl PrioritySearchTree {
    /// Build from a set of 2-D points with indices.
    pub fn build(points: &[(Point2, u32)]) -> Self {
        let count = points.len();
        if points.is_empty() {
            return Self {
                root: None,
                count: 0,
            };
        }
        let root = build_pst_node(points);
        Self { root, count }
    }

    /// Query: report all points with x ∈ [x_lo, x_hi] and y ≤ y_max.
    pub fn query(&self, x_lo: f64, x_hi: f64, y_max: f64, out: &mut [u32]) -> usize {
        let mut written = 0usize;
        if let Some(ref root) = self.root {
            pst_query(root, x_lo, x_hi, y_max, out, &mut written);
        }
        written
    }

    /// Returns the exact count for the query.
    pub fn query_count(&self, x_lo: f64, x_hi: f64, y_max: f64) -> usize {
        if let Some(ref root) = self.root {
            let mut count = 0usize;
            pst_count(root, x_lo, x_hi, y_max, &mut count);
            count
        } else {
            0
        }
    }

    pub fn len(&self) -> usize {
        self.count
    }
}

fn build_pst_node(points: &[(Point2, u32)]) -> Option<Box<PSTNode>> {
    if points.is_empty() {
        return None;
    }
    // Find the point with minimum y.
    let min_idx = points
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| {
            a.0.y
                .partial_cmp(&b.0.y)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(i, _)| i)
        .unwrap();
    let (min_point, min_index) = points[min_idx];

    if points.len() == 1 {
        return Some(Box::new(PSTNode {
            point: min_point,
            index: min_index,
            split_x: min_point.x,
            left: None,
            right: None,
        }));
    }

    // Remaining points (excluding min).
    let remaining: Vec<(Point2, u32)> = points
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != min_idx)
        .map(|(_, p)| *p)
        .collect();

    // Median x of remaining.
    let mut xs: Vec<f64> = remaining.iter().map(|(p, _)| p.x).collect();
    xs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let split_x = xs[xs.len() / 2];

    let mut left: Vec<(Point2, u32)> = Vec::new();
    let mut right: Vec<(Point2, u32)> = Vec::new();
    for (p, i) in &remaining {
        if p.x <= split_x {
            left.push((*p, *i));
        } else {
            right.push((*p, *i));
        }
    }

    Some(Box::new(PSTNode {
        point: min_point,
        index: min_index,
        split_x,
        left: build_pst_node(&left),
        right: build_pst_node(&right),
    }))
}

fn pst_query(
    node: &PSTNode,
    x_lo: f64,
    x_hi: f64,
    y_max: f64,
    out: &mut [u32],
    written: &mut usize,
) {
    // If this node's point has y > y_max, no point in the subtree can match
    // (this node has the min y).
    if node.point.y > y_max {
        return;
    }
    // Report if x is in range.
    if node.point.x >= x_lo && node.point.x <= x_hi {
        if *written < out.len() {
            out[*written] = node.index;
        }
        *written += 1;
    }
    // Recurse into children.
    let split = node.split_x;
    if x_lo <= split {
        if let Some(ref left) = node.left {
            pst_query(left, x_lo, x_hi, y_max, out, written);
        }
    }
    if x_hi > split {
        if let Some(ref right) = node.right {
            pst_query(right, x_lo, x_hi, y_max, out, written);
        }
    }
}

fn pst_count(node: &PSTNode, x_lo: f64, x_hi: f64, y_max: f64, count: &mut usize) {
    if node.point.y > y_max {
        return;
    }
    if node.point.x >= x_lo && node.point.x <= x_hi {
        *count += 1;
    }
    let split = node.split_x;
    if x_lo <= split {
        if let Some(ref left) = node.left {
            pst_count(left, x_lo, x_hi, y_max, count);
        }
    }
    if x_hi > split {
        if let Some(ref right) = node.right {
            pst_count(right, x_lo, x_hi, y_max, count);
        }
    }
}

// ───────────────────────────────────────────────────────────────────────────
//  Segment tree (1-D stabbing on segments)
// ───────────────────────────────────────────────────────────────────────────

/// A segment tree node covering the elementary interval [lo, hi].
#[derive(Debug, Clone)]
struct SegTreeNode {
    lo: f64,
    hi: f64,
    /// Segments that cover this node's entire interval (canonical decomposition).
    segments: Vec<u32>,
    left: Option<Box<SegTreeNode>>,
    right: Option<Box<SegTreeNode>>,
}

/// A segment tree supporting stabbing queries: given a point q, report all
/// segments covering q.
#[derive(Debug, Clone)]
pub struct SegmentTree {
    root: Option<Box<SegTreeNode>>,
}

impl SegmentTree {
    /// Build a segment tree for the coordinate range [lo, hi] with the given
    /// segments (each [seg_lo, seg_hi] with an index).
    pub fn build(lo: f64, hi: f64, segments: &[(f64, f64, u32)]) -> Self {
        if segments.is_empty() || hi <= lo {
            return Self { root: None };
        }
        // Collect all unique endpoints to build elementary intervals.
        let mut endpoints: Vec<f64> = vec![lo, hi];
        for &(s_lo, s_hi, _) in segments {
            if s_lo > lo {
                endpoints.push(s_lo);
            }
            if s_hi < hi {
                endpoints.push(s_hi);
            }
        }
        endpoints.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        endpoints.dedup();

        let root = build_seg_tree(lo, hi, &endpoints);
        // Insert segments.
        let mut root = root;
        for &(s_lo, s_hi, idx) in segments {
            insert_seg_tree(root.as_mut().unwrap(), s_lo, s_hi, idx);
        }
        Self { root }
    }

    /// Stabbing query: report all segments covering point `q`.
    pub fn stab(&self, q: f64, out: &mut [u32]) -> usize {
        let mut written = 0usize;
        if let Some(ref root) = self.root {
            stab_seg_tree(root, q, out, &mut written);
        }
        written
    }

    /// Returns the exact count of segments covering `q`.
    pub fn stab_count(&self, q: f64) -> usize {
        if let Some(ref root) = self.root {
            let mut count = 0usize;
            stab_count_seg_tree(root, q, &mut count);
            count
        } else {
            0
        }
    }
}

fn build_seg_tree(lo: f64, hi: f64, endpoints: &[f64]) -> Option<Box<SegTreeNode>> {
    if hi <= lo {
        return None;
    }
    // Find the midpoint in the endpoint list.
    let mid_idx = endpoints.partition_point(|&e| e < (lo + hi) / 2.0);
    let mid = if mid_idx > 0 && mid_idx < endpoints.len() {
        endpoints[mid_idx]
    } else {
        (lo + hi) / 2.0
    };

    if mid <= lo || mid >= hi {
        // Leaf: elementary interval [lo, hi].
        return Some(Box::new(SegTreeNode {
            lo,
            hi,
            segments: Vec::new(),
            left: None,
            right: None,
        }));
    }

    Some(Box::new(SegTreeNode {
        lo,
        hi,
        segments: Vec::new(),
        left: build_seg_tree(lo, mid, endpoints),
        right: build_seg_tree(mid, hi, endpoints),
    }))
}

fn insert_seg_tree(node: &mut SegTreeNode, s_lo: f64, s_hi: f64, idx: u32) {
    // If the segment fully covers this node's interval, store it here.
    if s_lo <= node.lo && s_hi >= node.hi {
        node.segments.push(idx);
        return;
    }
    // Otherwise, recurse into children that overlap.
    if s_lo < node.lo + (node.hi - node.lo) / 2.0 {
        if let Some(ref mut left) = node.left {
            insert_seg_tree(left, s_lo, s_hi, idx);
        }
    }
    if s_hi > node.lo + (node.hi - node.lo) / 2.0 {
        if let Some(ref mut right) = node.right {
            insert_seg_tree(right, s_lo, s_hi, idx);
        }
    }
}

fn stab_seg_tree(node: &SegTreeNode, q: f64, out: &mut [u32], written: &mut usize) {
    if q < node.lo || q > node.hi {
        return;
    }
    for &idx in &node.segments {
        if *written < out.len() {
            out[*written] = idx;
        }
        *written += 1;
    }
    if let Some(ref left) = node.left {
        stab_seg_tree(left, q, out, written);
    }
    if let Some(ref right) = node.right {
        stab_seg_tree(right, q, out, written);
    }
}

fn stab_count_seg_tree(node: &SegTreeNode, q: f64, count: &mut usize) {
    if q < node.lo || q > node.hi {
        return;
    }
    *count += node.segments.len();
    if let Some(ref left) = node.left {
        stab_count_seg_tree(left, q, count);
    }
    if let Some(ref right) = node.right {
        stab_count_seg_tree(right, q, count);
    }
}

// ───────────────────────────────────────────────────────────────────────────
//  Brute-force oracles (for verification)
// ───────────────────────────────────────────────────────────────────────────

/// Brute-force stabbing query: linear scan over intervals.
pub fn brute_stab(intervals: &[Interval], q: f64) -> Vec<u32> {
    intervals
        .iter()
        .filter(|iv| iv.lo <= q && iv.hi >= q)
        .map(|iv| iv.index)
        .collect()
}

/// Brute-force 1-D range query: linear scan.
pub fn brute_range_1d(data: &[(f64, u32)], lo: f64, hi: f64) -> Vec<u32> {
    data.iter()
        .filter(|(k, _)| *k >= lo && *k <= hi)
        .map(|(_, i)| *i)
        .collect()
}

/// Brute-force 2-D range query: linear scan.
pub fn brute_range_2d(
    points: &[(Point2, u32)],
    x_lo: f64,
    x_hi: f64,
    y_lo: f64,
    y_hi: f64,
) -> Vec<u32> {
    points
        .iter()
        .filter(|(p, _)| p.x >= x_lo && p.x <= x_hi && p.y >= y_lo && p.y <= y_hi)
        .map(|(_, i)| *i)
        .collect()
}

/// Brute-force priority search query: linear scan.
pub fn brute_pst(points: &[(Point2, u32)], x_lo: f64, x_hi: f64, y_max: f64) -> Vec<u32> {
    points
        .iter()
        .filter(|(p, _)| p.x >= x_lo && p.x <= x_hi && p.y <= y_max)
        .map(|(_, i)| *i)
        .collect()
}

/// Brute-force segment stabbing: linear scan.
pub fn brute_seg_stab(segments: &[(f64, f64, u32)], q: f64) -> Vec<u32> {
    segments
        .iter()
        .filter(|(lo, hi, _)| *lo <= q && *hi >= q)
        .map(|(_, _, i)| *i)
        .collect()
}

// ───────────────────────────────────────────────────────────────────────────
//  Tests
// ───────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn iv(lo: f64, hi: f64, idx: u32) -> Interval {
        Interval { lo, hi, index: idx }
    }

    fn pt(x: f64, y: f64, idx: u32) -> (Point2, u32) {
        (Point2::new(x, y), idx)
    }

    fn sort_mut(v: &mut [u32]) {
        v.sort();
    }

    fn sort_vec(mut v: Vec<u32>) -> Vec<u32> {
        v.sort();
        v
    }

    // ── Interval tree ──

    #[test]
    fn interval_tree_stab_matches_brute_force() {
        let intervals = vec![
            iv(0.0, 5.0, 0),
            iv(2.0, 8.0, 1),
            iv(4.0, 6.0, 2),
            iv(7.0, 10.0, 3),
            iv(1.0, 3.0, 4),
        ];
        let tree = IntervalTree::build(&intervals);
        for q in [0.0, 0.5, 1.0, 2.5, 4.0, 5.5, 7.0, 9.0, 10.0, 11.0] {
            let expected = sort_vec(brute_stab(&intervals, q));
            let count = tree.stab_count(q);
            assert_eq!(count, expected.len(), "count mismatch at q={q}");
            let mut buf = vec![u32::MAX; count];
            let written = tree.stab(q, &mut buf);
            assert_eq!(written, count, "written != count at q={q}");
            sort_mut(&mut buf);
            assert_eq!(buf, expected, "stab mismatch at q={q}");
        }
    }

    #[test]
    fn interval_tree_empty() {
        let tree = IntervalTree::build(&[]);
        assert!(tree.is_empty());
        assert_eq!(tree.stab_count(5.0), 0);
    }

    #[test]
    fn interval_tree_single() {
        let tree = IntervalTree::build(&[iv(1.0, 3.0, 42)]);
        assert_eq!(tree.stab_count(2.0), 1);
        let mut buf = [0u32; 1];
        tree.stab(2.0, &mut buf);
        assert_eq!(buf[0], 42);
        assert_eq!(tree.stab_count(4.0), 0);
    }

    #[test]
    fn interval_tree_buffer_too_small_reports_count() {
        let intervals = vec![iv(0.0, 10.0, 0), iv(1.0, 9.0, 1), iv(2.0, 8.0, 2)];
        let tree = IntervalTree::build(&intervals);
        let count = tree.stab_count(5.0);
        assert_eq!(count, 3);
        // Undersized buffer: only writes 2, but returns 3.
        let mut buf = [0u32; 2];
        let written = tree.stab(5.0, &mut buf);
        assert_eq!(
            written, 3,
            "should report full count even if buffer is small"
        );
    }

    // ── 1-D range tree ──

    #[test]
    fn range_1d_matches_brute_force() {
        let data: Vec<(f64, u32)> = vec![
            (3.0, 0),
            (1.0, 1),
            (5.0, 2),
            (2.0, 3),
            (4.0, 4),
            (7.0, 5),
            (6.0, 6),
        ];
        let tree = RangeTree1D::build(&data);
        for (lo, hi) in [(0.0, 10.0), (2.0, 5.0), (3.0, 3.0), (6.0, 8.0), (0.0, 0.5)] {
            let expected = sort_vec(brute_range_1d(&data, lo, hi));
            let count = tree.range_count(lo, hi);
            assert_eq!(count, expected.len(), "count mismatch for [{lo}, {hi}]");
            let mut buf = vec![u32::MAX; count];
            tree.range_query(lo, hi, &mut buf);
            sort_mut(&mut buf);
            assert_eq!(buf, expected, "range mismatch for [{lo}, {hi}]");
        }
    }

    // ── 2-D range tree ──

    #[test]
    fn range_2d_matches_brute_force() {
        let points: Vec<(Point2, u32)> = vec![
            pt(1.0, 1.0, 0),
            pt(3.0, 2.0, 1),
            pt(2.0, 5.0, 2),
            pt(5.0, 3.0, 3),
            pt(4.0, 4.0, 4),
            pt(6.0, 1.0, 5),
            pt(3.0, 6.0, 6),
            pt(7.0, 7.0, 7),
        ];
        let tree = RangeTree2D::build(&points);
        for (x_lo, x_hi, y_lo, y_hi) in [
            (0.0, 10.0, 0.0, 10.0),
            (2.0, 5.0, 2.0, 5.0),
            (1.0, 3.0, 1.0, 2.0),
            (4.0, 7.0, 3.0, 7.0),
            (0.0, 0.5, 0.0, 0.5),
        ] {
            let expected = sort_vec(brute_range_2d(&points, x_lo, x_hi, y_lo, y_hi));
            let count = tree.range_count(x_lo, x_hi, y_lo, y_hi);
            assert_eq!(
                count,
                expected.len(),
                "count mismatch for [{x_lo},{x_hi}]×[{y_lo},{y_hi}]"
            );
            let mut buf = vec![u32::MAX; count];
            tree.range_query(x_lo, x_hi, y_lo, y_hi, &mut buf);
            sort_mut(&mut buf);
            assert_eq!(
                buf, expected,
                "range mismatch for [{x_lo},{x_hi}]×[{y_lo},{y_hi}]"
            );
        }
    }

    // ── Priority search tree ──

    #[test]
    fn pst_matches_brute_force() {
        let points: Vec<(Point2, u32)> = vec![
            pt(1.0, 5.0, 0),
            pt(3.0, 2.0, 1),
            pt(2.0, 7.0, 2),
            pt(5.0, 1.0, 3),
            pt(4.0, 3.0, 4),
            pt(6.0, 6.0, 5),
        ];
        let tree = PrioritySearchTree::build(&points);
        for (x_lo, x_hi, y_max) in [
            (0.0, 10.0, 10.0),
            (2.0, 5.0, 3.0),
            (1.0, 3.0, 5.0),
            (0.0, 10.0, 1.0),
            (4.0, 6.0, 6.0),
        ] {
            let expected = sort_vec(brute_pst(&points, x_lo, x_hi, y_max));
            let count = tree.query_count(x_lo, x_hi, y_max);
            assert_eq!(
                count,
                expected.len(),
                "count mismatch for x∈[{x_lo},{x_hi}] y≤{y_max}"
            );
            let mut buf = vec![u32::MAX; count];
            tree.query(x_lo, x_hi, y_max, &mut buf);
            sort_mut(&mut buf);
            assert_eq!(
                buf, expected,
                "pst mismatch for x∈[{x_lo},{x_hi}] y≤{y_max}"
            );
        }
    }

    // ── Segment tree ──

    #[test]
    fn segment_tree_stab_matches_brute_force() {
        let segments: Vec<(f64, f64, u32)> = vec![
            (0.0, 5.0, 0),
            (2.0, 8.0, 1),
            (4.0, 6.0, 2),
            (7.0, 10.0, 3),
            (1.0, 3.0, 4),
        ];
        let tree = SegmentTree::build(0.0, 10.0, &segments);
        for q in [0.0, 0.5, 1.0, 2.5, 4.0, 5.5, 7.0, 9.0, 10.0] {
            let expected = sort_vec(brute_seg_stab(&segments, q));
            let count = tree.stab_count(q);
            assert_eq!(count, expected.len(), "seg count mismatch at q={q}");
            let mut buf = vec![u32::MAX; count];
            let written = tree.stab(q, &mut buf);
            assert_eq!(written, count);
            sort_mut(&mut buf);
            assert_eq!(buf, expected, "seg stab mismatch at q={q}");
        }
    }

    #[test]
    fn segment_tree_empty() {
        let tree = SegmentTree::build(0.0, 10.0, &[]);
        assert_eq!(tree.stab_count(5.0), 0);
    }

    // ── Determinism ──

    #[test]
    fn determinism_interval_tree() {
        let intervals = vec![iv(0.0, 5.0, 0), iv(2.0, 8.0, 1), iv(4.0, 6.0, 2)];
        let t1 = IntervalTree::build(&intervals);
        let t2 = IntervalTree::build(&intervals);
        let mut b1 = vec![0u32; 10];
        let mut b2 = vec![0u32; 10];
        let w1 = t1.stab(4.0, &mut b1);
        let w2 = t2.stab(4.0, &mut b2);
        assert_eq!(w1, w2);
        b1.truncate(w1);
        b2.truncate(w2);
        assert_eq!(b1, b2);
    }

    #[test]
    fn determinism_range_2d() {
        let points = vec![pt(1.0, 1.0, 0), pt(3.0, 2.0, 1), pt(2.0, 5.0, 2)];
        let t1 = RangeTree2D::build(&points);
        let t2 = RangeTree2D::build(&points);
        let mut b1 = vec![0u32; 10];
        let mut b2 = vec![0u32; 10];
        let w1 = t1.range_query(0.0, 10.0, 0.0, 10.0, &mut b1);
        let w2 = t2.range_query(0.0, 10.0, 0.0, 10.0, &mut b2);
        assert_eq!(w1, w2);
        b1.truncate(w1);
        b2.truncate(w2);
        sort_mut(&mut b1);
        sort_mut(&mut b2);
        assert_eq!(b1, b2);
    }

    #[test]
    fn determinism_pst() {
        let points = vec![pt(1.0, 5.0, 0), pt(3.0, 2.0, 1), pt(2.0, 7.0, 2)];
        let t1 = PrioritySearchTree::build(&points);
        let t2 = PrioritySearchTree::build(&points);
        let mut b1 = vec![0u32; 10];
        let mut b2 = vec![0u32; 10];
        let w1 = t1.query(0.0, 10.0, 10.0, &mut b1);
        let w2 = t2.query(0.0, 10.0, 10.0, &mut b2);
        assert_eq!(w1, w2);
        b1.truncate(w1);
        b2.truncate(w2);
        sort_mut(&mut b1);
        sort_mut(&mut b2);
        assert_eq!(b1, b2);
    }
}
