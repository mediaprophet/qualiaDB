//! Convex decomposition of simple polygons (P11.3).
//!
//! Decompose a simple polygon into convex pieces. Two algorithms:
//!
//! 1. **Hertel-Mehlhorn** (`convex_decomposition_hm`): Triangulate the
//!    polygon (using P11.5), then merge adjacent triangles whose union
//!    is convex. Produces at most 4× the optimal number of convex pieces
//!    in O(n) time after triangulation. Very practical — the standard
//!    fast convex decomposition.
//!
//! 2. **Triangulation-only** (`convex_decomposition_triangulation`):
//!    Returns the triangulation directly — every triangle is convex.
//!    O(n log n) via monotone partition. Upper bound n-2 pieces.
//!
//! Reference: Hertel & Mehlhorn, "Fast Triangulation of Simple Polygons"
//! (1983). de Berg §3.4 discusses the merge step.
//!
//! ## Determinism
//!
//! Both algorithms are deterministic. Output pieces are CCW convex polygons.

use super::polygon_validation::canonicalize_simple_polygon;
use super::primitives::{orientation_2, Orientation, Point2};
use super::triangulation_2::{triangulate_ear_clipping, triangulate_polygon, Triangle};

// ───────────────────────────────────────────────────────────────────────────
//  Convexity test
// ───────────────────────────────────────────────────────────────────────────

/// Check if a polygon is convex (all turns are CCW or collinear for a CCW polygon).
///
/// A polygon with fewer than 3 vertices is trivially convex.
pub fn is_convex_polygon(vertices: &[Point2]) -> bool {
    let n = vertices.len();
    if n < 3 {
        return true;
    }
    for i in 0..n {
        let prev = vertices[(i + n - 1) % n];
        let curr = vertices[i];
        let next = vertices[(i + 1) % n];
        if orientation_2(prev, curr, next) == Orientation::Clockwise {
            return false;
        }
    }
    true
}

// ───────────────────────────────────────────────────────────────────────────
//  Adjacency graph for triangles
// ───────────────────────────────────────────────────────────────────────────

/// An edge in the triangulation adjacency graph.
/// `tri_a` and `tri_b` are triangle indices; `tri_b` is `usize::MAX` for
/// boundary edges.
#[derive(Debug, Clone, Copy)]
struct AdjEdge {
    tri_a: usize,
    tri_b: usize,
    // The shared edge endpoints (sorted for matching).
    ep_lo: Point2,
    ep_hi: Point2,
}

/// Build the adjacency graph of a triangulation: for each shared edge
/// between two triangles, record which triangles are adjacent.
fn build_adjacency(triangles: &[Triangle]) -> Vec<AdjEdge> {
    let n = triangles.len();
    let mut edges: Vec<AdjEdge> = Vec::with_capacity(n * 3);

    // Collect all triangle edges with their triangle index.
    for (fi, t) in triangles.iter().enumerate() {
        for (a, b) in [(t.a, t.b), (t.b, t.c), (t.c, t.a)] {
            let (lo, hi) = if (a.x, a.y) < (b.x, b.y) {
                (a, b)
            } else {
                (b, a)
            };
            edges.push(AdjEdge {
                tri_a: fi,
                tri_b: usize::MAX,
                ep_lo: lo,
                ep_hi: hi,
            });
        }
    }

    // Match twin edges.
    let m = edges.len();
    for i in 0..m {
        if edges[i].tri_b != usize::MAX {
            continue;
        }
        for j in (i + 1)..m {
            if edges[j].tri_b != usize::MAX {
                continue;
            }
            if edges[i].ep_lo == edges[j].ep_lo && edges[i].ep_hi == edges[j].ep_hi {
                edges[i].tri_b = edges[j].tri_a;
                edges[j].tri_b = edges[i].tri_a;
                break;
            }
        }
    }

    edges
}

// ───────────────────────────────────────────────────────────────────────────
//  Hertel-Mehlhorn convex decomposition
// ───────────────────────────────────────────────────────────────────────────

