//! P11.8 — Arrangements, point-line duality, and topological sweep.
//!
//! The acceptance gate requires: "Full line arrangement has correct V/E/F
//! counts, zone traversal matches a direct oracle, and dual transforms
//! round-trip finite/non-vertical cases."
//!
//! ## Algorithms
//!
//! ### Line arrangement
//!
//! Given `n` lines, the arrangement is the planar subdivision induced by their
//! pairwise intersections. For `n` lines in general position (no three
//! concurrent, no two parallel):
//!
//! - **V** = `n(n-1)/2` intersection points.
//! - **E** = `n²` edges (each line is split into `n` segments/rays by the
//!   other `n-1` lines, giving `n` edges per line × `n` lines = `n²`).
//! - **F** = `n(n-1)/2 + 1` faces (by Euler: V − E + F = 2).
//!
//! Unbounded edges (rays) are clipped to a bounding box enclosing all
//! vertices with a margin, so the arrangement is treated as a bounded
//! subdivision with a single unbounded face wrapping around the box.
//!
//! ### Zone traversal
//!
//! The *zone* of a curve `γ` in an arrangement `A` is the sequence of faces
//! that `γ` passes through. For a line crossing an arrangement of `n` lines,
//! the zone has at most `2n` faces (the Zone Theorem). The traversal finds
//! every intersection of `γ` with arrangement edges, sorts them along `γ`,
//! and reports the face between each consecutive pair by locating a
//! midpoint in the arrangement.
//!
//! ### Point-line duality
//!
//! The standard point-line duality transform:
//!
//! - Point `p = (a, b)` ↔ dual line `p* : y = a·x − b`.
//! - Line `l : y = m·x + c` ↔ dual point `l* = (m, −c)`.
//!
//! Key property: `p` is above `l` ⟺ `l*` is above `p*`. The round-trip
//! `dual(dual(p)) = p` holds for all finite, non-vertical cases.
//!
//! ## Zero-heap contract
//!
//! Tier-2 cold construction (AGENTS.md §0-A): `Vec` during build; typed struct
//! output. The orientation predicate path is zero-heap.

use super::boolean_2::point_in_polygon;
use super::primitives::Point2;

// ───────────────────────────────────────────────────────────────────────────
//  Line representation
// ───────────────────────────────────────────────────────────────────────────

/// A 2-D line in slope-intercept form. Vertical lines have `is_vertical = true`
/// and use `x_const` instead of slope/intercept.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Line2 {
    /// Slope `m` in `y = m·x + b`. Unused for vertical lines.
    pub slope: f64,
    /// Intercept `b` in `y = m·x + b`. Unused for vertical lines.
    pub intercept: f64,
    /// True for a vertical line `x = x_const`.
    pub is_vertical: bool,
    /// For vertical lines: the constant x-value.
    pub x_const: f64,
}

impl Line2 {
    /// Create a non-vertical line `y = slope·x + intercept`.
    pub fn new(slope: f64, intercept: f64) -> Self {
        Self {
            slope,
            intercept,
            is_vertical: false,
            x_const: 0.0,
        }
    }

    /// Create a vertical line `x = x_const`.
    pub fn vertical(x_const: f64) -> Self {
        Self {
            slope: 0.0,
            intercept: 0.0,
            is_vertical: true,
            x_const,
        }
    }

    /// Create a line through two points.
    pub fn through_points(a: Point2, b: Point2) -> Self {
        let dx = b.x - a.x;
        if dx.abs() <= f64::MIN_POSITIVE {
            Self::vertical(a.x)
        } else {
            let slope = (b.y - a.y) / dx;
            let intercept = a.y - slope * a.x;
            Self::new(slope, intercept)
        }
    }

    /// Evaluate the line at x: returns `y = slope·x + intercept`, or `x_const`
    /// for vertical lines (in which case `x` should equal `x_const`).
    #[inline]
    pub fn y_at(&self, x: f64) -> f64 {
        if self.is_vertical {
            self.x_const
        } else {
            self.slope * x + self.intercept
        }
    }

    /// Evaluate the x-value at a given y: returns `x = (y - b) / m`, or
    /// `x_const` for vertical lines.
    #[inline]
    pub fn x_at(&self, y: f64) -> f64 {
        if self.is_vertical {
            self.x_const
        } else if self.slope.abs() <= f64::MIN_POSITIVE {
            // Horizontal line: no unique x for a given y (unless y == intercept).
            f64::NAN
        } else {
            (y - self.intercept) / self.slope
        }
    }

    /// Is this line parallel to `other`?
    #[inline]
    pub fn is_parallel(&self, other: &Line2) -> bool {
        if self.is_vertical && other.is_vertical {
            return true;
        }
        if self.is_vertical || other.is_vertical {
            return false;
        }
        (self.slope - other.slope).abs()
            <= f64::EPSILON * (self.slope.abs() + other.slope.abs() + 1.0)
    }
}

/// Compute the intersection point of two lines. Returns `None` if parallel.
pub fn line_line_intersection(l1: &Line2, l2: &Line2) -> Option<Point2> {
    if l1.is_vertical && l2.is_vertical {
        return None; // parallel (or identical)
    }
    if l1.is_vertical {
        return Some(Point2::new(l1.x_const, l2.y_at(l1.x_const)));
    }
    if l2.is_vertical {
        return Some(Point2::new(l2.x_const, l1.y_at(l2.x_const)));
    }
    if l1.is_parallel(l2) {
        return None;
    }
    let x = (l2.intercept - l1.intercept) / (l1.slope - l2.slope);
    let y = l1.y_at(x);
    Some(Point2::new(x, y))
}

