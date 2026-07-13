//! P11.3 — DCEL subdivision, overlay, and full polygon-set boolean output.
//!
//! The acceptance gate requires: "Overlay labels faces and returns boundary
//! cycles with holes for union/intersection/difference/xor; Euler and area
//! identities hold."
//!
//! ## Algorithms
//!
//! This is the textbook planar-overlay construction (de Berg Ch. 2):
//!
//! 1. **Vertex merge.** Collect every vertex of A and B plus every edge-edge
//!    intersection point (proper crossings, T-junctions, shared endpoints).
//!    Deduplicate by coordinate into a single indexed vertex table.
//! 2. **Edge split.** For each edge of A and B, find every merged vertex that
//!    lies on it, sort those vertices along the edge, and emit the sub-edges
//!    between consecutive split points. Zero-length sub-edges are dropped.
//!    Each sub-edge produces two half-edges (the edge and its twin).
//! 3. **DCEL linkage.** At every vertex, sort the outgoing half-edges
//!    counter-clockwise by direction. The standard linkage rule is applied:
//!    if the outgoing half-edges at a vertex are `g_0 … g_{m-1}` in CCW order,
//!    then `twin(g_i).next = g_{(i+1) mod m}`. This makes every face boundary
//!    a closed `next`-cycle with the face interior on the left.
//! 4. **Face walk.** Every half-edge not yet assigned to a face starts a new
//!    face cycle (follow `next` until returning to the start). Each cycle's
//!    signed area classifies it: positive → CCW outer boundary of a bounded
//!    face; negative → CW hole (of a bounded face, or a component of the
//!    unbounded face).
//! 5. **Hole nesting.** Each CW cycle is nested inside the smallest CCW cycle
//!    that contains a representative point of it; that CCW cycle's face owns
//!    the hole. CW cycles contained in no CCW cycle belong to the unbounded
//!    face.
//! 6. **Face labelling.** A representative point strictly inside each bounded
//!    face (the bottom-most outer vertex nudged along the inward bisector) is
//!    tested against A and B with the even-odd rule, yielding `(in_a, in_b)`.
//!    The unbounded face is `(false, false)`.
//! 7. **Boolean extraction.** For the requested op the selected faces' outer
//!    boundaries and holes are emitted as `PolygonWithHoles` components.
//!
//! ## Robustness
//!
//! Edge-edge intersection detection reuses the exact orientation predicate
//! ([`super::primitives::orientation_2`]) via
//! [`super::segment_intersection_2::classify_segment_intersection_2`]. The
//! constructed intersection point is the parametric intersection of the two
//! supporting lines. On-segment tests use a bounded tolerance scaled to the
//! edge length so that constructed points re-predicate as on-segment.
//!
//! ## Zero-heap contract
//!
//! This is a Tier-2 cold-construction module (AGENTS.md §0-A): `Vec` is used
//! during construction; the public output is typed structs. The hot predicate
//! path (orientation) is zero-heap.

use super::boolean_2::{point_in_polygon, polygon_signed_area};
use super::primitives::{orientation_2, Point2};
use super::segment_intersection_2::{classify_segment_intersection_2, SegmentIntersectionClass};

// ───────────────────────────────────────────────────────────────────────────
//  Tolerances
// ───────────────────────────────────────────────────────────────────────────

/// Coordinate equality tolerance for vertex deduplication. Scaled to be safe
/// for inputs with unit-order coordinates; callers requiring finer resolution
/// should pre-scale their input.
const VERTEX_EPS: f64 = 1e-9;

/// On-segment distance tolerance: a point is considered on a segment when its
/// squared distance to the segment is below `ON_SEGMENT_EPS * edge_len_sq`.
const ON_SEGMENT_EPS: f64 = 1e-12;

// ───────────────────────────────────────────────────────────────────────────
//  Types
// ───────────────────────────────────────────────────────────────────────────

/// One vertex of the DCEL: a coordinate plus any incident outgoing half-edge.
#[derive(Debug, Clone, PartialEq)]
pub struct DcelVertex {
    pub point: Point2,
    /// Any one half-edge whose origin is this vertex (`u32::MAX` if isolated).
    pub incident: u32,
}

/// One directed half-edge of the DCEL.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DcelHalfEdge {
    pub origin: u32,
    pub twin: u32,
    pub next: u32,
    pub prev: u32,
    pub face: u32,
}

impl Default for DcelHalfEdge {
    fn default() -> Self {
        Self {
            origin: u32::MAX,
            twin: u32::MAX,
            next: u32::MAX,
            prev: u32::MAX,
            face: u32::MAX,
        }
    }
}

/// Per-face membership label produced by the overlay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FaceLabel {
    pub in_a: bool,
    pub in_b: bool,
}

impl FaceLabel {
    /// Does this face belong to the result of `op`?
    #[inline]
    pub fn selected(self, op: BooleanOp) -> bool {
        match op {
            BooleanOp::Union => self.in_a || self.in_b,
            BooleanOp::Intersection => self.in_a && self.in_b,
            BooleanOp::Difference => self.in_a && !self.in_b,
            BooleanOp::Xor => self.in_a ^ self.in_b,
        }
    }
}

/// One face of the DCEL.
#[derive(Debug, Clone, PartialEq)]
pub struct DcelFace {
    /// A half-edge on the outer boundary (CCW), or `u32::MAX` for the unbounded
    /// face (which has no outer CCW component).
    pub outer: u32,
    /// Half-edges on the hole boundaries (CW cycles nested inside `outer`).
    pub holes: Vec<u32>,
    /// Membership label.
    pub label: FaceLabel,
    /// True for bounded faces (those with a CCW outer component).
    pub bounded: bool,
}

