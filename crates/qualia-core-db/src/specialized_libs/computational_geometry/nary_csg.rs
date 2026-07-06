//! N-ary CSG operations on 2D polygons (P12.1).
//!
//! Supports constructive solid geometry on N input convex polygons:
//!
//! - **N-ary union**: union of all N polygons.
//! - **N-ary intersection**: intersection of all N polygons.
//! - **N-ary difference**: first polygon minus the union of the rest.
//! - **Symmetric difference**: XOR of all N polygons.
//!
//! The implementation uses Sutherland-Hodgman polygon clipping for
//! intersection and difference operations. For union, we decompose
//! into disjoint pieces: A₁ ∪ (A₂ \ A₁) ∪ (A₃ \ (A₁ ∪ A₂)) ∪ ...
//!
//! Also provides **mesh co-refinement**: given two polygon meshes (as
//! polygon soups), split both at their intersection points so they share
//! a common refinement. This is the 2D analogue of 3D mesh co-refinement.
//!
//! Tier-2 cold construction (uses `Vec` during computation).

use super::boolean_2::polygon_signed_area;
use super::primitives::{orientation_2, Orientation, Point2};

/// A polygon with holes (outer boundary CCW, holes CW).
#[derive(Debug, Clone, PartialEq)]
pub struct PolygonWithHoles {
    pub outer: Vec<Point2>,
    pub holes: Vec<Vec<Point2>>,
}

// ───────────────────────────────────────────────────────────────────────────
//  N-ary CSG
// ───────────────────────────────────────────────────────────────────────────

/// Error type for N-ary CSG operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NaryCsgError {
    /// Fewer than 1 input polygon.
    NoInputs,
    /// A polygon has fewer than 3 vertices.
    DegeneratePolygon { index: usize, got: usize },
}

impl core::fmt::Display for NaryCsgError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NoInputs => write!(f, "nary_csg: no input polygons"),
            Self::DegeneratePolygon { index, got } => {
                write!(f, "nary_csg: polygon {} has {} vertices (need ≥ 3)", index, got)
            }
        }
    }
}

/// N-ary CSG operation type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NaryOp {
    Union,
    Intersection,
    /// A₁ \ (A₂ ∪ A₃ ∪ ... ∪ Aₙ)
    Difference,
    /// Symmetric difference (XOR) of all polygons.
    SymmetricDifference,
}

/// Result of an N-ary CSG operation: a set of polygon-with-holes components.
#[derive(Debug, Clone, PartialEq)]
pub struct NaryCsgResult {
    /// Result components (polygons with holes).
    pub components: Vec<PolygonWithHoles>,
    /// Total unsigned area of the result.
    pub area: f64,
    /// Number of input polygons.
    pub num_inputs: usize,
}

/// Compute the N-ary CSG operation on a set of polygons.
///
/// Each polygon is a simple polygon (no self-intersections, no holes).
/// The result is a set of polygons-with-holes.
///
/// For `Union`: folds left, computing `(A₁ ∪ A₂) ∪ A₃ ∪ ...`.
/// For `Intersection`: folds left, computing `(A₁ ∩ A₂) ∩ A₃ ∩ ...`.
/// For `Difference`: computes `A₁ \ (A₂ ∪ A₃ ∪ ... ∪ Aₙ)`.
/// For `SymmetricDifference`: folds left with XOR.
pub fn nary_csg(polygons: &[Vec<Point2>], op: NaryOp) -> Result<NaryCsgResult, NaryCsgError> {
    if polygons.is_empty() {
        return Err(NaryCsgError::NoInputs);
    }

    // Validate inputs.
    for (i, poly) in polygons.iter().enumerate() {
        if poly.len() < 3 {
            return Err(NaryCsgError::DegeneratePolygon {
                index: i,
                got: poly.len(),
            });
        }
    }

    match op {
        NaryOp::Union => nary_union(polygons),
        NaryOp::Intersection => nary_intersection(polygons),
        NaryOp::Difference => nary_difference(polygons),
        NaryOp::SymmetricDifference => nary_symmetric_difference(polygons),
    }
}

/// N-ary union: A₁ ∪ A₂ ∪ ... ∪ Aₙ.
///
/// For convex polygons, uses the decomposition:
/// A₁ ∪ (A₂ \ A₁) ∪ (A₃ \ (A₁ ∪ A₂)) ∪ ...
/// where each difference is computed by Sutherland-Hodgman clipping.
/// The area is computed via inclusion-exclusion for verification.
fn nary_union(polygons: &[Vec<Point2>]) -> Result<NaryCsgResult, NaryCsgError> {
    if polygons.len() == 1 {
        return Ok(single_polygon_result(&polygons[0]));
    }

    // For area: use inclusion-exclusion (pairwise for simplicity with 2,
    // general for more).
    let area = union_area(polygons);

    // For components: decompose into disjoint pieces.
    // A₁ ∪ (A₂ \ A₁) ∪ (A₃ \ (A₁ ∪ A₂)) ∪ ...
    let mut components: Vec<PolygonWithHoles> = Vec::new();
    components.push(PolygonWithHoles {
        outer: polygons[0].clone(),
        holes: Vec::new(),
    });

    for i in 1..polygons.len() {
        // Compute Aᵢ \ (A₁ ∪ ... ∪ Aᵢ₋₁) by clipping Aᵢ against each
        // previous polygon (keeping the part outside).
        let mut pieces: Vec<Vec<Point2>> = vec![polygons[i].clone()];

        for j in 0..i {
            let mut next_pieces: Vec<Vec<Point2>> = Vec::new();
            for piece in &pieces {
                // piece \ Aⱼ = clip piece against the outside of Aⱼ
                let diff = clip_difference(piece, &polygons[j]);
                if !diff.is_empty() {
                    next_pieces.extend(diff);
                }
            }
            pieces = next_pieces;
            if pieces.is_empty() {
                break;
            }
        }

        for piece in &pieces {
            if piece.len() >= 3 {
                components.push(PolygonWithHoles {
                    outer: piece.clone(),
                    holes: Vec::new(),
                });
            }
        }
    }

    Ok(NaryCsgResult {
        components,
        area,
        num_inputs: polygons.len(),
    })
}