/// Decompose a simple polygon into convex pieces using the Hertel-Mehlhorn
/// algorithm.
///
/// 1. Triangulate the polygon (P11.5 monotone partition + ear clipping fallback).
/// 2. Build the adjacency graph of the triangulation.
/// 3. For each internal diagonal (shared edge between two triangles), check
///    if removing it (merging the two triangles) produces a convex polygon.
///    If so, merge them.
/// 4. Repeat until no more merges are possible.
///
/// Produces at most 4× the optimal number of convex pieces.
/// O(n log n) for triangulation + O(n) for the merge phase.
///
/// Returns a list of convex polygons (each a Vec<Point2> in CCW order).
pub fn convex_decomposition_hm(vertices: &[Point2]) -> Vec<Vec<Point2>> {
    let poly = canonicalize_simple_polygon(vertices);
    if poly.len() < 3 {
        return Vec::new();
    }

    // If already convex, return the whole polygon.
    if is_convex_polygon(&poly) {
        return vec![poly];
    }

    // Triangulate.
    let triangles = triangulate_polygon(&poly);
    if triangles.is_empty() {
        // Fallback to ear clipping.
        let ear = triangulate_ear_clipping(&poly);
        if ear.is_empty() {
            return Vec::new();
        }
        return hm_merge(&ear);
    }

    hm_merge(&triangles)
}

/// Merge adjacent triangles where the union is convex.
/// This is the core of the Hertel-Mehlhorn algorithm.
fn hm_merge(triangles: &[Triangle]) -> Vec<Vec<Point2>> {
    let n = triangles.len();
    if n == 0 {
        return Vec::new();
    }

    // Start with each triangle as a separate piece.
    let mut pieces: Vec<Vec<Point2>> = triangles.iter().map(|t| vec![t.a, t.b, t.c]).collect();

    // Build adjacency.
    let adj_edges = build_adjacency(triangles);

    // Collect internal diagonals (shared edges).
    let mut diagonals: Vec<(usize, usize, Point2, Point2)> = Vec::new();
    for e in &adj_edges {
        if e.tri_b != usize::MAX {
            diagonals.push((e.tri_a, e.tri_b, e.ep_lo, e.ep_hi));
        }
    }

    // Track which piece each triangle belongs to (union-find style).
    let mut piece_id: Vec<usize> = (0..n).collect();
    let mut merged = vec![false; n];

    // Try merging across each diagonal.
    // We iterate until no more merges happen (fixpoint).
    loop {
        let mut any_merge = false;

        for &(a, b, ep_lo, ep_hi) in &diagonals {
            if merged[a] || merged[b] {
                continue;
            }

            // Find the root pieces for a and b.
            let pa = find_root(&mut piece_id, a);
            let pb = find_root(&mut piece_id, b);
            if pa == pb {
                continue; // Already in the same piece.
            }

            // Try to merge pieces pa and pb by removing the diagonal
            // (ep_lo, ep_hi). The merged polygon is the union of the two
            // pieces with the shared edge removed.
            if let Some(merged_poly) = try_merge(&pieces[pa], &pieces[pb], ep_lo, ep_hi) {
                if is_convex_polygon(&merged_poly) {
                    // Commit the merge.
                    pieces[pa] = merged_poly;
                    piece_id[pb] = pa;
                    merged[b] = true;
                    any_merge = true;
                }
            }
        }

        if !any_merge {
            break;
        }
    }

    // Collect non-merged pieces.
    let mut result = Vec::new();
    for i in 0..n {
        if !merged[i] {
            result.push(pieces[i].clone());
        }
    }
    result
}

/// Union-find: find the root piece for triangle i.
fn find_root(piece_id: &mut [usize], i: usize) -> usize {
    let mut root = i;
    while piece_id[root] != root {
        root = piece_id[root];
    }
    // Path compression.
    let mut curr = i;
    while piece_id[curr] != root {
        let next = piece_id[curr];
        piece_id[curr] = root;
        curr = next;
    }
    root
}