/// A planar subdivision stored as a doubly-connected edge list.
#[derive(Debug, Clone, PartialEq)]
pub struct Dcel {
    pub vertices: Vec<DcelVertex>,
    pub half_edges: Vec<DcelHalfEdge>,
    pub faces: Vec<DcelFace>,
}

/// The four polygon-set boolean operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BooleanOp {
    Union,
    Intersection,
    Difference,
    Xor,
}

/// A simple polygon outer boundary plus its holes (each hole a simple polygon).
#[derive(Debug, Clone, PartialEq)]
pub struct PolygonWithHoles {
    /// Outer boundary, CCW.
    pub outer: Vec<Point2>,
    /// Holes, CW.
    pub holes: Vec<Vec<Point2>>,
}

/// The full overlay result: the labelled subdivision plus the boolean
/// components for the requested operation.
#[derive(Debug, Clone, PartialEq)]
pub struct OverlayResult {
    pub dcel: Dcel,
    /// Components of `op(A, B)`, each a polygon-with-holes.
    pub components: Vec<PolygonWithHoles>,
    /// Euler characteristic V − E + F of the subdivision (including the
    /// unbounded face).
    pub euler: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DcelError {
    TooFewVertices { got: usize },
    DegenerateInput,
}

// ───────────────────────────────────────────────────────────────────────────
//  Small geometry helpers
// ───────────────────────────────────────────────────────────────────────────

#[inline]
fn approx_eq(a: Point2, b: Point2) -> bool {
    (a.x - b.x).abs() <= VERTEX_EPS && (a.y - b.y).abs() <= VERTEX_EPS
}

#[inline]
fn edge_len_sq(a: Point2, b: Point2) -> f64 {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    dx * dx + dy * dy
}

/// Squared distance from `p` to segment `ab`.
fn point_segment_dist_sq(p: Point2, a: Point2, b: Point2) -> f64 {
    let len_sq = edge_len_sq(a, b);
    if len_sq <= f64::MIN_POSITIVE {
        return edge_len_sq(p, a);
    }
    let t = ((p.x - a.x) * (b.x - a.x) + (p.y - a.y) * (b.y - a.y)) / len_sq;
    let t = t.clamp(0.0, 1.0);
    let cx = a.x + t * (b.x - a.x);
    let cy = a.y + t * (b.y - a.y);
    (p.x - cx) * (p.x - cx) + (p.y - cy) * (p.y - cy)
}

/// Is `p` on the closed segment `ab` (within a length-scaled tolerance)?
fn on_segment(p: Point2, a: Point2, b: Point2) -> bool {
    if approx_eq(p, a) || approx_eq(p, b) {
        return true;
    }
    // Must be collinear.
    if orientation_2(a, b, p) != super::primitives::Orientation::Collinear {
        return false;
    }
    let len_sq = edge_len_sq(a, b);
    if len_sq <= f64::MIN_POSITIVE {
        return approx_eq(p, a);
    }
    point_segment_dist_sq(p, a, b) <= ON_SEGMENT_EPS * len_sq
}

/// Parametric intersection point of two non-parallel segments' supporting
/// lines, clamped to lie on both segments. Returns `None` for parallel lines.
fn line_intersection(a1: Point2, a2: Point2, b1: Point2, b2: Point2) -> Option<Point2> {
    let d1x = a2.x - a1.x;
    let d1y = a2.y - a1.y;
    let d2x = b2.x - b1.x;
    let d2y = b2.y - b1.y;
    let denom = d1x * d2y - d1y * d2x;
    if denom.abs() <= f64::MIN_POSITIVE {
        return None;
    }
    let t = ((b1.x - a1.x) * d2y - (b1.y - a1.y) * d2x) / denom;
    Some(Point2::new(a1.x + t * d1x, a1.y + t * d1y))
}

/// Signed area of a half-edge cycle (summed shoelace over the walked vertices).
fn cycle_signed_area(cycle: &[Point2]) -> f64 {
    polygon_signed_area(cycle)
}

// ───────────────────────────────────────────────────────────────────────────
//  Step 1 — vertex merge
// ───────────────────────────────────────────────────────────────────────────

/// A coordinate-indexed vertex table that deduplicates by `VERTEX_EPS`.
struct VertexMerger {
    points: Vec<Point2>,
}

impl VertexMerger {
    fn new() -> Self {
        Self { points: Vec::new() }
    }

    /// Insert `p`, returning the index of the (possibly pre-existing) vertex.
    fn insert(&mut self, p: Point2) -> u32 {
        for (i, q) in self.points.iter().enumerate() {
            if approx_eq(*q, p) {
                return i as u32;
            }
        }
        let i = self.points.len() as u32;
        self.points.push(p);
        i
    }

    fn len(&self) -> usize {
        self.points.len()
    }
}

// ───────────────────────────────────────────────────────────────────────────
//  Step 2 — edge split
// ───────────────────────────────────────────────────────────────────────────

/// A directed sub-edge between two merged vertex indices.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SubEdge {
    from: u32,
    to: u32,
}