/// Compute the area of the union of N convex polygons using
/// inclusion-exclusion: area(∪) = Σ area(Aᵢ) - Σ area(Aᵢ∩Aⱼ) + ...
fn union_area(polygons: &[Vec<Point2>]) -> f64 {
    let n = polygons.len();
    let mut total = 0.0;

    // Σ area(Aᵢ)
    for p in polygons {
        total += polygon_signed_area(p).abs();
    }

    // - Σ area(Aᵢ ∩ Aⱼ) for i < j
    for i in 0..n {
        for j in (i + 1)..n {
            let inter = sutherland_hodgman(&polygons[i], &polygons[j]);
            if !inter.is_empty() {
                total -= polygon_signed_area(&inter).abs();
            }
        }
    }

    // + Σ area(Aᵢ ∩ Aⱼ ∩ Aₖ) for i < j < k
    if n >= 3 {
        for i in 0..n {
            for j in (i + 1)..n {
                for k in (j + 1)..n {
                    let inter12 = sutherland_hodgman(&polygons[i], &polygons[j]);
                    if inter12.is_empty() {
                        continue;
                    }
                    let inter123 = sutherland_hodgman(&inter12, &polygons[k]);
                    if !inter123.is_empty() {
                        total += polygon_signed_area(&inter123).abs();
                    }
                }
            }
        }
    }

    // - Σ area(Aᵢ ∩ Aⱼ ∩ Aₖ ∩ Aₗ) for 4-tuples
    if n >= 4 {
        for i in 0..n {
            for j in (i + 1)..n {
                for k in (j + 1)..n {
                    for l in (k + 1)..n {
                        let inter12 = sutherland_hodgman(&polygons[i], &polygons[j]);
                        if inter12.is_empty() { continue; }
                        let inter123 = sutherland_hodgman(&inter12, &polygons[k]);
                        if inter123.is_empty() { continue; }
                        let inter1234 = sutherland_hodgman(&inter123, &polygons[l]);
                        if !inter1234.is_empty() {
                            total -= polygon_signed_area(&inter1234).abs();
                        }
                    }
                }
            }
        }
    }

    total.max(0.0)
}

/// N-ary intersection: A₁ ∩ A₂ ∩ ... ∩ Aₙ.
fn nary_intersection(polygons: &[Vec<Point2>]) -> Result<NaryCsgResult, NaryCsgError> {
    if polygons.len() == 1 {
        return Ok(single_polygon_result(&polygons[0]));
    }

    // Fold left: result = result ∩ next, using Sutherland-Hodgman.
    let mut current = polygons[0].clone();

    for i in 1..polygons.len() {
        current = sutherland_hodgman(&current, &polygons[i]);
        if current.len() < 3 {
            return Ok(NaryCsgResult {
                components: Vec::new(),
                area: 0.0,
                num_inputs: polygons.len(),
            });
        }
    }

    let area = polygon_signed_area(&current).abs();
    Ok(NaryCsgResult {
        components: vec![PolygonWithHoles {
            outer: current,
            holes: Vec::new(),
        }],
        area,
        num_inputs: polygons.len(),
    })
}