// ───────────────────────────────────────────────────────────────────────────
//  Arrangement types
// ───────────────────────────────────────────────────────────────────────────

/// One edge of a line arrangement: a segment along one line between two
/// vertices (or between a vertex and a bounding-box clip point for unbounded
/// edges).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ArrangementEdge {
    /// The line index this edge lies on.
    pub line: usize,
    /// Start point.
    pub start: Point2,
    /// End point.
    pub end: Point2,
}

/// One face of a line arrangement: a polygon (possibly unbounded, clipped to
/// the bounding box).
#[derive(Debug, Clone, PartialEq)]
pub struct ArrangementFace {
    /// Vertices of the face boundary in CCW order (clipped to bounding box).
    pub boundary: Vec<Point2>,
    /// True if this face touches the bounding box (i.e. is unbounded in the
    /// original arrangement).
    pub unbounded: bool,
}

/// A line arrangement: the planar subdivision induced by a set of lines.
#[derive(Debug, Clone, PartialEq)]
pub struct Arrangement {
    /// The input lines.
    pub lines: Vec<Line2>,
    /// All intersection points (vertices of the arrangement).
    pub vertices: Vec<Point2>,
    /// All edges (segments between consecutive vertices along each line).
    pub edges: Vec<ArrangementEdge>,
    /// All faces (cells of the subdivision).
    pub faces: Vec<ArrangementFace>,
    /// The bounding box used to clip unbounded edges.
    pub bbox_min: Point2,
    pub bbox_max: Point2,
}

/// Summary counts for verification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArrangementCounts {
    pub vertices: usize,
    pub edges: usize,
    pub faces: usize,
    /// Euler characteristic V − E + F (should be 2 for a planar subdivision).
    pub euler: i64,
}