/// Compute the proper intersection point of two segments `ab` and `cd` when
/// they cross or touch (T-junction). Returns the constructed point, or `None`
/// for disjoint / collinear-overlap (whose endpoints are already merged).
fn intersection_point(a: Point2, b: Point2, c: Point2, d: Point2) -> Option<Point2> {
    let class = classify_segment_intersection_2(a, b, c, d).class;
    match class {
        SegmentIntersectionClass::Disjoint | SegmentIntersectionClass::CollinearDisjoint => None,
        // Shared endpoint or collinear touch → the touching point is an
        // endpoint, already in the vertex table.
        SegmentIntersectionClass::Endpoint | SegmentIntersectionClass::CollinearTouch => {
            // Identify which endpoint is shared.
            if approx_eq(a, c) {
                Some(a)
            } else if approx_eq(a, d) {
                Some(a)
            } else if approx_eq(b, c) {
                Some(b)
            } else if approx_eq(b, d) {
                Some(b)
            } else {
                None
            }
        }
        // T-junction: an endpoint of one segment lies on the other.
        SegmentIntersectionClass::TJunction(side) => {
            use super::segment_intersection_2::TJunctionSide;
            match side {
                TJunctionSide::AbOnCd => {
                    if on_segment(a, c, d) {
                        Some(a)
                    } else if on_segment(b, c, d) {
                        Some(b)
                    } else {
                        None
                    }
                }
                TJunctionSide::CdOnAb => {
                    if on_segment(c, a, b) {
                        Some(c)
                    } else if on_segment(d, a, b) {
                        Some(d)
                    } else {
                        None
                    }
                }
            }
        }
        // Proper crossing or collinear overlap → parametric point (for proper)
        // or nothing (overlap endpoints already merged).
        SegmentIntersectionClass::Proper => line_intersection(a, b, c, d),
        SegmentIntersectionClass::CollinearOverlap => None,
    }
}

/// Collect every intersection point between edges of `a` and edges of `b`.
fn collect_intersections(a: &[Point2], b: &[Point2]) -> Vec<Point2> {
    let mut pts = Vec::new();
    let na = a.len();
    let nb = b.len();
    for i in 0..na {
        let a1 = a[i];
        let a2 = a[(i + 1) % na];
        for j in 0..nb {
            let b1 = b[j];
            let b2 = b[(j + 1) % nb];
            if let Some(p) = intersection_point(a1, a2, b1, b2) {
                pts.push(p);
            }
        }
    }
    pts
}

/// A directed edge of an input polygon, stored as endpoint coordinates so the
/// split logic can sort on-segment vertices along it.
struct InputEdge {
    a: Point2,
    b: Point2,
}

/// Split every edge of `a` and `b` into sub-edges at every merged vertex lying
/// on it. Returns the deduplicated set of directed sub-edges (each undirected
/// edge appears once, oriented `from → to`).
fn split_edges(a: &[Point2], b: &[Point2], verts: &VertexMerger) -> Vec<SubEdge> {
    let na = a.len();
    let nb = b.len();
    let mut input_edges: Vec<InputEdge> = Vec::with_capacity(na + nb);
    for i in 0..na {
        input_edges.push(InputEdge {
            a: a[i],
            b: a[(i + 1) % na],
        });
    }
    for i in 0..nb {
        input_edges.push(InputEdge {
            a: b[i],
            b: b[(i + 1) % nb],
        });
    }

    let mut seen: Vec<SubEdge> = Vec::new();
    let mut out: Vec<SubEdge> = Vec::new();

    for e in &input_edges {
        // Find every merged vertex on this edge.
        let mut on: Vec<(u32, Point2)> = Vec::new();
        for (idx, v) in verts.points.iter().enumerate() {
            if on_segment(*v, e.a, e.b) {
                on.push((idx as u32, *v));
            }
        }
        if on.len() < 2 {
            continue;
        }
        // Sort along the edge by projection onto (b - a).
        let dx = e.b.x - e.a.x;
        let dy = e.b.y - e.a.y;
        on.sort_by(|p, q| {
            let tp = (p.1.x - e.a.x) * dx + (p.1.y - e.a.y) * dy;
            let tq = (q.1.x - e.a.x) * dx + (q.1.y - e.a.y) * dy;
            tp.partial_cmp(&tq).unwrap_or(std::cmp::Ordering::Equal)
        });
        // Emit sub-edges between consecutive distinct vertices.
        for w in on.windows(2) {
            let from = w[0].0;
            let to = w[1].0;
            if from == to {
                continue;
            }
            // Skip zero-length sub-edges (vertices coincide within tolerance).
            if approx_eq(verts.points[from as usize], verts.points[to as usize]) {
                continue;
            }
            let se = SubEdge { from, to };
            // Deduplicate (an edge may be re-discovered from both polygons).
            if !seen.contains(&se) {
                seen.push(se);
                out.push(se);
            }
        }
    }

    out
}

// ───────────────────────────────────────────────────────────────────────────
//  Top-level DCEL construction (steps 2–5)
// ───────────────────────────────────────────────────────────────────────────