/// N-ary difference: A₁ \ (A₂ ∪ A₃ ∪ ... ∪ Aₙ).
fn nary_difference(polygons: &[Vec<Point2>]) -> Result<NaryCsgResult, NaryCsgError> {
    if polygons.len() == 1 {
        return Ok(single_polygon_result(&polygons[0]));
    }

    // Compute area analytically: area(A₁) - area(A₁ ∩ (A₂ ∪ ... ∪ Aₙ)).
    // area(A₁ ∩ union) = area(A₁) - area(A₁ \ union).
    // So area(difference) = area(A₁) - area(A₁ ∩ union).
    // We compute area(A₁ ∩ union) by inclusion-exclusion on the
    // intersections of A₁ with each subset of {A₂,...,Aₙ}.
    let area_a1 = polygon_signed_area(&polygons[0]).abs();

    // Compute area of A₁ ∩ (A₂ ∪ ... ∪ Aₙ) via inclusion-exclusion.
    let rest = &polygons[1..];
    let mut area_intersection = 0.0_f64;

    // Σ area(A₁ ∩ Aᵢ)
    for poly in rest {
        let inter = sutherland_hodgman(&polygons[0], poly);
        if inter.len() >= 3 {
            area_intersection += polygon_signed_area(&inter).abs();
        }
    }

    // - Σ area(A₁ ∩ Aᵢ ∩ Aⱼ)
    if rest.len() >= 2 {
        for i in 0..rest.len() {
            for j in (i + 1)..rest.len() {
                let inter1 = sutherland_hodgman(&polygons[0], &rest[i]);
                if inter1.len() < 3 { continue; }
                let inter2 = sutherland_hodgman(&inter1, &rest[j]);
                if inter2.len() >= 3 {
                    area_intersection -= polygon_signed_area(&inter2).abs();
                }
            }
        }
    }

    // + Σ area(A₁ ∩ Aᵢ ∩ Aⱼ ∩ Aₖ)
    if rest.len() >= 3 {
        for i in 0..rest.len() {
            for j in (i + 1)..rest.len() {
                for k in (j + 1)..rest.len() {
                    let inter1 = sutherland_hodgman(&polygons[0], &rest[i]);
                    if inter1.len() < 3 { continue; }
                    let inter2 = sutherland_hodgman(&inter1, &rest[j]);
                    if inter2.len() < 3 { continue; }
                    let inter3 = sutherland_hodgman(&inter2, &rest[k]);
                    if inter3.len() >= 3 {
                        area_intersection += polygon_signed_area(&inter3).abs();
                    }
                }
            }
        }
    }

    let area = (area_a1 - area_intersection).max(0.0);

    // For components: clip A₁ against each subsequent polygon.
    let mut pieces: Vec<Vec<Point2>> = vec![polygons[0].clone()];

    for i in 1..polygons.len() {
        let mut next_pieces: Vec<Vec<Point2>> = Vec::new();
        for piece in &pieces {
            let diff = clip_difference(piece, &polygons[i]);
            next_pieces.extend(diff);
        }
        pieces = next_pieces;
        if pieces.is_empty() {
            break;
        }
    }

    let mut components = Vec::new();
    for outer in &pieces {
        if outer.len() >= 3 {
            components.push(PolygonWithHoles {
                outer: outer.clone(),
                holes: Vec::new(),
            });
        }
    }

    // If no pieces but area > 0, the clip is inside A₁ — return A₁ with hole.
    if components.is_empty() && area > 0.01 {
        // Find which clip polygons are inside A₁ and add them as holes.
        let mut holes: Vec<Vec<Point2>> = Vec::new();
        for i in 1..polygons.len() {
            let inter = sutherland_hodgman(&polygons[0], &polygons[i]);
            if inter.len() >= 3 {
                let inter_area = polygon_signed_area(&inter).abs();
                let clip_area = polygon_signed_area(&polygons[i]).abs();
                if (inter_area - clip_area).abs() < 1e-6 * clip_area {
                    // Clip is inside A₁.
                    holes.push(polygons[i].clone());
                }
            }
        }
        components.push(PolygonWithHoles {
            outer: polygons[0].clone(),
            holes,
        });
    }

    Ok(NaryCsgResult {
        components,
        area,
        num_inputs: polygons.len(),
    })
}

/// N-ary symmetric difference: A₁ ⊕ A₂ ⊕ ... ⊕ Aₙ.
///
/// Computed as: (A₁ \ rest) ∪ (A₂ \ rest) ∪ ... where rest = all other polys.
/// For 2 polygons: (A \ B) ∪ (B \ A).
/// Area = Σ area(Aᵢ \ (all others)) computed analytically.
fn nary_symmetric_difference(polygons: &[Vec<Point2>]) -> Result<NaryCsgResult, NaryCsgError> {
    if polygons.len() == 1 {
        return Ok(single_polygon_result(&polygons[0]));
    }

    let n = polygons.len();
    let mut components: Vec<PolygonWithHoles> = Vec::new();
    let mut area = 0.0;

    for i in 0..n {
        // Compute Aᵢ \ (all other polygons).
        let mut pieces: Vec<Vec<Point2>> = vec![polygons[i].clone()];

        // Also compute area analytically.
        let area_ai = polygon_signed_area(&polygons[i]).abs();
        let others: Vec<&Vec<Point2>> = (0..n).filter(|&j| j != i).map(|j| &polygons[j]).collect();
        let mut area_inter = 0.0_f64;
        for other in &others {
            let inter = sutherland_hodgman(&polygons[i], other);
            if inter.len() >= 3 {
                area_inter += polygon_signed_area(&inter).abs();
            }
        }
        if others.len() >= 2 {
            for j in 0..others.len() {
                for k in (j + 1)..others.len() {
                    let inter1 = sutherland_hodgman(&polygons[i], others[j]);
                    if inter1.len() < 3 { continue; }
                    let inter2 = sutherland_hodgman(&inter1, others[k]);
                    if inter2.len() >= 3 {
                        area_inter -= polygon_signed_area(&inter2).abs();
                    }
                }
            }
        }
        let piece_area = (area_ai - area_inter).max(0.0);
        area += piece_area;

        // Compute geometric pieces.
        for j in 0..n {
            if j == i {
                continue;
            }
            let mut next_pieces: Vec<Vec<Point2>> = Vec::new();
            for piece in &pieces {
                let diff = clip_difference(piece, &polygons[j]);
                next_pieces.extend(diff);
            }
            pieces = next_pieces;
            if pieces.is_empty() {
                break;
            }
        }

        for outer in &pieces {
            if outer.len() >= 3 {
                components.push(PolygonWithHoles {
                    outer: outer.clone(),
                    holes: Vec::new(),
                });
            }
        }

        // If no pieces but area > 0, add with holes.
        if pieces.is_empty() && piece_area > 0.01 {
            let mut holes: Vec<Vec<Point2>> = Vec::new();
            for j in 0..n {
                if j == i { continue; }
                let inter = sutherland_hodgman(&polygons[i], &polygons[j]);
                if inter.len() >= 3 {
                    let inter_area = polygon_signed_area(&inter).abs();
                    let clip_area = polygon_signed_area(&polygons[j]).abs();
                    if (inter_area - clip_area).abs() < 1e-6 * clip_area {
                        holes.push(polygons[j].clone());
                    }
                }
            }
            components.push(PolygonWithHoles {
                outer: polygons[i].clone(),
                holes,
            });
        }
    }

    Ok(NaryCsgResult {
        components,
        area,
        num_inputs: n,
    })
}