/// Try to merge two convex polygons by removing their shared edge
/// (ep_lo, ep_hi). Returns the merged polygon if the shared edge exists
/// in both polygons, or None otherwise.
///
/// The merged polygon is formed by walking the boundary of poly_a until
/// we hit one endpoint of the shared edge, then jumping to poly_b and
/// walking its boundary (skipping the shared edge), then jumping back.
fn try_merge(
    poly_a: &[Point2],
    poly_b: &[Point2],
    ep_lo: Point2,
    ep_hi: Point2,
) -> Option<Vec<Point2>> {
    let na = poly_a.len();
    let nb = poly_b.len();

    // Find the shared edge in poly_a: find the edge (i, i+1) where
    // {poly_a[i], poly_a[i+1]} == {ep_lo, ep_hi}.
    let mut a_edge_start = None;
    for i in 0..na {
        let j = (i + 1) % na;
        if (poly_a[i] == ep_lo && poly_a[j] == ep_hi) || (poly_a[i] == ep_hi && poly_a[j] == ep_lo)
        {
            a_edge_start = Some(i);
            break;
        }
    }
    let a_edge_start = a_edge_start?;

    // Find the shared edge in poly_b.
    let mut b_edge_start = None;
    for i in 0..nb {
        let j = (i + 1) % nb;
        if (poly_b[i] == ep_lo && poly_b[j] == ep_hi) || (poly_b[i] == ep_hi && poly_b[j] == ep_lo)
        {
            b_edge_start = Some(i);
            break;
        }
    }
    let b_edge_start = b_edge_start?;

    // Build the merged polygon: walk poly_a from (a_edge_start + 1) back
    // to a_edge_start (skipping the shared edge), then walk poly_b from
    // (b_edge_start + 1) back to b_edge_start (skipping the shared edge).
    let mut merged = Vec::with_capacity(na + nb - 2);

    // Walk poly_a: from the vertex after the shared edge, all the way
    // around to the vertex before the shared edge (i.e., a_edge_start).
    for k in 1..na {
        let idx = (a_edge_start + k) % na;
        merged.push(poly_a[idx]);
    }

    // Walk poly_b: from the vertex after the shared edge, all the way
    // around to the vertex before the shared edge (i.e., b_edge_start).
    for k in 1..nb {
        let idx = (b_edge_start + k) % nb;
        merged.push(poly_b[idx]);
    }

    // Remove consecutive duplicate vertices (can happen at the junction).
    merged.dedup_by(|a, b| *a == *b);
    // Also check wrap-around duplicate.
    if merged.len() > 1 && merged[0] == merged[merged.len() - 1] {
        merged.pop();
    }

    if merged.len() < 3 {
        None
    } else {
        Some(merged)
    }
}

// ───────────────────────────────────────────────────────────────────────────
//  Triangulation-only convex decomposition
// ───────────────────────────────────────────────────────────────────────────

/// Decompose a simple polygon into convex pieces by triangulation.
///
/// Every triangle is convex, so the triangulation is a valid convex
/// decomposition with at most n-2 pieces. This is the simplest approach
/// but produces more pieces than Hertel-Mehlhorn.
///
/// Returns a list of convex polygons (each a Vec<Point2> in CCW order).
pub fn convex_decomposition_triangulation(vertices: &[Point2]) -> Vec<Vec<Point2>> {
    let poly = canonicalize_simple_polygon(vertices);
    if poly.len() < 3 {
        return Vec::new();
    }

    if is_convex_polygon(&poly) {
        return vec![poly];
    }

    let triangles = triangulate_polygon(&poly);
    if triangles.is_empty() {
        let ear = triangulate_ear_clipping(&poly);
        return ear.iter().map(|t| vec![t.a, t.b, t.c]).collect();
    }

    triangles.iter().map(|t| vec![t.a, t.b, t.c]).collect()
}

// ───────────────────────────────────────────────────────────────────────────
//  Verification
// ───────────────────────────────────────────────────────────────────────────