/// Build the full DCEL for the overlay of polygons `a` and `b`, with every
/// face labelled by `(in_a, in_b)`.
fn build_overlay_dcel(a: &[Point2], b: &[Point2]) -> Result<Dcel, DcelError> {
    if a.len() < 3 || b.len() < 3 {
        return Err(DcelError::TooFewVertices {
            got: a.len().min(b.len()),
        });
    }

    // Step 1 — merge vertices + intersection points.
    let mut merger = VertexMerger::new();
    for &p in a {
        merger.insert(p);
    }
    for &p in b {
        merger.insert(p);
    }
    for p in collect_intersections(a, b) {
        merger.insert(p);
    }

    let vertex_count = merger.len();
    let points = merger.points.clone();

    // Step 2 — split edges into sub-edges.
    let sub_edges = split_edges(a, b, &merger);
    if sub_edges.is_empty() {
        return Err(DcelError::DegenerateInput);
    }
    let m = sub_edges.len();

    // Step 3 — half-edge table with twin linkage.
    let mut he = vec![DcelHalfEdge::default(); 2 * m];
    for (i, se) in sub_edges.iter().enumerate() {
        let f = i as u32;
        let t = (i + m) as u32;
        he[f as usize].origin = se.from;
        he[f as usize].twin = t;
        he[t as usize].origin = se.to;
        he[t as usize].twin = f;
    }

    // Group outgoing half-edges per vertex and sort CCW by direction angle.
    let mut outgoing: Vec<Vec<u32>> = vec![Vec::new(); vertex_count];
    for (i, h) in he.iter().enumerate() {
        outgoing[h.origin as usize].push(i as u32);
    }
    for v in 0..vertex_count {
        let o = points[v];
        outgoing[v].sort_by(|&h1, &h2| {
            let d1 = points[he[he[h1 as usize].twin as usize].origin as usize];
            let d2 = points[he[he[h2 as usize].twin as usize].origin as usize];
            let a1 = (d1.y - o.y).atan2(d1.x - o.x);
            let a2 = (d2.y - o.y).atan2(d2.x - o.x);
            a1.partial_cmp(&a2).unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    // Linkage rule: at vertex v with CCW-sorted outgoing [g_0..g_{m-1}],
    // twin(g_i).next = g_{(i-1) mod m} (the CCW-predecessor). This keeps the
    // face on the left of each half-edge: arriving at v via twin(g_i), the
    // next edge of the face is the most clockwise outgoing edge still CCW of
    // the incoming direction, which is the predecessor of twin(g_i) in the
    // CCW order. (Using the successor instead inverts face orientation.)
    for v in 0..vertex_count {
        let gs = &outgoing[v];
        let k = gs.len();
        if k == 0 {
            continue;
        }
        for i in 0..k {
            let g_i = gs[i];
            let twin = he[g_i as usize].twin;
            let next = gs[(i + k - 1) % k];
            he[twin as usize].next = next;
            he[next as usize].prev = twin;
        }
    }

    // Step 4 — walk faces.
    let n_he = he.len();
    let mut face_id: u32 = 0;
    let mut face_outer: Vec<u32> = Vec::new(); // representative half-edge of each cycle
    let mut face_cycles: Vec<Vec<u32>> = Vec::new(); // half-edges per cycle
    for start in 0..n_he {
        if he[start].face != u32::MAX {
            continue;
        }
        // Walk the cycle.
        let mut cycle: Vec<u32> = Vec::new();
        let mut cur = start as u32;
        loop {
            if he[cur as usize].face != u32::MAX {
                // Malformed cycle (should not happen for a planar subdivision);
                // bail out of this cycle.
                break;
            }
            he[cur as usize].face = face_id;
            cycle.push(cur);
            cur = he[cur as usize].next;
            if cur as usize == start {
                break;
            }
            if cycle.len() > n_he {
                break;
            }
        }
        face_outer.push(start as u32);
        face_cycles.push(cycle);
        face_id += 1;
    }

    // Compute each cycle's signed area and vertex list.
    let mut cycle_pts: Vec<Vec<Point2>> = Vec::with_capacity(face_cycles.len());
    let mut cycle_area: Vec<f64> = Vec::with_capacity(face_cycles.len());
    for cyc in &face_cycles {
        let pts: Vec<Point2> = cyc
            .iter()
            .map(|&h| points[he[h as usize].origin as usize])
            .collect();
        let area = cycle_signed_area(&pts);
        cycle_pts.push(pts);
        cycle_area.push(area);
    }

    // Step 5 — classify cycles and group holes into faces.
    // CCW (area > 0) cycles are outer boundaries of bounded faces.
    // CW (area < 0) cycles are holes; each nests inside the smallest CCW cycle
    // containing it. CW cycles in no CCW cycle belong to the unbounded face.
    let n_cycles = face_cycles.len();

    // Build face records: one per cycle initially; CCW cycles become bounded
    // faces, CW cycles get attached as holes.
    let mut faces: Vec<DcelFace> = Vec::with_capacity(n_cycles);
    let mut cycle_to_face: Vec<u32> = vec![u32::MAX; n_cycles];
    let mut unbounded_face: Option<u32> = None;

    // First pass: CCW cycles are bounded faces.
    for c in 0..n_cycles {
        if cycle_area[c] > 0.0 {
            let fid = faces.len() as u32;
            cycle_to_face[c] = fid;
            faces.push(DcelFace {
                outer: face_outer[c],
                holes: Vec::new(),
                label: FaceLabel {
                    in_a: false,
                    in_b: false,
                },
                bounded: true,
            });
        }
    }
    // Second pass: CW cycles. Find the containing CCW cycle (smallest area).
    for c in 0..n_cycles {
        if cycle_area[c] >= 0.0 {
            continue;
        }
        // Representative point of this CW cycle: any vertex.
        let rep = cycle_pts[c][0];
        let mut best: Option<usize> = None; // cycle index of containing CCW cycle
        let mut best_area = f64::INFINITY;
        for cc in 0..n_cycles {
            if cycle_area[cc] <= 0.0 {
                continue;
            }
            if point_in_polygon(rep, &cycle_pts[cc]) {
                if cycle_area[cc] < best_area {
                    best_area = cycle_area[cc];
                    best = Some(cc);
                }
            }
        }
        match best {
            Some(cc) => {
                let fid = cycle_to_face[cc];
                // Re-label this cycle's half-edges to the parent face.
                for &h in &face_cycles[c] {
                    he[h as usize].face = fid;
                }
                faces[fid as usize].holes.push(face_outer[c]);
            }
            None => {
                // Belongs to the unbounded face.
                let fid = if let Some(id) = unbounded_face {
                    id
                } else {
                    let id = faces.len() as u32;
                    faces.push(DcelFace {
                        outer: u32::MAX,
                        holes: Vec::new(),
                        label: FaceLabel {
                            in_a: false,
                            in_b: false,
                        },
                        bounded: false,
                    });
                    unbounded_face = Some(id);
                    id
                };
                for &h in &face_cycles[c] {
                    he[h as usize].face = fid;
                }
                faces[fid as usize].holes.push(face_outer[c]);
            }
        }
    }
    // If there were no CW cycles at all, still ensure an unbounded face exists
    // so the Euler identity counts it.
    if unbounded_face.is_none() {
        faces.push(DcelFace {
            outer: u32::MAX,
            holes: Vec::new(),
            label: FaceLabel {
                in_a: false,
                in_b: false,
            },
            bounded: false,
        });
    }

    // Step 6 — label bounded faces by representative point.
    for f in 0..faces.len() {
        if !faces[f].bounded {
            continue;
        }
        let rep = representative_point(faces[f].outer, &he, &points);
        let (in_a, in_b) = match rep {
            Some(p) => (point_in_polygon(p, a), point_in_polygon(p, b)),
            None => (false, false),
        };
        faces[f].label = FaceLabel { in_a, in_b };
    }

    // Build vertex incident pointers (any outgoing half-edge).
    let mut vertices: Vec<DcelVertex> = points
        .iter()
        .map(|&p| DcelVertex {
            point: p,
            incident: u32::MAX,
        })
        .collect();
    for (i, h) in he.iter().enumerate() {
        let o = h.origin as usize;
        if o < vertices.len() && vertices[o].incident == u32::MAX {
            vertices[o].incident = i as u32;
        }
    }

    Ok(Dcel {
        vertices,
        half_edges: he,
        faces,
    })
}

/// Compute a point strictly inside the face whose outer boundary starts at
/// `outer_he`. Uses the bottom-most outer vertex nudged along the inward
/// bisector of its two incident boundary edges — robust for non-convex faces.
fn representative_point(outer_he: u32, he: &[DcelHalfEdge], points: &[Point2]) -> Option<Point2> {
    if outer_he == u32::MAX {
        return None;
    }
    // Collect the outer cycle vertices in order.
    let mut cycle: Vec<u32> = Vec::new();
    let mut cur = outer_he;
    loop {
        cycle.push(he[cur as usize].origin);
        cur = he[cur as usize].next;
        if cur == outer_he {
            break;
        }
        if cycle.len() > he.len() {
            return None;
        }
    }
    if cycle.is_empty() {
        return None;
    }
    // Bottom-most vertex (min y, then min x).
    let mut vidx = 0usize;
    for i in 1..cycle.len() {
        let pi = points[cycle[i] as usize];
        let pv = points[cycle[vidx] as usize];
        if pi.y < pv.y || (pi.y == pv.y && pi.x < pv.x) {
            vidx = i;
        }
    }
    let n = cycle.len();
    let v = points[cycle[vidx] as usize];
    let prev = points[cycle[(vidx + n - 1) % n] as usize];
    let next = points[cycle[(vidx + 1) % n] as usize];
    // Inward bisector: average of unit vectors from v to prev and v to next.
    let to_prev = Point2::new(prev.x - v.x, prev.y - v.y);
    let to_next = Point2::new(next.x - v.x, next.y - v.y);
    let lp = to_prev.x.hypot(to_prev.y).max(f64::MIN_POSITIVE);
    let ln = to_next.x.hypot(to_next.y).max(f64::MIN_POSITIVE);
    let bx = to_prev.x / lp + to_next.x / ln;
    let by = to_prev.y / lp + to_next.y / ln;
    let blen = bx.hypot(by).max(f64::MIN_POSITIVE);
    // Nudge proportional to the smaller incident edge length.
    let nudge = 1e-6 * lp.min(ln);
    Some(Point2::new(
        v.x + nudge * bx / blen,
        v.y + nudge * by / blen,
    ))
}

// ───────────────────────────────────────────────────────────────────────────
//  Step 7 — boolean extraction
// ───────────────────────────────────────────────────────────────────────────

/// Is `face_id` selected by `op`? The unbounded face (which has label
/// `(false, false)`) is never selected by any of the four boolean ops.
#[inline]
fn face_selected(dcel: &Dcel, op: BooleanOp, face_id: u32) -> bool {
    if (face_id as usize) >= dcel.faces.len() {
        return false;
    }
    dcel.faces[face_id as usize].label.selected(op)
}

/// Find the next boundary half-edge after `h` in a result cycle. `h` must be a
/// boundary edge (selected on its left, unselected on its right). The walk
/// advances through `next` within the selected face; when it hits an internal
/// edge (both sides selected) it crosses into the adjacent selected face via
/// the twin and continues, until it reaches the next boundary edge.
fn next_boundary(dcel: &Dcel, op: BooleanOp, h: u32) -> u32 {
    let he = &dcel.half_edges;
    let mut cur = he[h as usize].next;
    // Guard against infinite loops in malformed subdivisions.
    for _ in 0..he.len() {
        let twin = he[cur as usize].twin;
        // `cur` is internal iff the face on the other side is also selected.
        if !face_selected(dcel, op, he[twin as usize].face) {
            return cur; // boundary edge — selected left, unselected right.
        }
        // Cross into the adjacent selected face and continue.
        cur = he[twin as usize].next;
    }
    cur
}

/// Extract the boolean result components for `op` from a labelled overlay DCEL.
///
/// A result boundary half-edge is one whose left face is selected and whose
/// right face (the face of its twin) is not. Walking `next_boundary` from each
/// such edge traces a result cycle; CCW cycles are outer boundaries, CW cycles
/// are holes. Holes are nested into the smallest containing outer cycle.
fn extract_boolean(dcel: &Dcel, op: BooleanOp) -> Vec<PolygonWithHoles> {
    let he = &dcel.half_edges;
    let points: Vec<Point2> = dcel.vertices.iter().map(|v| v.point).collect();
    let n = he.len();

    // Collect boundary half-edges: left selected, right not selected.
    let mut is_boundary = vec![false; n];
    for h in 0..n {
        let f_left = he[h].face;
        let f_right = he[he[h].twin as usize].face;
        if face_selected(dcel, op, f_left) && !face_selected(dcel, op, f_right) {
            is_boundary[h] = true;
        }
    }

    // Walk result cycles.
    let mut cycles: Vec<Vec<Point2>> = Vec::new();
    let mut visited = vec![false; n];
    for start in 0..n {
        if !is_boundary[start] || visited[start] {
            continue;
        }
        let mut pts: Vec<Point2> = Vec::new();
        let mut cur = start as u32;
        loop {
            if visited[cur as usize] {
                break;
            }
            visited[cur as usize] = true;
            pts.push(points[he[cur as usize].origin as usize]);
            cur = next_boundary(dcel, op, cur);
            if cur as usize == start {
                break;
            }
            if pts.len() > n {
                break;
            }
        }
        if pts.len() >= 3 {
            cycles.push(pts);
        }
    }

    // Separate outer (CCW, positive area) from holes (CW, negative area).
    let mut outers: Vec<(usize, Vec<Point2>)> = Vec::new(); // (index, cycle)
    let mut holes: Vec<Vec<Point2>> = Vec::new();
    for c in cycles {
        if polygon_signed_area(&c) > 0.0 {
            outers.push((outers.len(), c));
        } else {
            holes.push(c);
        }
    }

    // Nest each hole inside the smallest outer containing it.
    let mut hole_of: Vec<Vec<Vec<Point2>>> = vec![Vec::new(); outers.len()];
    for h in &holes {
        let rep = h[0];
        let mut best: Option<usize> = None;
        let mut best_area = f64::INFINITY;
        for (oi, (_, oc)) in outers.iter().enumerate() {
            if point_in_polygon(rep, oc) {
                let a = polygon_signed_area(oc).abs();
                if a < best_area {
                    best_area = a;
                    best = Some(oi);
                }
            }
        }
        if let Some(oi) = best {
            hole_of[oi].push(h.clone());
        }
    }

    outers
        .into_iter()
        .enumerate()
        .map(|(oi, (_, outer))| PolygonWithHoles {
            outer,
            holes: std::mem::take(&mut hole_of[oi]),
        })
        .collect()
}

// ───────────────────────────────────────────────────────────────────────────
//  Identities
// ───────────────────────────────────────────────────────────────────────────

/// Euler characteristic V − E + F of the subdivision, counting the unbounded
/// face. For a connected planar subdivision this is 2; for C components it is
/// 1 + C.
pub fn euler_characteristic(dcel: &Dcel) -> i64 {
    let v = dcel.vertices.len() as i64;
    let e = (dcel.half_edges.len() / 2) as i64;
    let f = dcel.faces.len() as i64;
    v - e + f
}

/// Total unsigned area of a set of polygons-with-holes (holes subtracted).
pub fn total_area(components: &[PolygonWithHoles]) -> f64 {
    let mut sum = 0.0;
    for c in components {
        sum += polygon_signed_area(&c.outer).abs();
        for h in &c.holes {
            sum -= polygon_signed_area(h).abs();
        }
    }
    sum
}

// ───────────────────────────────────────────────────────────────────────────
//  Public entry point
// ───────────────────────────────────────────────────────────────────────────

/// Compute the overlay of two simple polygons and extract the boolean result
/// for `op`. The returned [`OverlayResult`] carries the labelled DCEL, the
/// extracted polygon-with-holes components, and the Euler characteristic.
pub fn overlay_boolean(
    a: &[Point2],
    b: &[Point2],
    op: BooleanOp,
) -> Result<OverlayResult, DcelError> {
    let dcel = build_overlay_dcel(a, b)?;
    let components = extract_boolean(&dcel, op);
    let euler = euler_characteristic(&dcel);
    Ok(OverlayResult {
        dcel,
        components,
        euler,
    })
}

// ───────────────────────────────────────────────────────────────────────────
//  Tests
// ───────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// A CCW unit square.
    fn unit_square() -> Vec<Point2> {
        vec![
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(1.0, 1.0),
            Point2::new(0.0, 1.0),
        ]
    }

    /// A CCW square shifted by (s, s).
    fn shifted_square(s: f64) -> Vec<Point2> {
        vec![
            Point2::new(s, s),
            Point2::new(s + 1.0, s),
            Point2::new(s + 1.0, s + 1.0),
            Point2::new(s, s + 1.0),
        ]
    }

    #[test]
    fn rejects_too_few_vertices() {
        let a = vec![Point2::new(0.0, 0.0), Point2::new(1.0, 0.0)];
        let b = unit_square();
        assert_eq!(
            overlay_boolean(&a, &b, BooleanOp::Union),
            Err(DcelError::TooFewVertices { got: 2 })
        );
    }

    #[test]
    fn disjoint_union_has_two_components() {
        // Two unit squares far apart → union is two separate components.
        let a = unit_square();
        let b = shifted_square(5.0);
        let res = overlay_boolean(&a, &b, BooleanOp::Union).unwrap();
        assert_eq!(
            res.components.len(),
            2,
            "disjoint union should be 2 components"
        );
        // Area identity: area(union) = area(A) + area(B) = 2.
        let area = total_area(&res.components);
        assert!((area - 2.0).abs() < 1e-9, "union area {area} should be 2.0");
    }

    #[test]
    fn disjoint_intersection_is_empty() {
        let a = unit_square();
        let b = shifted_square(5.0);
        let res = overlay_boolean(&a, &b, BooleanOp::Intersection).unwrap();
        assert!(
            res.components.is_empty(),
            "disjoint intersection should be empty"
        );
    }

    #[test]
    fn identical_squares_union_is_one_square() {
        let a = unit_square();
        let b = unit_square();
        let res = overlay_boolean(&a, &b, BooleanOp::Union).unwrap();
        assert_eq!(
            res.components.len(),
            1,
            "identical union should be 1 component"
        );
        let area = total_area(&res.components);
        assert!(
            (area - 1.0).abs() < 1e-9,
            "identical union area {area} should be 1.0"
        );
    }

    #[test]
    fn identical_squares_intersection_is_one_square() {
        let a = unit_square();
        let b = unit_square();
        let res = overlay_boolean(&a, &b, BooleanOp::Intersection).unwrap();
        assert_eq!(res.components.len(), 1);
        let area = total_area(&res.components);
        assert!(
            (area - 1.0).abs() < 1e-9,
            "identical intersection area {area} should be 1.0"
        );
    }

    #[test]
    fn half_overlap_union_area_identity() {
        // A = [0,1]², B = [0.5,1.5]². Overlap = [0.5,1]² area 0.25.
        // union area = 1 + 1 - 0.25 = 1.75.
        let a = unit_square();
        let b = shifted_square(0.5);
        let res = overlay_boolean(&a, &b, BooleanOp::Union).unwrap();
        let area = total_area(&res.components);
        assert!(
            (area - 1.75).abs() < 1e-9,
            "half-overlap union area {area} should be 1.75"
        );
    }

    #[test]
    fn half_overlap_intersection_area_identity() {
        let a = unit_square();
        let b = shifted_square(0.5);
        let res = overlay_boolean(&a, &b, BooleanOp::Intersection).unwrap();
        assert_eq!(
            res.components.len(),
            1,
            "half-overlap intersection is one piece"
        );
        let area = total_area(&res.components);
        assert!(
            (area - 0.25).abs() < 1e-9,
            "half-overlap intersection area {area} should be 0.25"
        );
    }

    #[test]
    fn half_overlap_difference_area_identity() {
        // A \ B = [0,1]² minus [0.5,1.5]² = L-shape area 0.75.
        let a = unit_square();
        let b = shifted_square(0.5);
        let res = overlay_boolean(&a, &b, BooleanOp::Difference).unwrap();
        let area = total_area(&res.components);
        assert!(
            (area - 0.75).abs() < 1e-9,
            "difference area {area} should be 0.75"
        );
    }

    #[test]
    fn half_overlap_xor_area_identity() {
        // A xor B = union minus intersection = 1.75 - 0.25 = 1.5.
        let a = unit_square();
        let b = shifted_square(0.5);
        let res = overlay_boolean(&a, &b, BooleanOp::Xor).unwrap();
        let area = total_area(&res.components);
        assert!((area - 1.5).abs() < 1e-9, "xor area {area} should be 1.5");
    }

    #[test]
    fn euler_identity_half_overlap() {
        // The overlay of two overlapping squares is a connected planar
        // subdivision → V − E + F = 2 (one component).
        let a = unit_square();
        let b = shifted_square(0.5);
        let res = overlay_boolean(&a, &b, BooleanOp::Union).unwrap();
        assert_eq!(
            res.euler, 2,
            "Euler characteristic of a connected overlay subdivision should be 2"
        );
    }

    #[test]
    fn euler_identity_disjoint() {
        // Two disjoint squares → 2 components → V − E + F = 1 + C = 3.
        let a = unit_square();
        let b = shifted_square(5.0);
        let res = overlay_boolean(&a, &b, BooleanOp::Union).unwrap();
        assert_eq!(
            res.euler, 3,
            "Euler characteristic of a 2-component subdivision should be 3"
        );
    }

    #[test]
    fn contained_square_produces_hole_in_difference() {
        // A = big square [0,3]², B = small square [1,2]² inside it.
        // A \ B should be one component with a single hole.
        let a = vec![
            Point2::new(0.0, 0.0),
            Point2::new(3.0, 0.0),
            Point2::new(3.0, 3.0),
            Point2::new(0.0, 3.0),
        ];
        let b = vec![
            Point2::new(1.0, 1.0),
            Point2::new(2.0, 1.0),
            Point2::new(2.0, 2.0),
            Point2::new(1.0, 2.0),
        ];
        let res = overlay_boolean(&a, &b, BooleanOp::Difference).unwrap();
        assert_eq!(
            res.components.len(),
            1,
            "difference should be one component"
        );
        assert_eq!(
            res.components[0].holes.len(),
            1,
            "difference should have one hole"
        );
        let area = total_area(&res.components);
        // 3×3 square minus 1×1 hole = 9 - 1 = 8.
        assert!(
            (area - 8.0).abs() < 1e-9,
            "difference-with-hole area {area} should be 8.0"
        );
    }

    #[test]
    fn union_of_nested_squares_is_outer_only() {
        // A = big [0,3]², B = small [1,2]² inside. Union = big square (no hole).
        let a = vec![
            Point2::new(0.0, 0.0),
            Point2::new(3.0, 0.0),
            Point2::new(3.0, 3.0),
            Point2::new(0.0, 3.0),
        ];
        let b = vec![
            Point2::new(1.0, 1.0),
            Point2::new(2.0, 1.0),
            Point2::new(2.0, 2.0),
            Point2::new(1.0, 2.0),
        ];
        let res = overlay_boolean(&a, &b, BooleanOp::Union).unwrap();
        assert_eq!(res.components.len(), 1);
        assert!(
            res.components[0].holes.is_empty(),
            "union of nested squares has no hole"
        );
        let area = total_area(&res.components);
        assert!(
            (area - 9.0).abs() < 1e-9,
            "nested union area {area} should be 9.0"
        );
    }

    #[test]
    fn intersection_of_nested_squares_is_inner() {
        let a = vec![
            Point2::new(0.0, 0.0),
            Point2::new(3.0, 0.0),
            Point2::new(3.0, 3.0),
            Point2::new(0.0, 3.0),
        ];
        let b = vec![
            Point2::new(1.0, 1.0),
            Point2::new(2.0, 1.0),
            Point2::new(2.0, 2.0),
            Point2::new(1.0, 2.0),
        ];
        let res = overlay_boolean(&a, &b, BooleanOp::Intersection).unwrap();
        assert_eq!(res.components.len(), 1);
        let area = total_area(&res.components);
        assert!(
            (area - 1.0).abs() < 1e-9,
            "nested intersection area {area} should be 1.0"
        );
    }

    #[test]
    fn face_labels_classify_correctly() {
        // Half-overlap: the DCEL should have a bounded face labelled (true,true)
        // for the intersection region.
        let a = unit_square();
        let b = shifted_square(0.5);
        let res = overlay_boolean(&a, &b, BooleanOp::Union).unwrap();
        let labels: Vec<FaceLabel> = res
            .dcel
            .faces
            .iter()
            .filter(|f| f.bounded)
            .map(|f| f.label)
            .collect();
        // Expect at least one (in_a, in_b) = (true,true) face.
        assert!(
            labels.iter().any(|l| l.in_a && l.in_b),
            "expected an intersection face labelled (true,true), got {labels:?}"
        );
        // Expect at least one (true,false) and one (false,true) face.
        assert!(
            labels.iter().any(|l| l.in_a && !l.in_b),
            "expected an A-only face labelled (true,false), got {labels:?}"
        );
        assert!(
            labels.iter().any(|l| !l.in_a && l.in_b),
            "expected a B-only face labelled (false,true), got {labels:?}"
        );
    }

    #[test]
    fn cross_overlap_union_is_single_component() {
        // A vertical rectangle and a horizontal rectangle crossing in the
        // middle → union is one connected plus-shaped component.
        let a = vec![
            Point2::new(0.4, 0.0),
            Point2::new(0.6, 0.0),
            Point2::new(0.6, 1.0),
            Point2::new(0.4, 1.0),
        ];
        let b = vec![
            Point2::new(0.0, 0.4),
            Point2::new(1.0, 0.4),
            Point2::new(1.0, 0.6),
            Point2::new(0.0, 0.6),
        ];
        let res = overlay_boolean(&a, &b, BooleanOp::Union).unwrap();
        assert_eq!(
            res.components.len(),
            1,
            "cross union should be one component"
        );
        // Area = 0.2*1 + 1*0.2 - 0.2*0.2 = 0.2 + 0.2 - 0.04 = 0.36.
        let area = total_area(&res.components);
        assert!(
            (area - 0.36).abs() < 1e-9,
            "cross union area {area} should be 0.36"
        );
        assert_eq!(res.euler, 2, "cross union subdivision is connected");
    }

    #[test]
    fn cross_overlap_intersection_is_central_square() {
        let a = vec![
            Point2::new(0.4, 0.0),
            Point2::new(0.6, 0.0),
            Point2::new(0.6, 1.0),
            Point2::new(0.4, 1.0),
        ];
        let b = vec![
            Point2::new(0.0, 0.4),
            Point2::new(1.0, 0.4),
            Point2::new(1.0, 0.6),
            Point2::new(0.0, 0.6),
        ];
        let res = overlay_boolean(&a, &b, BooleanOp::Intersection).unwrap();
        assert_eq!(res.components.len(), 1);
        let area = total_area(&res.components);
        assert!(
            (area - 0.04).abs() < 1e-9,
            "cross intersection area {area} should be 0.04"
        );
    }

    #[test]
    fn outer_boundary_is_ccw_and_holes_cw() {
        // The extracted outer boundaries must be CCW (positive signed area) and
        // holes CW (negative signed area), per the DCEL convention.
        let a = vec![
            Point2::new(0.0, 0.0),
            Point2::new(3.0, 0.0),
            Point2::new(3.0, 3.0),
            Point2::new(0.0, 3.0),
        ];
        let b = vec![
            Point2::new(1.0, 1.0),
            Point2::new(2.0, 1.0),
            Point2::new(2.0, 2.0),
            Point2::new(1.0, 2.0),
        ];
        let res = overlay_boolean(&a, &b, BooleanOp::Difference).unwrap();
        let c = &res.components[0];
        assert!(
            polygon_signed_area(&c.outer) > 0.0,
            "outer boundary must be CCW (positive signed area)"
        );
        for h in &c.holes {
            assert!(
                polygon_signed_area(h) < 0.0,
                "hole must be CW (negative signed area)"
            );
        }
    }

    #[test]
    fn triangle_square_overlay_area_identity() {
        // A triangle overlapping a square.
        let a = vec![
            Point2::new(0.0, 0.0),
            Point2::new(2.0, 0.0),
            Point2::new(0.0, 2.0),
        ];
        let b = vec![
            Point2::new(0.5, 0.5),
            Point2::new(1.5, 0.5),
            Point2::new(1.5, 1.5),
            Point2::new(0.5, 1.5),
        ];
        let area_a = polygon_signed_area(&a).abs();
        let area_b = polygon_signed_area(&b).abs();
        let inter = overlay_boolean(&a, &b, BooleanOp::Intersection).unwrap();
        let area_inter = total_area(&inter.components);
        let uni = overlay_boolean(&a, &b, BooleanOp::Union).unwrap();
        let area_union = total_area(&uni.components);
        // area(A∪B) = area(A) + area(B) − area(A∩B).
        assert!(
            (area_union - (area_a + area_b - area_inter)).abs() < 1e-9,
            "union area identity: {area_union} vs {}",
            area_a + area_b - area_inter
        );
        // area(A\B) = area(A) − area(A∩B).
        let diff = overlay_boolean(&a, &b, BooleanOp::Difference).unwrap();
        let area_diff = total_area(&diff.components);
        assert!(
            (area_diff - (area_a - area_inter)).abs() < 1e-9,
            "difference area identity: {area_diff} vs {}",
            area_a - area_inter
        );
    }

    #[test]
    fn determinism_same_input_same_output() {
        let a = unit_square();
        let b = shifted_square(0.5);
        let r1 = overlay_boolean(&a, &b, BooleanOp::Union).unwrap();
        let r2 = overlay_boolean(&a, &b, BooleanOp::Union).unwrap();
        assert_eq!(
            r1.components, r2.components,
            "overlay must be deterministic"
        );
        assert_eq!(r1.euler, r2.euler);
    }
}