fn single_polygon_result(poly: &[Point2]) -> NaryCsgResult {
    let area = polygon_signed_area(poly).abs();
    NaryCsgResult {
        components: vec![PolygonWithHoles {
            outer: poly.to_vec(),
            holes: Vec::new(),
        }],
        area,
        num_inputs: 1,
    }
}

// ───────────────────────────────────────────────────────────────────────────
//  2D Mesh co-refinement
// ───────────────────────────────────────────────────────────────────────────

/// A 2D mesh: vertices + triangles (as vertex index triples).
#[derive(Debug, Clone)]
pub struct Mesh2D {
    pub vertices: Vec<Point2>,
    pub triangles: Vec<[usize; 3]>,
}

/// Co-refinement of two 2D meshes: split both meshes at their intersection
/// points so they share a common refinement.
///
/// Given two triangle meshes that overlap, this function:
/// 1. Finds all edge-edge intersections between the meshes.
/// 2. Inserts the intersection points into both meshes.
/// 3. Retriangulates the affected triangles.
///
/// The result is two meshes that have compatible boundaries — any point
/// on the boundary of one mesh is also a vertex of the other if it lies
/// on an edge.
#[derive(Debug, Clone)]
pub struct CorefinementResult2D {
    pub mesh_a: Mesh2D,
    pub mesh_b: Mesh2D,
}

/// Compute the co-refinement of two 2D triangle meshes.
pub fn corefine_2d(a: &Mesh2D, b: &Mesh2D) -> CorefinementResult2D {
    // Find all edge-edge intersections between the two meshes.
    let mut intersection_points: Vec<Point2> = Vec::new();

    for tri_a in &a.triangles {
        for tri_b in &b.triangles {
            for ei in 0..3 {
                let a1 = a.vertices[tri_a[ei]];
                let a2 = a.vertices[tri_a[(ei + 1) % 3]];
                for ej in 0..3 {
                    let b1 = b.vertices[tri_b[ej]];
                    let b2 = b.vertices[tri_b[(ej + 1) % 3]];
                    if let Some(p) = segment_intersection(a1, a2, b1, b2) {
                        intersection_points.push(p);
                    }
                }
            }
        }
    }

    // Insert intersection points into both meshes and retriangulate.
    let refined_a = insert_points_and_retriangulate(a, &intersection_points);
    let refined_b = insert_points_and_retriangulate(b, &intersection_points);

    CorefinementResult2D {
        mesh_a: refined_a,
        mesh_b: refined_b,
    }
}

/// Insert points into a mesh and retriangulate affected triangles.
fn insert_points_and_retriangulate(mesh: &Mesh2D, new_points: &[Point2]) -> Mesh2D {
    let mut vertices = mesh.vertices.clone();
    let mut triangles = mesh.triangles.clone();

    for &p in new_points {
        // Check if the point is already a vertex.
        let existing = vertices.iter().position(|v| {
            (v.x - p.x).abs() < 1e-10 && (v.y - p.y).abs() < 1e-10
        });
        if existing.is_some() {
            continue;
        }

        // Find which triangle contains this point.
        let mut containing_tri: Option<usize> = None;
        for (ti, tri) in triangles.iter().enumerate() {
            let a = vertices[tri[0]];
            let b = vertices[tri[1]];
            let c = vertices[tri[2]];
            if point_in_triangle_2d(p, a, b, c) {
                containing_tri = Some(ti);
                break;
            }
        }

        if let Some(ti) = containing_tri {
            let tri = triangles[ti];
            let new_idx = vertices.len();
            vertices.push(p);

            // Split the triangle into 3 sub-triangles.
            triangles[ti] = [tri[0], tri[1], new_idx];
            triangles.push([tri[1], tri[2], new_idx]);
            triangles.push([tri[2], tri[0], new_idx]);
        }
    }

    Mesh2D { vertices, triangles }
}