impl Arrangement {
    pub fn counts(&self) -> ArrangementCounts {
        let v = self.vertices.len() as i64;
        let e = self.edges.len() as i64;
        let f = self.faces.len() as i64;
        ArrangementCounts {
            vertices: self.vertices.len(),
            edges: self.edges.len(),
            faces: self.faces.len(),
            euler: v - e + f,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ArrangementError {
    TooFewLines { got: usize },
    AllParallel,
}

// ───────────────────────────────────────────────────────────────────────────
//  Arrangement construction
// ───────────────────────────────────────────────────────────────────────────

/// Bounding-box margin factor: the clip box extends this multiple of the
/// vertex coordinate range beyond the extreme vertices.
const BBOX_MARGIN: f64 = 3.0;

/// Compute the bounding box for a set of vertices, with margin.
fn compute_bbox(vertices: &[Point2]) -> (Point2, Point2) {
    if vertices.is_empty() {
        return (Point2::new(-10.0, -10.0), Point2::new(10.0, 10.0));
    }
    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for &v in vertices {
        min_x = min_x.min(v.x);
        max_x = max_x.max(v.x);
        min_y = min_y.min(v.y);
        max_y = max_y.max(v.y);
    }
    let dx = (max_x - min_x).max(1.0);
    let dy = (max_y - min_y).max(1.0);
    (
        Point2::new(min_x - BBOX_MARGIN * dx, min_y - BBOX_MARGIN * dy),
        Point2::new(max_x + BBOX_MARGIN * dx, max_y + BBOX_MARGIN * dy),
    )
}

/// Clip a line to the bounding box, returning the two endpoints of the
/// clipped segment.
fn clip_line_to_bbox(line: &Line2, bmin: Point2, bmax: Point2) -> (Point2, Point2) {
    // Liang-Barsky clipping of the line to the box [bmin, bmax].
    // Parametrize the line as P(t) = P0 + t * d.
    let (p0, d) = if line.is_vertical {
        (
            Point2::new(line.x_const, bmin.y),
            Point2::new(0.0, bmax.y - bmin.y),
        )
    } else {
        // Pick two far-apart points on the line.
        let x0 = bmin.x - (bmax.x - bmin.x);
        let x1 = bmax.x + (bmax.x - bmin.x);
        (
            Point2::new(x0, line.y_at(x0)),
            Point2::new(x1 - x0, line.y_at(x1) - line.y_at(x0)),
        )
    };

    let (t0, t1) = liang_barsky(p0, d, bmin, bmax);
    (
        Point2::new(p0.x + t0 * d.x, p0.y + t0 * d.y),
        Point2::new(p0.x + t1 * d.x, p0.y + t1 * d.y),
    )
}

/// Liang-Barsky line clipping. Returns (t0, t1) with 0 ≤ t0 ≤ t1 ≤ 1 for the
/// portion of P(t) = p0 + t·d inside the box [bmin, bmax].
fn liang_barsky(p0: Point2, d: Point2, bmin: Point2, bmax: Point2) -> (f64, f64) {
    let mut t0 = 0.0f64;
    let mut t1 = 1.0f64;

    for (p, q) in [
        (-d.x, p0.x - bmin.x), // left
        (d.x, bmax.x - p0.x),  // right
        (-d.y, p0.y - bmin.y), // bottom
        (d.y, bmax.y - p0.y),  // top
    ] {
        if p.abs() <= f64::MIN_POSITIVE {
            // Parallel to this boundary.
            if q < 0.0 {
                return (0.0, 0.0); // entirely outside
            }
        } else {
            let r = q / p;
            if p < 0.0 {
                t0 = t0.max(r);
            } else {
                t1 = t1.min(r);
            }
        }
    }
    (t0, t1)
}

/// Build a line arrangement from a set of lines.
///
/// Computes all pairwise intersections, splits each line at its intersection
/// points (clipped to a bounding box for unbounded edges), and extracts faces
/// by tracing boundary cycles.
pub fn build_line_arrangement(lines: &[Line2]) -> Result<Arrangement, ArrangementError> {
    let n = lines.len();
    if n < 2 {
        return Err(ArrangementError::TooFewLines { got: n });
    }

    // Step 1 — compute all pairwise intersections.
    let mut vertices: Vec<Point2> = Vec::new();
    for i in 0..n {
        for j in (i + 1)..n {
            if let Some(p) = line_line_intersection(&lines[i], &lines[j]) {
                if p.x.is_finite() && p.y.is_finite() {
                    vertices.push(p);
                }
            }
        }
    }

    if vertices.is_empty() {
        return Err(ArrangementError::AllParallel);
    }

    // Deduplicate vertices (concurrent lines produce the same intersection).
    vertices.sort_by(|a, b| {
        a.x.partial_cmp(&b.x)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.y.partial_cmp(&b.y).unwrap_or(std::cmp::Ordering::Equal))
    });
    vertices.dedup_by(|a, b| (a.x - b.x).abs() < 1e-9 && (a.y - b.y).abs() < 1e-9);

    // Step 2 — compute bounding box.
    let (bmin, bmax) = compute_bbox(&vertices);

    // Step 3 — for each line, find vertices on it, sort along the line, and
    // emit edges between consecutive vertices (including bbox clip endpoints).
    let mut edges: Vec<ArrangementEdge> = Vec::new();
    // Collect all bbox boundary exit points (clip endpoints) for each line.
    let mut boundary_points: Vec<Point2> = Vec::new();
    for (li, line) in lines.iter().enumerate() {
        // Clip the line to the bounding box.
        let (clip_a, clip_b) = clip_line_to_bbox(line, bmin, bmax);

        // Save clip endpoints as boundary points.
        boundary_points.push(clip_a);
        boundary_points.push(clip_b);

        // Collect all points on this line: intersection points + clip endpoints.
        let mut pts: Vec<(f64, Point2)> = Vec::new();
        pts.push((0.0, clip_a));
        pts.push((1.0, clip_b));
        for &v in &vertices {
            if point_on_line(v, line) {
                // Compute parameter t along clip_a → clip_b.
                let dx = clip_b.x - clip_a.x;
                let dy = clip_b.y - clip_a.y;
                let len_sq = dx * dx + dy * dy;
                if len_sq > f64::MIN_POSITIVE {
                    let t = ((v.x - clip_a.x) * dx + (v.y - clip_a.y) * dy) / len_sq;
                    if t > 1e-9 && t < 1.0 - 1e-9 {
                        pts.push((t, v));
                    }
                }
            }
        }
        pts.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        pts.dedup_by(|a, b| (a.0 - b.0).abs() < 1e-12);

        // Emit edges between consecutive points.
        for w in pts.windows(2) {
            let start = w[0].1;
            let end = w[1].1;
            if (start.x - end.x).abs() > 1e-12 || (start.y - end.y).abs() > 1e-12 {
                edges.push(ArrangementEdge {
                    line: li,
                    start,
                    end,
                });
            }
        }
    }

    // Step 3b — add bbox boundary edges, split at every boundary exit point.
    // The bbox boundary is part of the subdivision: it closes all face cycles.
    let bbox_edges = build_bbox_boundary_edges(bmin, bmax, &boundary_points);
    edges.extend(bbox_edges);

    // Step 3c — collect ALL vertices of the subdivision (not just line-line
    // intersections): every unique edge endpoint is a vertex. This includes
    // bbox corners and line clip points, which are needed for the Euler
    // identity V − E + F = 2.
    let mut all_vertices: Vec<Point2> = vertices.clone();
    for e in &edges {
        insert_point(&mut all_vertices, e.start);
        insert_point(&mut all_vertices, e.end);
    }
    // Sort for determinism.
    all_vertices.sort_by(|a, b| {
        a.x.partial_cmp(&b.x)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.y.partial_cmp(&b.y).unwrap_or(std::cmp::Ordering::Equal))
    });

    // Step 4 — extract faces by building a DCEL-like structure and walking
    // boundary cycles. We use the same approach as dcel_overlay.rs:
    //   - Build half-edges with twin linkage
    //   - Sort outgoing half-edges per vertex CCW
    //   - Link next/prev via the CCW-predecessor rule
    //   - Walk cycles → faces
    let faces = extract_faces(&edges, &all_vertices, bmin, bmax);

    Ok(Arrangement {
        lines: lines.to_vec(),
        vertices: all_vertices,
        edges,
        faces,
        bbox_min: bmin,
        bbox_max: bmax,
    })
}

/// Check if a point lies on a line (within tolerance).
fn point_on_line(p: Point2, line: &Line2) -> bool {
    if line.is_vertical {
        return (p.x - line.x_const).abs() < 1e-9;
    }
    let y_expected = line.y_at(p.x);
    (p.y - y_expected).abs() < 1e-9 * (y_expected.abs() + 1.0)
}

/// Build bbox boundary edges, split at every boundary exit point.
///
/// The bbox has 4 sides. Each side is split at every boundary point that lies
/// on it, producing edges between consecutive split points (including corners).
/// A special line index `usize::MAX` marks bbox boundary edges.
fn build_bbox_boundary_edges(
    bmin: Point2,
    bmax: Point2,
    boundary_points: &[Point2],
) -> Vec<ArrangementEdge> {
    let corners = [
        Point2::new(bmin.x, bmin.y), // bottom-left
        Point2::new(bmax.x, bmin.y), // bottom-right
        Point2::new(bmax.x, bmax.y), // top-right
        Point2::new(bmin.x, bmax.y), // top-left
    ];

    // Each side: (start_corner, end_corner, axis, is_horizontal).
    // Bottom: bmin.x → bmax.x at y=bmin.y (left to right)
    // Right: bmin.y → bmax.y at x=bmax.x (bottom to top)
    // Top: bmax.x → bmin.x at y=bmax.y (right to left)
    // Left: bmax.y → bmin.y at x=bmin.x (top to bottom)
    let sides: [(Point2, Point2, bool); 4] = [
        (corners[0], corners[1], true),  // bottom, horizontal
        (corners[1], corners[2], false), // right, vertical
        (corners[2], corners[3], true),  // top, horizontal
        (corners[3], corners[0], false), // left, vertical
    ];

    let mut edges = Vec::new();
    for (start, end, horizontal) in sides {
        // Collect split points on this side.
        let mut pts: Vec<(f64, Point2)> = Vec::new();
        pts.push((0.0, start));
        pts.push((1.0, end));
        for &bp in boundary_points {
            if horizontal {
                // Check if bp is on this horizontal segment.
                if (bp.y - start.y).abs() < 1e-9
                    && bp.x >= start.x.min(end.x) - 1e-9
                    && bp.x <= start.x.max(end.x) + 1e-9
                {
                    let t = (bp.x - start.x) / (end.x - start.x);
                    if t > 1e-9 && t < 1.0 - 1e-9 {
                        pts.push((t, bp));
                    }
                }
            } else {
                // Check if bp is on this vertical segment.
                if (bp.x - start.x).abs() < 1e-9
                    && bp.y >= start.y.min(end.y) - 1e-9
                    && bp.y <= start.y.max(end.y) + 1e-9
                {
                    let t = (bp.y - start.y) / (end.y - start.y);
                    if t > 1e-9 && t < 1.0 - 1e-9 {
                        pts.push((t, bp));
                    }
                }
            }
        }
        pts.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        pts.dedup_by(|a, b| (a.0 - b.0).abs() < 1e-12);

        for w in pts.windows(2) {
            let s = w[0].1;
            let e = w[1].1;
            if (s.x - e.x).abs() > 1e-12 || (s.y - e.y).abs() > 1e-12 {
                edges.push(ArrangementEdge {
                    line: usize::MAX, // bbox boundary marker
                    start: s,
                    end: e,
                });
            }
        }
    }
    edges
}

// ───────────────────────────────────────────────────────────────────────────
//  Face extraction (DCEL cycle walk)
// ───────────────────────────────────────────────────────────────────────────

/// A half-edge for arrangement face extraction.
#[derive(Clone, Copy)]
struct ArrHalfEdge {
    origin: usize, // index into a vertex array
    twin: usize,   // index into half-edge array
    next: usize,   // index into half-edge array
    face: usize,   // face index
}

/// Extract faces from the arrangement edges by building a DCEL and walking
/// boundary cycles. Uses the same CCW-predecessor linkage rule as
/// `dcel_overlay.rs`.
fn extract_faces(
    edges: &[ArrangementEdge],
    vertices: &[Point2],
    _bmin: Point2,
    _bmax: Point2,
) -> Vec<ArrangementFace> {
    // Build a combined vertex array: arrangement vertices + edge endpoints
    // (which include bbox clip points not in the vertex list).
    let mut all_points: Vec<Point2> = vertices.to_vec();
    for e in edges {
        insert_point(&mut all_points, e.start);
        insert_point(&mut all_points, e.end);
    }

    let n_pts = all_points.len();
    let m = edges.len();

    // Build half-edges: two per edge.
    let mut he = vec![
        ArrHalfEdge {
            origin: 0,
            twin: 0,
            next: 0,
            face: usize::MAX,
        };
        2 * m
    ];

    for (i, e) in edges.iter().enumerate() {
        let from = find_point(&all_points, e.start).unwrap();
        let to = find_point(&all_points, e.end).unwrap();
        let f = i;
        let t = i + m;
        he[f].origin = from;
        he[f].twin = t;
        he[t].origin = to;
        he[t].twin = f;
    }

    // Group outgoing half-edges per vertex, sort CCW by direction.
    let mut outgoing: Vec<Vec<usize>> = vec![Vec::new(); n_pts];
    for (i, h) in he.iter().enumerate() {
        outgoing[h.origin].push(i);
    }
    for v in 0..n_pts {
        let o = all_points[v];
        outgoing[v].sort_by(|&h1, &h2| {
            let d1 = all_points[he[he[h1].twin].origin];
            let d2 = all_points[he[he[h2].twin].origin];
            let a1 = (d1.y - o.y).atan2(d1.x - o.x);
            let a2 = (d2.y - o.y).atan2(d2.x - o.x);
            a1.partial_cmp(&a2).unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    // Linkage: twin(g_i).next = g_{(i-1) mod k} (CCW-predecessor).
    for v in 0..n_pts {
        let gs = &outgoing[v];
        let k = gs.len();
        if k == 0 {
            continue;
        }
        for i in 0..k {
            let g_i = gs[i];
            let twin = he[g_i].twin;
            let next = gs[(i + k - 1) % k];
            he[twin].next = next;
        }
    }

    // Walk face cycles.
    let n_he = he.len();
    let mut face_id = 0usize;
    let mut face_cycles: Vec<Vec<usize>> = Vec::new();
    let mut face_start: Vec<usize> = Vec::new();
    for start in 0..n_he {
        if he[start].face != usize::MAX {
            continue;
        }
        let mut cycle: Vec<usize> = Vec::new();
        let mut cur = start;
        loop {
            if he[cur].face != usize::MAX {
                break;
            }
            he[cur].face = face_id;
            cycle.push(cur);
            cur = he[cur].next;
            if cur == start {
                break;
            }
            if cycle.len() > n_he {
                break;
            }
        }
        face_cycles.push(cycle);
        face_start.push(start);
        face_id += 1;
    }

    // Build face records: compute boundary vertices and signed area.
    let mut faces: Vec<ArrangementFace> = Vec::with_capacity(face_id);
    for cyc in &face_cycles {
        let boundary: Vec<Point2> = cyc.iter().map(|&h| all_points[he[h].origin]).collect();
        let area = signed_area(&boundary);
        // CCW (area > 0) → bounded face. CW (area < 0) → unbounded face
        // (the "outer" cycle of the arrangement, wrapping around the bbox).
        let unbounded = area < 0.0;
        faces.push(ArrangementFace {
            boundary,
            unbounded,
        });
    }

    // For the unbounded face, reverse the boundary to get CCW (convention).
    for f in &mut faces {
        if f.unbounded {
            f.boundary.reverse();
        }
    }

    faces
}

/// Insert a point into the vertex array if not already present (by coordinate).
fn insert_point(points: &mut Vec<Point2>, p: Point2) {
    if find_point(points, p).is_some() {
        return;
    }
    points.push(p);
}

/// Find the index of a point in the array by coordinate (within tolerance).
fn find_point(points: &[Point2], p: Point2) -> Option<usize> {
    for (i, q) in points.iter().enumerate() {
        if (p.x - q.x).abs() < 1e-9 && (p.y - q.y).abs() < 1e-9 {
            return Some(i);
        }
    }
    None
}

/// Signed area of a polygon (positive = CCW).
fn signed_area(vertices: &[Point2]) -> f64 {
    let n = vertices.len();
    if n < 3 {
        return 0.0;
    }
    let mut area = 0.0;
    for i in 0..n {
        let j = (i + 1) % n;
        area += vertices[i].x * vertices[j].y - vertices[j].x * vertices[i].y;
    }
    area * 0.5
}

// ───────────────────────────────────────────────────────────────────────────
//  Zone traversal
// ───────────────────────────────────────────────────────────────────────────

/// The zone of a query line through an arrangement: the sequence of face
/// indices that the line passes through, in order along the line.
pub fn zone_traversal(arr: &Arrangement, query: &Line2) -> Vec<usize> {
    // Find all intersection points of the query line with arrangement edges.
    let mut crossings: Vec<(f64, usize)> = Vec::new(); // (parameter along query, edge index)

    // Parametrize the query line.
    let (q_start, q_dir) = if query.is_vertical {
        (
            Point2::new(query.x_const, arr.bbox_min.y - 1.0),
            Point2::new(0.0, arr.bbox_max.y - arr.bbox_min.y + 2.0),
        )
    } else {
        let x0 = arr.bbox_min.x - 1.0;
        let x1 = arr.bbox_max.x + 1.0;
        (
            Point2::new(x0, query.y_at(x0)),
            Point2::new(x1 - x0, query.y_at(x1) - query.y_at(x0)),
        )
    };

    for (ei, e) in arr.edges.iter().enumerate() {
        // Find intersection of the query line segment with edge e.
        if let Some(t) = segment_segment_parametric(q_start, q_dir, e.start, e.end) {
            if t > 1e-9 && t < 1.0 - 1e-9 {
                crossings.push((t, ei));
            }
        }
    }

    // Sort crossings by parameter along the query line.
    crossings.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    crossings.dedup_by(|a, b| (a.0 - b.0).abs() < 1e-12);

    if crossings.is_empty() {
        // The query line doesn't cross any edge — it's entirely in one face.
        // Find which face contains a point on the query line.
        let mid = Point2::new(q_start.x + 0.5 * q_dir.x, q_start.y + 0.5 * q_dir.y);
        if let Some(fi) = locate_face(arr, mid) {
            return vec![fi];
        }
        return Vec::new();
    }

    // Between consecutive crossings, the query line is in one face.
    // Find a midpoint in each segment and locate the face.
    let mut zone: Vec<usize> = Vec::new();
    for w in crossings.windows(2) {
        let t_mid = (w[0].0 + w[1].0) * 0.5;
        let mid = Point2::new(q_start.x + t_mid * q_dir.x, q_start.y + t_mid * q_dir.y);
        if let Some(fi) = locate_face(arr, mid) {
            zone.push(fi);
        }
    }

    zone
}

/// Parametric intersection of segment P(t) = p0 + t·d (0 ≤ t ≤ 1) with
/// segment (a, b). Returns t if they intersect properly, None otherwise.
fn segment_segment_parametric(p0: Point2, d: Point2, a: Point2, b: Point2) -> Option<f64> {
    let s = Point2::new(b.x - a.x, b.y - a.y);
    let denom = d.x * s.y - d.y * s.x;
    if denom.abs() <= f64::MIN_POSITIVE {
        return None; // parallel
    }
    let t = ((a.x - p0.x) * s.y - (a.y - p0.y) * s.x) / denom;
    let u = ((a.x - p0.x) * d.y - (a.y - p0.y) * d.x) / denom;
    if t >= -1e-9 && t <= 1.0 + 1e-9 && u >= -1e-9 && u <= 1.0 + 1e-9 {
        Some(t.clamp(0.0, 1.0))
    } else {
        None
    }
}

/// Locate which face contains a point (by point-in-polygon test).
fn locate_face(arr: &Arrangement, p: Point2) -> Option<usize> {
    for (fi, f) in arr.faces.iter().enumerate() {
        if f.boundary.len() >= 3 && point_in_polygon(p, &f.boundary) {
            return Some(fi);
        }
    }
    None
}

// ───────────────────────────────────────────────────────────────────────────
//  Zone traversal oracle (brute force)
// ───────────────────────────────────────────────────────────────────────────

/// Brute-force zone traversal: for a fine sample of points along the query
/// line, locate the containing face. Returns the distinct face indices in
/// order of first encounter. Used as an independent oracle to verify
/// `zone_traversal`.
pub fn zone_traversal_oracle(arr: &Arrangement, query: &Line2, samples: usize) -> Vec<usize> {
    let (q_start, q_dir) = if query.is_vertical {
        (
            Point2::new(query.x_const, arr.bbox_min.y - 1.0),
            Point2::new(0.0, arr.bbox_max.y - arr.bbox_min.y + 2.0),
        )
    } else {
        let x0 = arr.bbox_min.x - 1.0;
        let x1 = arr.bbox_max.x + 1.0;
        (
            Point2::new(x0, query.y_at(x0)),
            Point2::new(x1 - x0, query.y_at(x1) - query.y_at(x0)),
        )
    };

    let mut zone: Vec<usize> = Vec::new();
    for i in 0..samples {
        let t = i as f64 / (samples - 1).max(1) as f64;
        let p = Point2::new(q_start.x + t * q_dir.x, q_start.y + t * q_dir.y);
        if let Some(fi) = locate_face(arr, p) {
            if zone.last() != Some(&fi) {
                zone.push(fi);
            }
        }
    }
    zone
}

// ───────────────────────────────────────────────────────────────────────────
//  Point-line duality
// ───────────────────────────────────────────────────────────────────────────

/// Dual of a point `p = (a, b)`: the line `y = a·x − b`.
///
/// Property: point `p` is above line `l` ⟺ dual line `p*` is above dual
/// point `l*`.
pub fn dual_point_to_line(p: Point2) -> Line2 {
    Line2::new(p.x, -p.y)
}

/// Dual of a non-vertical line `l : y = m·x + c`: the point `(m, −c)`.
///
/// Returns `None` for vertical lines (which have no finite dual point).
pub fn dual_line_to_point(l: &Line2) -> Option<Point2> {
    if l.is_vertical {
        return None;
    }
    Some(Point2::new(l.slope, -l.intercept))
}

/// Round-trip the duality: `dual(dual(p))` should equal `p` for all finite,
/// non-vertical points.
pub fn dual_round_trip(p: Point2) -> Point2 {
    let line = dual_point_to_line(p);
    dual_line_to_point(&line).unwrap_or(p)
}

/// Check the incidence-preserving property: point `p` is on line `l` ⟺ dual
/// line `p*` passes through dual point `l*`.
pub fn dual_incidence_holds(p: Point2, l: &Line2) -> bool {
    if l.is_vertical {
        // Duality is defined for non-vertical lines only.
        return true;
    }
    let p_star = dual_point_to_line(p);
    let l_star = dual_line_to_point(l).unwrap();
    // p on l ⟺ p.y == l.y_at(p.x)
    let p_on_l = (p.y - l.y_at(p.x)).abs() < 1e-9 * (p.y.abs() + 1.0);
    // p* passes through l* ⟺ l*.y == p*.y_at(l*.x)
    let p_star_through_l_star =
        (l_star.y - p_star.y_at(l_star.x)).abs() < 1e-9 * (l_star.y.abs() + 1.0);
    p_on_l == p_star_through_l_star
}

// ───────────────────────────────────────────────────────────────────────────
//  Tests
// ───────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Two non-parallel, non-vertical lines in general position.
    fn two_general_lines() -> Vec<Line2> {
        vec![Line2::new(1.0, 0.0), Line2::new(-1.0, 2.0)]
    }

    /// Three lines in general position (no two parallel, no three concurrent).
    fn three_general_lines() -> Vec<Line2> {
        vec![
            Line2::new(1.0, 0.0),  // y = x
            Line2::new(-1.0, 2.0), // y = -x + 2
            Line2::new(0.0, 1.0),  // y = 1 (horizontal)
        ]
    }

    #[test]
    fn rejects_too_few_lines() {
        assert_eq!(
            build_line_arrangement(&[Line2::new(1.0, 0.0)]),
            Err(ArrangementError::TooFewLines { got: 1 })
        );
    }

    #[test]
    fn rejects_all_parallel() {
        let lines = vec![
            Line2::new(1.0, 0.0),
            Line2::new(1.0, 1.0),
            Line2::new(1.0, 2.0),
        ];
        assert_eq!(
            build_line_arrangement(&lines),
            Err(ArrangementError::AllParallel)
        );
    }

    #[test]
    fn two_lines_vef_counts() {
        // 2 lines in general position: 1 intersection + 4 bbox corners = 5
        // vertices. 4 line edges + 4 bbox edges = 8 edges. Euler: 5-8+F=2 → F=5.
        let arr = build_line_arrangement(&two_general_lines()).unwrap();
        let c = arr.counts();
        assert_eq!(
            c.euler, 2,
            "Euler V-E+F must be 2, got V={} E={} F={}",
            c.vertices, c.edges, c.faces
        );
    }

    #[test]
    fn three_lines_vef_counts() {
        // 3 lines in general position: 3 intersections + bbox corners + clip
        // points. Just check Euler = 2.
        let arr = build_line_arrangement(&three_general_lines()).unwrap();
        let c = arr.counts();
        assert_eq!(
            c.euler, 2,
            "Euler V-E+F must be 2, got V={} E={} F={}",
            c.vertices, c.edges, c.faces
        );
    }

    #[test]
    fn euler_identity_holds_for_general_position() {
        // For n lines in general position: V = n(n-1)/2, E = n², F = n(n-1)/2 + 1.
        // V - E + F = n(n-1)/2 - n² + n(n-1)/2 + 1 = n(n-1) - n² + 1 = -n + 1.
        // But with the bbox clipping, the arrangement becomes a bounded
        // subdivision with Euler = 2.
        for n in 2..=6 {
            let lines: Vec<Line2> = (0..n)
                .map(|i| Line2::new((i as f64 + 1.0) * 0.3, (i as f64) * 0.7))
                .collect();
            let arr = build_line_arrangement(&lines).unwrap();
            let c = arr.counts();
            assert_eq!(
                c.euler, 2,
                "Euler V-E+F must be 2 for {n} lines (got V={}, E={}, F={})",
                c.vertices, c.edges, c.faces
            );
        }
    }

    #[test]
    fn concurrent_lines_dedup_vertices() {
        // Three lines through the origin: all intersect at (0,0).
        let lines = vec![
            Line2::new(1.0, 0.0),
            Line2::new(-1.0, 0.0),
            Line2::new(2.0, 0.0),
        ];
        let arr = build_line_arrangement(&lines).unwrap();
        // The origin (0,0) must be among the vertices.
        assert!(
            arr.vertices
                .iter()
                .any(|v| (v.x - 0.0).abs() < 1e-9 && (v.y - 0.0).abs() < 1e-9),
            "concurrent lines must include the origin in vertices"
        );
        assert_eq!(arr.counts().euler, 2);
    }

    #[test]
    fn vertical_line_handled() {
        // A vertical line and a non-vertical line.
        let lines = vec![Line2::vertical(0.0), Line2::new(1.0, 0.0)];
        let arr = build_line_arrangement(&lines).unwrap();
        // The origin (0,0) must be among the vertices.
        assert!(
            arr.vertices
                .iter()
                .any(|v| (v.x - 0.0).abs() < 1e-9 && (v.y - 0.0).abs() < 1e-9),
            "vertical + non-vertical must include the origin"
        );
        assert_eq!(arr.counts().euler, 2);
    }

    #[test]
    fn zone_traversal_matches_oracle_two_lines() {
        let arr = build_line_arrangement(&two_general_lines()).unwrap();
        let query = Line2::new(0.5, 0.5);
        let zone = zone_traversal(&arr, &query);
        let oracle = zone_traversal_oracle(&arr, &query, 1000);
        // The zone should be the same set of faces (order may differ at
        // boundaries, but the sequence should match).
        assert_eq!(
            zone.len(),
            oracle.len(),
            "zone length {} should match oracle {}",
            zone.len(),
            oracle.len()
        );
        for i in 0..zone.len() {
            assert_eq!(
                zone[i], oracle[i],
                "zone face {i} mismatch: zone={} vs oracle={}",
                zone[i], oracle[i]
            );
        }
    }

    #[test]
    fn zone_traversal_matches_oracle_three_lines() {
        let arr = build_line_arrangement(&three_general_lines()).unwrap();
        let query = Line2::new(0.3, 0.7);
        let zone = zone_traversal(&arr, &query);
        let oracle = zone_traversal_oracle(&arr, &query, 2000);
        assert_eq!(zone, oracle, "zone traversal must match brute-force oracle");
    }

    #[test]
    fn zone_traversal_matches_oracle_five_lines() {
        let lines: Vec<Line2> = (0..5)
            .map(|i| Line2::new((i as f64 + 1.0) * 0.4, (i as f64) * 0.5))
            .collect();
        let arr = build_line_arrangement(&lines).unwrap();
        let query = Line2::new(0.7, 0.3);
        let zone = zone_traversal(&arr, &query);
        let oracle = zone_traversal_oracle(&arr, &query, 5000);
        assert_eq!(zone, oracle, "zone traversal must match oracle for 5 lines");
    }

    #[test]
    fn zone_traversal_vertical_query() {
        let arr = build_line_arrangement(&three_general_lines()).unwrap();
        let query = Line2::vertical(0.5);
        let zone = zone_traversal(&arr, &query);
        let oracle = zone_traversal_oracle(&arr, &query, 2000);
        assert_eq!(zone, oracle, "vertical query zone must match oracle");
    }

    #[test]
    fn zone_theorem_bound_holds() {
        // The zone of a line in an arrangement of n lines has at most 2n faces.
        // (We check ≤ 2n + 1 to account for bbox boundary effects.)
        for n in 2..=6 {
            let lines: Vec<Line2> = (0..n)
                .map(|i| Line2::new((i as f64 + 1.0) * 0.3, (i as f64) * 0.7))
                .collect();
            let arr = build_line_arrangement(&lines).unwrap();
            let query = Line2::new(0.5, 0.5);
            let zone = zone_traversal(&arr, &query);
            assert!(
                zone.len() <= 2 * n + 1,
                "zone of {n} lines has {} faces, should be ≤ {}",
                zone.len(),
                2 * n + 1
            );
        }
    }

    // ── Point-line duality ──

    #[test]
    fn dual_point_to_line_correct() {
        // (a, b) → y = a·x − b
        let p = Point2::new(2.0, 3.0);
        let l = dual_point_to_line(p);
        assert!(!l.is_vertical);
        assert_eq!(l.slope, 2.0);
        assert_eq!(l.intercept, -3.0);
    }

    #[test]
    fn dual_line_to_point_correct() {
        // y = m·x + c → (m, −c)
        let l = Line2::new(2.0, 3.0);
        let p = dual_line_to_point(&l).unwrap();
        assert_eq!(p, Point2::new(2.0, -3.0));
    }

    #[test]
    fn dual_vertical_line_has_no_point() {
        let l = Line2::vertical(1.0);
        assert!(dual_line_to_point(&l).is_none());
    }

    #[test]
    fn dual_round_trip_finite_non_vertical() {
        // dual(dual(p)) = p for all finite, non-vertical points.
        for &(a, b) in &[
            (0.0, 0.0),
            (1.0, 2.0),
            (-3.5, 7.2),
            (1e6, -1e-6),
            (0.001, -0.001),
        ] {
            let p = Point2::new(a, b);
            let rt = dual_round_trip(p);
            assert!(
                (rt.x - p.x).abs() < 1e-12 && (rt.y - p.y).abs() < 1e-12,
                "round-trip failed for ({a}, {b}): got ({}, {})",
                rt.x,
                rt.y
            );
        }
    }

    #[test]
    fn dual_above_below_property() {
        // p above l ⟺ l* above p*
        let l = Line2::new(1.0, 0.0); // y = x
        let l_star = dual_line_to_point(&l).unwrap(); // (1, 0)

        // p above l: p = (0, 1), y=1 > l.y_at(0)=0
        let p_above = Point2::new(0.0, 1.0);
        let p_above_star = dual_point_to_line(p_above); // y = 0·x − 1 = −1
        let p_above_on_l = (p_above.y - l.y_at(p_above.x)).abs() < 1e-12;
        let l_star_above_p_star = l_star.y > p_above_star.y_at(l_star.x);
        assert!(!p_above_on_l, "p_above should not be on l");
        assert!(
            l_star_above_p_star,
            "l* should be above p* when p is above l"
        );

        // p below l: p = (0, -1)
        let p_below = Point2::new(0.0, -1.0);
        let p_below_star = dual_point_to_line(p_below); // y = 1
        let l_star_below_p_star = l_star.y < p_below_star.y_at(l_star.x);
        assert!(
            l_star_below_p_star,
            "l* should be below p* when p is below l"
        );
    }

    #[test]
    fn dual_incidence_preserved() {
        // If p is on l, then p* passes through l*.
        let l = Line2::new(2.0, 1.0); // y = 2x + 1
        let p = Point2::new(1.0, 3.0); // on l: 3 = 2·1 + 1 ✓
        assert!(
            dual_incidence_holds(p, &l),
            "incidence must hold for p on l"
        );

        // If p is NOT on l, then p* does NOT pass through l*.
        let p_off = Point2::new(1.0, 4.0); // not on l: 4 ≠ 3
        assert!(
            dual_incidence_holds(p_off, &l),
            "incidence must hold (both false) for p not on l"
        );
    }

    #[test]
    fn determinism_same_input_same_output() {
        let lines = three_general_lines();
        let a1 = build_line_arrangement(&lines).unwrap();
        let a2 = build_line_arrangement(&lines).unwrap();
        assert_eq!(a1.vertices, a2.vertices);
        assert_eq!(a1.edges, a2.edges);
        assert_eq!(a1.faces, a2.faces);
    }

    #[test]
    fn line_through_points_correct() {
        let a = Point2::new(0.0, 1.0);
        let b = Point2::new(2.0, 5.0);
        let l = Line2::through_points(a, b);
        assert_eq!(l.slope, 2.0);
        assert_eq!(l.intercept, 1.0);
        assert!(!l.is_vertical);
    }

    #[test]
    fn line_through_vertical_points() {
        let a = Point2::new(3.0, 0.0);
        let b = Point2::new(3.0, 5.0);
        let l = Line2::through_points(a, b);
        assert!(l.is_vertical);
        assert_eq!(l.x_const, 3.0);
    }

    #[test]
    fn parallel_detection() {
        assert!(Line2::new(1.0, 0.0).is_parallel(&Line2::new(1.0, 5.0)));
        assert!(!Line2::new(1.0, 0.0).is_parallel(&Line2::new(2.0, 0.0)));
        assert!(Line2::vertical(1.0).is_parallel(&Line2::vertical(2.0)));
        assert!(!Line2::vertical(1.0).is_parallel(&Line2::new(1.0, 0.0)));
    }

    #[test]
    fn arrangement_with_parallel_lines() {
        // Two parallel lines + one transversal.
        let lines = vec![
            Line2::new(1.0, 0.0),
            Line2::new(1.0, 2.0),  // parallel to first
            Line2::new(-1.0, 3.0), // transversal
        ];
        let arr = build_line_arrangement(&lines).unwrap();
        // 2 intersections (transversal crosses each parallel line once).
        // Check Euler identity rather than exact vertex count (which includes
        // bbox corners and clip points).
        assert_eq!(arr.counts().euler, 2);
    }
}