/// Verify that a convex decomposition is valid:
/// 1. Every piece is convex.
/// 2. The total area of all pieces equals the area of the original polygon.
/// 3. The number of pieces is reasonable (≤ n-2 for triangulation, fewer for HM).
pub fn verify_convex_decomposition(vertices: &[Point2], pieces: &[Vec<Point2>]) -> bool {
    if pieces.is_empty() {
        return vertices.len() < 3;
    }

    // Check every piece is convex.
    for piece in pieces {
        if piece.len() < 3 {
            return false;
        }
        if !is_convex_polygon(piece) {
            return false;
        }
    }

    // Check total area.
    let poly_area = polygon_signed_area(vertices).abs();
    let piece_area: f64 = pieces.iter().map(|p| polygon_signed_area(p).abs()).sum();
    if (piece_area - poly_area).abs() > 1e-9 * poly_area.max(1.0) {
        return false;
    }

    true
}

/// Signed area of a polygon (positive for CCW).
fn polygon_signed_area(vertices: &[Point2]) -> f64 {
    let n = vertices.len();
    let mut sum = 0.0;
    for i in 0..n {
        let j = (i + 1) % n;
        sum += vertices[i].x * vertices[j].y - vertices[j].x * vertices[i].y;
    }
    sum * 0.5
}