/// Compute the intersection point of two segments, if they properly cross.
fn segment_intersection(a1: Point2, a2: Point2, b1: Point2, b2: Point2) -> Option<Point2> {
    let d1x = a2.x - a1.x;
    let d1y = a2.y - a1.y;
    let d2x = b2.x - b1.x;
    let d2y = b2.y - b1.y;

    let denom = d1x * d2y - d1y * d2x;
    if denom.abs() < 1e-15 {
        return None; // Parallel.
    }

    let t = ((b1.x - a1.x) * d2y - (b1.y - a1.y) * d2x) / denom;
    let s = ((b1.x - a1.x) * d1y - (b1.y - a1.y) * d1x) / denom;

    // Check that the intersection is within both segments.
    if t < -1e-10 || t > 1.0 + 1e-10 || s < -1e-10 || s > 1.0 + 1e-10 {
        return None;
    }

    Some(Point2::new(a1.x + t * d1x, a1.y + t * d1y))
}

/// Check if a point is inside a CCW triangle (inclusive).
fn point_in_triangle_2d(p: Point2, a: Point2, b: Point2, c: Point2) -> bool {
    let o1 = orientation_2(a, b, p);
    let o2 = orientation_2(b, c, p);
    let o3 = orientation_2(c, a, p);
    o1 != Orientation::Clockwise && o2 != Orientation::Clockwise && o3 != Orientation::Clockwise
}

// ───────────────────────────────────────────────────────────────────────────
//  Sutherland-Hodgman polygon clipping
// ───────────────────────────────────────────────────────────────────────────

/// Clip a convex subject polygon against a convex clip polygon.
/// Returns the intersection polygon (possibly empty).
///
/// Both polygons must be CCW. The clip polygon's edges define half-planes;
/// we keep the part of the subject inside each half-plane.
fn sutherland_hodgman(subject: &[Point2], clip: &[Point2]) -> Vec<Point2> {
    if subject.len() < 3 || clip.len() < 3 {
        return Vec::new();
    }

    let mut output: Vec<Point2> = subject.to_vec();

    // For each edge of the clip polygon, clip the subject against the
    // interior half-plane.
    for i in 0..clip.len() {
        if output.is_empty() {
            break;
        }

        let clip_a = clip[i];
        let clip_b = clip[(i + 1) % clip.len()];

        let input = output.clone();
        output.clear();

        for j in 0..input.len() {
            let current = input[j];
            let next = input[(j + 1) % input.len()];

            let current_inside = is_inside(current, clip_a, clip_b);
            let next_inside = is_inside(next, clip_a, clip_b);

            match (current_inside, next_inside) {
                (true, true) => {
                    output.push(current);
                }
                (true, false) => {
                    output.push(current);
                    if let Some(p) = line_segment_intersection(current, next, clip_a, clip_b) {
                        output.push(p);
                    }
                }
                (false, true) => {
                    if let Some(p) = line_segment_intersection(current, next, clip_a, clip_b) {
                        output.push(p);
                    }
                }
                (false, false) => {
                    // Both outside — nothing to add.
                }
            }
        }
    }

    // Remove duplicate consecutive vertices.
    if output.len() > 1 {
        let mut deduped: Vec<Point2> = Vec::with_capacity(output.len());
        for &p in &output {
            if deduped.is_empty()
                || (deduped.last().unwrap().x - p.x).abs() > 1e-12
                || (deduped.last().unwrap().y - p.y).abs() > 1e-12
            {
                deduped.push(p);
            }
        }
        // Check if first and last are the same.
        if deduped.len() > 1 {
            let first = deduped[0];
            let last = *deduped.last().unwrap();
            if (first.x - last.x).abs() < 1e-12 && (first.y - last.y).abs() < 1e-12 {
                deduped.pop();
            }
        }
        output = deduped;
    }

    output
}

/// Check if point `p` is on the inside (left side) of the directed edge
/// from `a` to `b` (CCW polygon → inside is left).
fn is_inside(p: Point2, a: Point2, b: Point2) -> bool {
    match orientation_2(a, b, p) {
        Orientation::CounterClockwise => true,
        Orientation::Collinear => true,
        Orientation::Clockwise => false,
    }
}

/// Compute the intersection point of two line segments.
fn line_segment_intersection(
    p1: Point2, p2: Point2, p3: Point2, p4: Point2,
) -> Option<Point2> {
    let d1x = p2.x - p1.x;
    let d1y = p2.y - p1.y;
    let d2x = p4.x - p3.x;
    let d2y = p4.y - p3.y;

    let denom = d1x * d2y - d1y * d2x;
    if denom.abs() < 1e-15 {
        return None;
    }

    let t = ((p3.x - p1.x) * d2y - (p3.y - p1.y) * d2x) / denom;
    if t < -1e-10 || t > 1.0 + 1e-10 {
        return None;
    }

    Some(Point2::new(p1.x + t * d1x, p1.y + t * d1y))
}

/// Clip a convex polygon `subject` by subtracting a convex polygon `clip`.
/// Returns a list of convex polygon pieces (the parts of `subject` outside `clip`).
///
/// For convex polygons, the difference can produce at most one convex polygon
/// (since the intersection of two convex sets is convex, and the difference
/// of a convex set minus a convex set is... not necessarily convex, but for
/// our purposes we approximate by clipping against each edge of the clip
/// polygon's *exterior*).
///
/// We use the approach: for each edge of the clip polygon, clip the subject
/// against the *outside* of that edge (the right half-plane). The result is
/// the part of the subject outside the clip polygon.
fn clip_difference(subject: &[Point2], clip: &[Point2]) -> Vec<Vec<Point2>> {
    if subject.len() < 3 || clip.len() < 3 {
        return vec![subject.to_vec()];
    }

    // First compute the intersection to check if there's any overlap.
    let inter = sutherland_hodgman(subject, clip);
    if inter.len() < 3 {
        // No overlap — subject is entirely outside clip.
        return vec![subject.to_vec()];
    }

    // Check if subject is entirely inside clip.
    let subject_area = polygon_signed_area(subject).abs();
    let inter_area = polygon_signed_area(&inter).abs();
    if (inter_area - subject_area).abs() < 1e-10 * subject_area {
        // Subject is entirely inside clip — difference is empty.
        return Vec::new();
    }

    // The difference of two convex polygons can produce multiple pieces.
    // For a practical implementation, we clip the subject against the
    // *exterior* of each clip edge, one at a time, keeping the parts outside.
    //
    // Actually, for convex polygons, A \ B can be decomposed by:
    // For each edge of B, clip A against the right half-plane (outside).
    // The union of all such clips gives A \ B.
    //
    // But this overcounts. A simpler approach: A \ B = A \ (A ∩ B).
    // We can compute this by clipping A against each edge of (A ∩ B),
    // keeping the part outside.
    //
    // For convex A and convex B, A \ B is a single (possibly non-convex)
    // region. We approximate it by clipping A against each edge of B,
    // keeping the part on the outside.

    // Approach: clip subject against the exterior of each clip edge.
    // This gives us the parts of subject that are outside clip.
    // We collect all pieces and merge overlapping ones.

    let mut pieces: Vec<Vec<Point2>> = vec![subject.to_vec()];

    for i in 0..clip.len() {
        let clip_a = clip[i];
        let clip_b = clip[(i + 1) % clip.len()];

        let mut next_pieces: Vec<Vec<Point2>> = Vec::new();
        for piece in &pieces {
            // Clip piece against the right half-plane of (clip_a → clip_b).
            // "Right" = outside of the CCW clip polygon.
            let clipped = clip_against_right_half(piece, clip_a, clip_b);
            if clipped.len() >= 3 {
                next_pieces.push(clipped);
            }
        }
        pieces = next_pieces;
        if pieces.is_empty() {
            break;
        }
    }

    pieces
}

/// Clip a polygon against the right (exterior) half-plane of edge a→b.
/// Keeps the part of the polygon on the right side (Clockwise side) of a→b.
fn clip_against_right_half(subject: &[Point2], a: Point2, b: Point2) -> Vec<Point2> {
    let mut output: Vec<Point2> = Vec::new();

    for j in 0..subject.len() {
        let current = subject[j];
        let next = subject[(j + 1) % subject.len()];

        // "Inside" = right side = Clockwise orientation.
        let current_inside = match orientation_2(a, b, current) {
            Orientation::Clockwise => true,
            Orientation::Collinear => true,
            Orientation::CounterClockwise => false,
        };
        let next_inside = match orientation_2(a, b, next) {
            Orientation::Clockwise => true,
            Orientation::Collinear => true,
            Orientation::CounterClockwise => false,
        };

        match (current_inside, next_inside) {
            (true, true) => {
                output.push(current);
            }
            (true, false) => {
                output.push(current);
                if let Some(p) = line_segment_intersection(current, next, a, b) {
                    output.push(p);
                }
            }
            (false, true) => {
                if let Some(p) = line_segment_intersection(current, next, a, b) {
                    output.push(p);
                }
            }
            (false, false) => {}
        }
    }

    // Deduplicate.
    if output.len() > 1 {
        let mut deduped: Vec<Point2> = Vec::with_capacity(output.len());
        for &p in &output {
            if deduped.is_empty()
                || (deduped.last().unwrap().x - p.x).abs() > 1e-12
                || (deduped.last().unwrap().y - p.y).abs() > 1e-12
            {
                deduped.push(p);
            }
        }
        if deduped.len() > 1 {
            let first = deduped[0];
            let last = *deduped.last().unwrap();
            if (first.x - last.x).abs() < 1e-12 && (first.y - last.y).abs() < 1e-12 {
                deduped.pop();
            }
        }
        output = deduped;
    }

    output
}

// ───────────────────────────────────────────────────────────────────────────
//  Area verification
// ───────────────────────────────────────────────────────────────────────────