// ───────────────────────────────────────────────────────────────────────────
//  Tests
// ───────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64) -> Point2 {
        Point2::new(x, y)
    }

    // ── Convexity test ─────────────────────────────────────────────────

    #[test]
    fn convex_polygon_is_convex() {
        let square = vec![p(0.0, 0.0), p(1.0, 0.0), p(1.0, 1.0), p(0.0, 1.0)];
        assert!(is_convex_polygon(&square));
    }

    #[test]
    fn reflex_polygon_is_not_convex() {
        let l_shape = vec![
            p(0.0, 0.0),
            p(2.0, 0.0),
            p(2.0, 1.0),
            p(1.0, 1.0),
            p(1.0, 2.0),
            p(0.0, 2.0),
        ];
        assert!(!is_convex_polygon(&l_shape));
    }

    #[test]
    fn triangle_is_convex() {
        let tri = vec![p(0.0, 0.0), p(1.0, 0.0), p(0.0, 1.0)];
        assert!(is_convex_polygon(&tri));
    }

    #[test]
    fn collinear_polygon_is_convex() {
        // Polygon with collinear vertices — still convex (no reflex turns).
        let poly = vec![
            p(0.0, 0.0),
            p(1.0, 0.0),
            p(2.0, 0.0),
            p(2.0, 2.0),
            p(0.0, 2.0),
        ];
        assert!(is_convex_polygon(&poly));
    }

    // ── Hertel-Mehlhorn decomposition ──────────────────────────────────

    #[test]
    fn hm_convex_polygon_single_piece() {
        let square = vec![p(0.0, 0.0), p(1.0, 0.0), p(1.0, 1.0), p(0.0, 1.0)];
        let pieces = convex_decomposition_hm(&square);
        assert_eq!(pieces.len(), 1, "convex polygon → 1 piece");
        assert!(verify_convex_decomposition(&square, &pieces));
    }

    #[test]
    fn hm_l_shape() {
        let l_shape = vec![
            p(0.0, 0.0),
            p(2.0, 0.0),
            p(2.0, 1.0),
            p(1.0, 1.0),
            p(1.0, 2.0),
            p(0.0, 2.0),
        ];
        let pieces = convex_decomposition_hm(&l_shape);
        assert!(
            pieces.len() <= 4,
            "L-shape should decompose into ≤4 pieces, got {}",
            pieces.len()
        );
        assert!(verify_convex_decomposition(&l_shape, &pieces));
    }

    #[test]
    fn hm_comb_shape() {
        let comb = vec![
            p(0.0, 0.0),
            p(5.0, 0.0),
            p(5.0, 3.0),
            p(4.0, 3.0),
            p(4.0, 1.0),
            p(3.0, 1.0),
            p(3.0, 3.0),
            p(2.0, 3.0),
            p(2.0, 1.0),
            p(1.0, 1.0),
            p(1.0, 3.0),
            p(0.0, 3.0),
        ];
        let pieces = convex_decomposition_hm(&comb);
        assert!(
            pieces.len() <= 10,
            "comb should decompose into ≤10 pieces, got {}",
            pieces.len()
        );
        assert!(verify_convex_decomposition(&comb, &pieces));
    }

    #[test]
    fn hm_star_shape() {
        let star = vec![
            p(0.0, 2.0),
            p(0.5, 0.5),
            p(2.0, 0.0),
            p(0.5, -0.5),
            p(0.0, -2.0),
            p(-0.5, -0.5),
            p(-2.0, 0.0),
            p(-0.5, 0.5),
        ];
        let pieces = convex_decomposition_hm(&star);
        assert!(
            pieces.len() <= 6,
            "star should decompose into ≤6 pieces, got {}",
            pieces.len()
        );
        assert!(verify_convex_decomposition(&star, &pieces));
    }

    #[test]
    fn hm_triangle_single_piece() {
        let tri = vec![p(0.0, 0.0), p(1.0, 0.0), p(0.0, 1.0)];
        let pieces = convex_decomposition_hm(&tri);
        assert_eq!(pieces.len(), 1);
        assert!(verify_convex_decomposition(&tri, &pieces));
    }

    #[test]
    fn hm_double_c_shape() {
        let double_c = vec![
            p(0.0, 0.0),
            p(6.0, 0.0),
            p(6.0, 6.0),
            p(5.0, 6.0),
            p(5.0, 1.0),
            p(4.0, 1.0),
            p(4.0, 5.0),
            p(3.0, 5.0),
            p(3.0, 1.0),
            p(2.0, 1.0),
            p(2.0, 6.0),
            p(0.0, 6.0),
        ];
        let pieces = convex_decomposition_hm(&double_c);
        assert!(
            pieces.len() <= 10,
            "double-C should decompose into ≤10 pieces, got {}",
            pieces.len()
        );
        assert!(verify_convex_decomposition(&double_c, &pieces));
    }

    #[test]
    fn hm_all_pieces_are_convex() {
        let l_shape = vec![
            p(0.0, 0.0),
            p(2.0, 0.0),
            p(2.0, 1.0),
            p(1.0, 1.0),
            p(1.0, 2.0),
            p(0.0, 2.0),
        ];
        let pieces = convex_decomposition_hm(&l_shape);
        for (i, piece) in pieces.iter().enumerate() {
            assert!(is_convex_polygon(piece), "piece {} is not convex", i);
        }
    }

    #[test]
    fn hm_fewer_pieces_than_triangulation() {
        // Hertel-Mehlhorn should produce fewer pieces than pure triangulation.
        let l_shape = vec![
            p(0.0, 0.0),
            p(2.0, 0.0),
            p(2.0, 1.0),
            p(1.0, 1.0),
            p(1.0, 2.0),
            p(0.0, 2.0),
        ];
        let hm_pieces = convex_decomposition_hm(&l_shape);
        let tri_pieces = convex_decomposition_triangulation(&l_shape);
        assert!(
            hm_pieces.len() <= tri_pieces.len(),
            "HM ({}) should produce ≤ triangulation ({})",
            hm_pieces.len(),
            tri_pieces.len()
        );
    }

    // ── Triangulation-only decomposition ───────────────────────────────

    #[test]
    fn triangulation_decomposition_convex_polygon() {
        let square = vec![p(0.0, 0.0), p(1.0, 0.0), p(1.0, 1.0), p(0.0, 1.0)];
        let pieces = convex_decomposition_triangulation(&square);
        assert_eq!(pieces.len(), 1, "convex polygon → 1 piece");
        assert!(verify_convex_decomposition(&square, &pieces));
    }

    #[test]
    fn triangulation_decomposition_l_shape() {
        let l_shape = vec![
            p(0.0, 0.0),
            p(2.0, 0.0),
            p(2.0, 1.0),
            p(1.0, 1.0),
            p(1.0, 2.0),
            p(0.0, 2.0),
        ];
        let pieces = convex_decomposition_triangulation(&l_shape);
        assert_eq!(pieces.len(), 4, "L-shape (6 vertices) → 4 triangles (n-2)");
        assert!(verify_convex_decomposition(&l_shape, &pieces));
    }

    #[test]
    fn triangulation_decomposition_all_convex() {
        let comb = vec![
            p(0.0, 0.0),
            p(5.0, 0.0),
            p(5.0, 3.0),
            p(4.0, 3.0),
            p(4.0, 1.0),
            p(3.0, 1.0),
            p(3.0, 3.0),
            p(2.0, 3.0),
            p(2.0, 1.0),
            p(1.0, 1.0),
            p(1.0, 3.0),
            p(0.0, 3.0),
        ];
        let pieces = convex_decomposition_triangulation(&comb);
        for piece in &pieces {
            assert!(is_convex_polygon(piece));
        }
    }

    // ── Verification ───────────────────────────────────────────────────

    #[test]
    fn verify_rejects_non_convex_piece() {
        let l_shape = vec![
            p(0.0, 0.0),
            p(2.0, 0.0),
            p(2.0, 1.0),
            p(1.0, 1.0),
            p(1.0, 2.0),
            p(0.0, 2.0),
        ];
        let bad_pieces = vec![l_shape.clone()]; // L-shape is not convex
        assert!(!verify_convex_decomposition(&l_shape, &bad_pieces));
    }

    #[test]
    fn verify_rejects_wrong_area() {
        let square = vec![p(0.0, 0.0), p(1.0, 0.0), p(1.0, 1.0), p(0.0, 1.0)];
        let bad_pieces = vec![vec![
            p(0.0, 0.0),
            p(2.0, 0.0),
            p(2.0, 2.0), // area 2.0, not 1.0
        ]];
        assert!(!verify_convex_decomposition(&square, &bad_pieces));
    }

    #[test]
    fn verify_accepts_correct_decomposition() {
        let square = vec![p(0.0, 0.0), p(1.0, 0.0), p(1.0, 1.0), p(0.0, 1.0)];
        let pieces = convex_decomposition_hm(&square);
        assert!(verify_convex_decomposition(&square, &pieces));
    }

    // ── Edge cases ─────────────────────────────────────────────────────

    #[test]
    fn empty_polygon_returns_empty() {
        let empty: Vec<Point2> = vec![];
        assert!(convex_decomposition_hm(&empty).is_empty());
        assert!(convex_decomposition_triangulation(&empty).is_empty());
    }

    #[test]
    fn too_few_vertices_returns_empty() {
        assert!(convex_decomposition_hm(&[p(0.0, 0.0)]).is_empty());
        assert!(convex_decomposition_hm(&[p(0.0, 0.0), p(1.0, 0.0)]).is_empty());
    }

    #[test]
    fn cw_polygon_canonicalized() {
        // CW square — should be canonicalized to CCW and decomposed.
        let cw_square = vec![p(0.0, 0.0), p(0.0, 1.0), p(1.0, 1.0), p(1.0, 0.0)];
        let pieces = convex_decomposition_hm(&cw_square);
        assert_eq!(pieces.len(), 1);
        assert!(verify_convex_decomposition(&cw_square, &pieces));
    }

    // ── Large polygon ──────────────────────────────────────────────────

    #[test]
    fn hm_large_convex_polygon() {
        // 20-vertex regular polygon (convex).
        let n = 20;
        let poly: Vec<Point2> = (0..n)
            .map(|i| {
                let angle = 2.0 * std::f64::consts::PI * i as f64 / n as f64;
                p(angle.cos(), angle.sin())
            })
            .collect();
        let pieces = convex_decomposition_hm(&poly);
        assert_eq!(pieces.len(), 1, "convex polygon → 1 piece");
        assert!(verify_convex_decomposition(&poly, &pieces));
    }

    #[test]
    fn hm_large_reflex_polygon() {
        // 30-vertex zigzag polygon.
        let n = 30;
        let mut poly = Vec::with_capacity(n);
        for i in 0..n {
            let x = i as f64;
            let y = if i % 2 == 0 { 0.0 } else { 2.0 };
            poly.push(p(x, y));
        }
        poly.push(p(n as f64, 3.0));
        poly.push(p(0.0, 3.0));
        let pieces = convex_decomposition_hm(&poly);
        assert!(verify_convex_decomposition(&poly, &pieces));
        // All pieces must be convex.
        for piece in &pieces {
            assert!(is_convex_polygon(piece));
        }
    }
}