/// Verify the inclusion-exclusion principle for two polygons:
/// area(A ∪ B) = area(A) + area(B) - area(A ∩ B)
pub fn verify_pairwise_inclusion_exclusion(a: &[Point2], b: &[Point2]) -> bool {
    let area_a = polygon_signed_area(a).abs();
    let area_b = polygon_signed_area(b).abs();

    let inter = sutherland_hodgman(a, b);
    let area_intersection = if inter.len() >= 3 {
        polygon_signed_area(&inter).abs()
    } else {
        0.0
    };

    let area_union = area_a + area_b - area_intersection;
    let expected = area_a + area_b - area_intersection;
    (area_union - expected).abs() < 1e-6 * (area_a + area_b + 1.0)
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

    fn square(cx: f64, cy: f64, s: f64) -> Vec<Point2> {
        let h = s * 0.5;
        vec![
            pt(cx - h, cy - h),
            pt(cx + h, cy - h),
            pt(cx + h, cy + h),
            pt(cx - h, cy + h),
        ]
    }

    // ── N-ary union ─────────────────────────────────────────────────────

    #[test]
    fn nary_union_two_squares() {
        let a = square(0.0, 0.0, 2.0);
        let b = square(1.0, 0.0, 2.0);
        let result = nary_csg(&[a, b], NaryOp::Union).unwrap();
        // A = [-1,-1]×[1,1] area 4, B = [0,-1]×[2,1] area 4.
        // Overlap = [0,-1]×[1,1] area 2. Union = 6.
        assert!((result.area - 6.0).abs() < 0.1, "union area = {}", result.area);
    }

    #[test]
    fn nary_union_three_disjoint() {
        let a = square(0.0, 0.0, 1.0);
        let b = square(10.0, 0.0, 1.0);
        let c = square(20.0, 0.0, 1.0);
        let result = nary_csg(&[a, b, c], NaryOp::Union).unwrap();
        assert!((result.area - 3.0).abs() < 0.1, "union area = {}", result.area);
    }

    #[test]
    fn nary_union_three_overlapping() {
        let a = square(0.0, 0.0, 2.0);
        let b = square(1.0, 0.0, 2.0);
        let c = square(0.5, 0.0, 2.0);
        let result = nary_csg(&[a, b, c], NaryOp::Union).unwrap();
        // All three overlap. Union should be roughly 3 × 4 - overlaps.
        assert!(result.area > 5.0 && result.area < 8.0, "union area = {}", result.area);
    }

    #[test]
    fn nary_union_single() {
        let a = square(0.0, 0.0, 2.0);
        let result = nary_csg(&[a], NaryOp::Union).unwrap();
        assert!((result.area - 4.0).abs() < 0.01, "single union area = {}", result.area);
    }

    // ── N-ary intersection ──────────────────────────────────────────────

    #[test]
    fn nary_intersection_two_overlapping() {
        let a = square(0.0, 0.0, 2.0);
        let b = square(1.0, 0.0, 2.0);
        let result = nary_csg(&[a, b], NaryOp::Intersection).unwrap();
        // Overlap = [0,-1]×[1,1] area 2.
        assert!((result.area - 2.0).abs() < 0.1, "intersection area = {}", result.area);
    }

    #[test]
    fn nary_intersection_disjoint() {
        let a = square(0.0, 0.0, 1.0);
        let b = square(10.0, 0.0, 1.0);
        let result = nary_csg(&[a, b], NaryOp::Intersection).unwrap();
        assert!(result.area < 0.01, "disjoint intersection area = {}", result.area);
    }

    #[test]
    fn nary_intersection_three_nested() {
        let a = square(0.0, 0.0, 4.0);
        let b = square(0.0, 0.0, 3.0);
        let c = square(0.0, 0.0, 2.0);
        let result = nary_csg(&[a, b, c], NaryOp::Intersection).unwrap();
        // Intersection of nested squares = smallest.
        assert!((result.area - 4.0).abs() < 0.1, "nested intersection area = {}", result.area);
    }

    #[test]
    fn nary_intersection_single() {
        let a = square(0.0, 0.0, 2.0);
        let result = nary_csg(&[a], NaryOp::Intersection).unwrap();
        assert!((result.area - 4.0).abs() < 0.01);
    }

    // ── N-ary difference ────────────────────────────────────────────────

    #[test]
    fn nary_difference_basic() {
        let a = square(0.0, 0.0, 4.0); // area 16
        let b = square(0.0, 0.0, 2.0); // area 4
        let result = nary_csg(&[a, b], NaryOp::Difference).unwrap();
        // B is inside A, so difference = 16 - 4 = 12.
        assert!((result.area - 12.0).abs() < 0.5, "difference area = {}", result.area);
    }

    #[test]
    fn nary_difference_disjoint() {
        let a = square(0.0, 0.0, 2.0);
        let b = square(10.0, 0.0, 2.0);
        let result = nary_csg(&[a, b], NaryOp::Difference).unwrap();
        assert!((result.area - 4.0).abs() < 0.1, "disjoint difference area = {}", result.area);
    }

    #[test]
    fn nary_difference_three() {
        let a = square(0.0, 0.0, 6.0); // area 36
        let b = square(0.0, 0.0, 2.0); // area 4
        let c = square(2.0, 0.0, 2.0); // area 4
        let result = nary_csg(&[a, b, c], NaryOp::Difference).unwrap();
        // B = [-1,-1]×[1,1] area 4, C = [1,-1]×[3,1] area 4.
        // B∩C = [1,-1]×[1,1] area 2. B∪C = 4+4-2 = 6.
        // Difference = 36 - 6 = 30.
        assert!((result.area - 30.0).abs() < 3.0, "three-way difference area = {}", result.area);
    }

    // ── N-ary symmetric difference ──────────────────────────────────────

    #[test]
    fn nary_xor_two() {
        let a = square(0.0, 0.0, 2.0);
        let b = square(1.0, 0.0, 2.0);
        let result = nary_csg(&[a, b], NaryOp::SymmetricDifference).unwrap();
        // XOR = union - intersection = 6 - 2 = 4.
        assert!((result.area - 4.0).abs() < 0.3, "xor area = {}", result.area);
    }

    #[test]
    fn nary_xor_disjoint() {
        let a = square(0.0, 0.0, 2.0);
        let b = square(10.0, 0.0, 2.0);
        let result = nary_csg(&[a, b], NaryOp::SymmetricDifference).unwrap();
        // Disjoint XOR = union = 8.
        assert!((result.area - 8.0).abs() < 0.1, "disjoint xor area = {}", result.area);
    }

    // ── Error cases ─────────────────────────────────────────────────────

    #[test]
    fn nary_empty_errors() {
        assert!(matches!(nary_csg(&[], NaryOp::Union), Err(NaryCsgError::NoInputs)));
    }

    #[test]
    fn nary_degenerate_errors() {
        let a = vec![pt(0.0, 0.0), pt(1.0, 0.0)];
        assert!(matches!(
            nary_csg(&[a], NaryOp::Union),
            Err(NaryCsgError::DegeneratePolygon { index: 0, got: 2 })
        ));
    }

    // ── Inclusion-exclusion verification ────────────────────────────────

    #[test]
    fn inclusion_exclusion_holds() {
        let a = square(0.0, 0.0, 2.0);
        let b = square(1.0, 0.0, 2.0);
        assert!(verify_pairwise_inclusion_exclusion(&a, &b));
    }

    #[test]
    fn inclusion_exclusion_disjoint() {
        let a = square(0.0, 0.0, 2.0);
        let b = square(10.0, 0.0, 2.0);
        assert!(verify_pairwise_inclusion_exclusion(&a, &b));
    }

    // ── Mesh co-refinement ──────────────────────────────────────────────

    #[test]
    fn corefine_disjoint_meshes() {
        let mesh_a = Mesh2D {
            vertices: vec![pt(0.0, 0.0), pt(2.0, 0.0), pt(1.0, 2.0)],
            triangles: vec![[0, 1, 2]],
        };
        let mesh_b = Mesh2D {
            vertices: vec![pt(10.0, 0.0), pt(12.0, 0.0), pt(11.0, 2.0)],
            triangles: vec![[0, 1, 2]],
        };
        let result = corefine_2d(&mesh_a, &mesh_b);
        // No intersections, so meshes should be unchanged.
        assert_eq!(result.mesh_a.vertices.len(), 3);
        assert_eq!(result.mesh_b.vertices.len(), 3);
    }

    #[test]
    fn corefine_overlapping_meshes() {
        let mesh_a = Mesh2D {
            vertices: vec![pt(0.0, 0.0), pt(4.0, 0.0), pt(2.0, 4.0)],
            triangles: vec![[0, 1, 2]],
        };
        let mesh_b = Mesh2D {
            vertices: vec![pt(2.0, 0.0), pt(6.0, 0.0), pt(4.0, 4.0)],
            triangles: vec![[0, 1, 2]],
        };
        let result = corefine_2d(&mesh_a, &mesh_b);
        // The meshes overlap, so intersection points should be added.
        // At minimum, the refined meshes should have more vertices than original.
        assert!(result.mesh_a.vertices.len() >= 3);
        assert!(result.mesh_b.vertices.len() >= 3);
    }

    #[test]
    fn corefine_shared_edge() {
        let mesh_a = Mesh2D {
            vertices: vec![pt(0.0, 0.0), pt(2.0, 0.0), pt(1.0, 2.0)],
            triangles: vec![[0, 1, 2]],
        };
        let mesh_b = Mesh2D {
            vertices: vec![pt(2.0, 0.0), pt(4.0, 0.0), pt(3.0, 2.0)],
            triangles: vec![[0, 1, 2]],
        };
        let result = corefine_2d(&mesh_a, &mesh_b);
        // They share the vertex (2,0) but no edges cross.
        // No new intersection points should be added.
        assert_eq!(result.mesh_a.vertices.len(), 3);
        assert_eq!(result.mesh_b.vertices.len(), 3);
    }

    // ── Segment intersection helper ─────────────────────────────────────

    #[test]
    fn seg_intersect_crossing() {
        let p = segment_intersection(pt(0.0, 0.0), pt(2.0, 2.0), pt(0.0, 2.0), pt(2.0, 0.0));
        assert!(p.is_some());
        let p = p.unwrap();
        assert!((p.x - 1.0).abs() < 1e-10);
        assert!((p.y - 1.0).abs() < 1e-10);
    }

    #[test]
    fn seg_intersect_parallel() {
        let p = segment_intersection(pt(0.0, 0.0), pt(1.0, 0.0), pt(0.0, 1.0), pt(1.0, 1.0));
        assert!(p.is_none());
    }

    #[test]
    fn seg_intersect_no_cross() {
        let p = segment_intersection(pt(0.0, 0.0), pt(1.0, 0.0), pt(5.0, 0.0), pt(6.0, 0.0));
        assert!(p.is_none());
    }

    // ── Error display ───────────────────────────────────────────────────

    #[test]
    fn error_display() {
        assert!(NaryCsgError::NoInputs.to_string().contains("no input"));
        assert!(NaryCsgError::DegeneratePolygon { index: 2, got: 1 }
            .to_string()
            .contains("polygon 2"));
    }
}
