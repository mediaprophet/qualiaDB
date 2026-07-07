//! Combinatorial map (2-map / CMap) minimal core.
//!
//! A combatorial map generalises the half-edge structure. Its primitive is the
//! **dart** (one directed edge, equivalent to a `HalfEdge`). A 2-map is defined
//! by two involutions on its darts:
//!
//! - **β₁ (beta-1)** — maps each dart to the next dart in the same face. This is
//!   a *permutation* whose cycles are the faces. It is exactly `HalfEdge::next`.
//! - **β₂ (beta-2)** — maps each dart to its twin (the oppositely-oriented dart
//!   on the same edge). This is an *involution* (`β₂(β₂(d)) == d`). Boundary
//!   darts carry `β₂ == INVALID_INDEX`.
//!
//! The `Dart` layout is field-for-field identical to `HalfEdge` (renamed), so
//! the half-edge ↔ 2-map round-trip is byte-identical by construction. The
//! *conceptual* distinction is that a 2-map is defined by its involutions, and
//! [`validate_combinatorial_map`] enforces the involution / permutation /
//! face-consistency / origin-connectivity invariants that a raw `&[HalfEdge]`
//! array does not.
//!
//! All public functions are caller-buffered and zero-heap (no `Vec` / `String`
//! / `Box`). Test helpers may allocate for setup only.

use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};

use super::topology::{HalfEdge, INVALID_INDEX};

// ---------------------------------------------------------------------------
// Dart POD
// ---------------------------------------------------------------------------

/// One dart of a 2-map. Field-for-field identical to [`HalfEdge`] (renamed):
/// `beta1` == `next`, `beta2` == `twin`, `face` == `face`, `origin` == `origin`.
///
/// 16 bytes, `#[repr(C)]`, `Pod`/`Zeroable` so it can be `bytemuck::cast_slice`d
/// alongside `HalfEdge` buffers where ABI-compatible.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Pod, Zeroable, Serialize, Deserialize)]
pub struct Dart {
    /// Next dart in the same face (β₁ permutation). == `HalfEdge::next`.
    pub beta1: u32,
    /// Twin dart (β₂ involution). `INVALID_INDEX` for boundary darts.
    /// == `HalfEdge::twin`.
    pub beta2: u32,
    /// Face this dart belongs to. == `HalfEdge::face`.
    pub face: u32,
    /// Origin vertex of this dart. == `HalfEdge::origin`.
    pub origin: u32,
}

impl Default for Dart {
    #[inline]
    fn default() -> Self {
        Self {
            beta1: INVALID_INDEX,
            beta2: INVALID_INDEX,
            face: INVALID_INDEX,
            origin: INVALID_INDEX,
        }
    }
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors raised by the combinatorial-map core. Each variant names the specific
/// invariant that was violated so callers can surface a precise diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CMapError {
    /// `darts` buffer too small to hold the converted half-edges.
    DartBufferTooSmall { required: usize },
    /// `half_edges` buffer too small to hold the converted darts.
    HalfEdgeBufferTooSmall { required: usize },
    /// A β₁ or β₂ link points outside the dart array.
    DartOutOfRange { index: u32 },
    /// A β₁ link points to `INVALID_INDEX` mid-cycle (a dangling next pointer).
    Beta1Dangling { dart: u32, points_to: u32 },
    /// Following β₁ from `dart` did not return to `dart` within `len` hops —
    /// the permutation is malformed (not a finite cycle).
    Beta1NotPermutation { dart: u32 },
    /// `β₂(β₂(d)) != d` — the twin link is not an involution.
    Beta2NotInvolution {
        dart: u32,
        beta2: u32,
        beta2_beta2: u32,
    },
    /// Two darts in the same β₁-cycle carry different `face` fields.
    FaceInconsistent { dart: u32, face_a: u32, face_b: u32 },
    /// The origin connectivity invariant failed:
    /// `origin(d) != origin(β₁(β₂(d)))` for an interior dart.
    OriginInconsistent {
        dart: u32,
        expected: u32,
        found: u32,
    },
}

// ---------------------------------------------------------------------------
// Conversions (byte-for-byte field mapping)
// ---------------------------------------------------------------------------

/// Convert a half-edge array into a dart array.
///
/// Field mapping is byte-for-byte: `next → beta1`, `twin → beta2`, `face → face`,
/// `origin → origin`. The dart buffer must hold at least `half_edges.len()`
/// entries; on success returns the number of darts written.
///
/// Zero-heap. Deterministic. The inverse of [`darts_to_half_edges`].
#[inline]
pub fn half_edges_to_darts(
    half_edges: &[HalfEdge],
    darts: &mut [Dart],
) -> Result<usize, CMapError> {
    let n = half_edges.len();
    if darts.len() < n {
        return Err(CMapError::DartBufferTooSmall { required: n });
    }
    for (src, dst) in half_edges.iter().zip(darts.iter_mut()) {
        dst.beta1 = src.next;
        dst.beta2 = src.twin;
        dst.face = src.face;
        dst.origin = src.origin;
    }
    Ok(n)
}

/// Convert a dart array back into a half-edge array.
///
/// Inverse of [`half_edges_to_darts`]: `beta1 → next`, `beta2 → twin`,
/// `face → face`, `origin → origin`. The half-edge buffer must hold at least
/// `darts.len()` entries; on success returns the number of half-edges written.
///
/// Zero-heap. Deterministic.
#[inline]
pub fn darts_to_half_edges(
    darts: &[Dart],
    half_edges: &mut [HalfEdge],
) -> Result<usize, CMapError> {
    let n = darts.len();
    if half_edges.len() < n {
        return Err(CMapError::HalfEdgeBufferTooSmall { required: n });
    }
    for (src, dst) in darts.iter().zip(half_edges.iter_mut()) {
        dst.next = src.beta1;
        dst.twin = src.beta2;
        dst.face = src.face;
        dst.origin = src.origin;
    }
    Ok(n)
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

/// Validate the 2-map invariants over `darts`:
///
/// 1. **β₁ is a permutation** — every dart's `beta1` is either a valid index or
///    `INVALID_INDEX` only as the *last* hop of a cycle that has already
///    returned to its start (in practice a well-formed `build_triangle_half_edges`
///    graph never stores `INVALID_INDEX` in `next`, so any `INVALID_INDEX` is
///    reported as `Beta1Dangling`). Following `beta1` from any dart returns to
///    it within `len` hops (`Beta1NotPermutation` otherwise).
/// 2. **β₂ is an involution** — for every dart `d` with `beta2(d) != INVALID_INDEX`,
///    `beta2(beta2(d)) == d` (`Beta2NotInvolution`). Boundary darts keep
///    `beta2 == INVALID_INDEX`.
/// 3. **β₂ symmetry** — folded into the involution check (β₂(β₂(d)) == d).
/// 4. **Face consistency** — all darts in a β₁-cycle share the same `face`
///    (`FaceInconsistent`).
/// 5. **Origin connectivity** — for every interior dart `d` (i.e.
///    `beta2(d) != INVALID_INDEX`), `origin(d) == origin(beta1(beta2(d)))`
///    (`OriginInconsistent`). This is the half-edge connectivity invariant: the
///    dart after the twin starts at the same vertex this dart ends at.
///
/// All link targets are range-checked first (`DartOutOfRange`).
///
/// Zero-heap: the cycle-walk uses a fixed-size visited-bitset only when needed
/// for cycle face-consistency; here we walk each cycle in O(cycle) without
/// allocation by re-walking from each dart (total O(N·max_cycle) worst case,
/// O(N) for typical triangle meshes where every cycle has length 3).
#[allow(clippy::too_many_lines)]
pub fn validate_combinatorial_map(darts: &[Dart]) -> Result<(), CMapError> {
    let len = darts.len();
    if len == 0 {
        return Ok(());
    }
    let len_u32 = len as u32;

    // ---- Pass 1: range-check every link + β₂ involution -------------------
    for (i, d) in darts.iter().enumerate() {
        let i_u32 = i as u32;

        // β₁ range / dangling.
        if d.beta1 == INVALID_INDEX {
            return Err(CMapError::Beta1Dangling {
                dart: i_u32,
                points_to: INVALID_INDEX,
            });
        }
        if d.beta1 >= len_u32 {
            return Err(CMapError::DartOutOfRange { index: d.beta1 });
        }

        // β₂ range + involution. Boundary (INVALID_INDEX) is allowed.
        if d.beta2 != INVALID_INDEX {
            if d.beta2 >= len_u32 {
                return Err(CMapError::DartOutOfRange { index: d.beta2 });
            }
            let partner = darts[d.beta2 as usize];
            if partner.beta2 != i_u32 {
                return Err(CMapError::Beta2NotInvolution {
                    dart: i_u32,
                    beta2: d.beta2,
                    beta2_beta2: partner.beta2,
                });
            }
        }
    }

    // ---- Pass 2: β₁ permutation (every dart returns to itself in ≤ len hops)
    for (start, _) in darts.iter().enumerate() {
        let start_u32 = start as u32;
        let mut cur = start_u32;
        let mut hops = 0usize;
        loop {
            cur = darts[cur as usize].beta1;
            if cur == start_u32 {
                break;
            }
            hops += 1;
            if hops >= len {
                return Err(CMapError::Beta1NotPermutation { dart: start_u32 });
            }
        }
    }

    // ---- Pass 3: face consistency within each β₁-cycle --------------------
    // For each dart, walk its cycle and confirm every member shares its face.
    // O(N · cycle_len); for triangle meshes cycle_len == 3 so this is O(N).
    for (start, d) in darts.iter().enumerate() {
        let start_u32 = start as u32;
        let face0 = d.face;
        let mut cur = d.beta1;
        while cur != start_u32 {
            let member = darts[cur as usize];
            if member.face != face0 {
                return Err(CMapError::FaceInconsistent {
                    dart: start_u32,
                    face_a: face0,
                    face_b: member.face,
                });
            }
            cur = member.beta1;
        }
    }

    // ---- Pass 4: origin connectivity (interior darts only) ----------------
    // For an interior dart d: origin(d) must equal origin(β₁(β₂(d))).
    // β₁(β₂(d)) is the dart that follows the twin — i.e. the next edge of the
    // adjacent face, which must start at the vertex d ends at. Because
    // HalfEdge encodes the *origin* (not the destination), the half-edge
    // invariant is: origin(d) == origin(next(twin(d))) for interior d.
    for (i, d) in darts.iter().enumerate() {
        let i_u32 = i as u32;
        if d.beta2 == INVALID_INDEX {
            continue; // boundary dart — no twin to check against.
        }
        let twin = d.beta2;
        let after_twin = darts[twin as usize].beta1;
        // after_twin is range-checked in pass 1 (it's a beta1 link).
        let expected = darts[after_twin as usize].origin;
        if d.origin != expected {
            return Err(CMapError::OriginInconsistent {
                dart: i_u32,
                expected,
                found: d.origin,
            });
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::specialized_libs::computational_geometry::topology::{
        build_triangle_half_edges, required_edge_slots, EdgeSlot, HalfEdge,
    };

    /// Build a half-edge graph from triangles and return the edge slice.
    /// Uses `vec!` for test setup only (allowed by project rules).
    fn build_he(vertex_count: u32, triangles: &[[u32; 3]]) -> Vec<HalfEdge> {
        let n = triangles.len() * 3;
        let mut edges = vec![HalfEdge::default(); n];
        let mut slots = vec![EdgeSlot::default(); required_edge_slots(triangles.len())];
        let summary =
            build_triangle_half_edges(vertex_count, triangles, &mut edges, &mut slots).unwrap();
        assert_eq!(summary.half_edge_count as usize, n);
        edges
    }

    fn round_trip(edges: &[HalfEdge]) -> Vec<HalfEdge> {
        let mut darts = vec![Dart::default(); edges.len()];
        let written = half_edges_to_darts(edges, &mut darts).unwrap();
        assert_eq!(written, edges.len());
        let mut back = vec![HalfEdge::default(); edges.len()];
        let written2 = darts_to_half_edges(&darts, &mut back).unwrap();
        assert_eq!(written2, edges.len());
        back
    }

    fn assert_byte_identical(a: &[HalfEdge], b: &[HalfEdge]) {
        assert_eq!(a.len(), b.len(), "length mismatch");
        for (i, (x, y)) in a.iter().zip(b.iter()).enumerate() {
            assert_eq!(x, y, "byte mismatch at half-edge {i}");
        }
    }

    // --- Golden meshes -----------------------------------------------------

    fn single_triangle() -> Vec<HalfEdge> {
        build_he(3, &[[0, 1, 2]])
    }

    fn two_triangles_shared_edge() -> Vec<HalfEdge> {
        // Triangles (0,1,2) and (2,1,3) share the oriented edge 1→2 / 2→1.
        build_he(4, &[[0, 1, 2], [2, 1, 3]])
    }

    fn tetrahedron() -> Vec<HalfEdge> {
        // Closed mesh: 4 triangles, every edge has a twin.
        build_he(4, &[[0, 1, 2], [0, 2, 3], [0, 3, 1], [1, 3, 2]])
    }

    fn boundary_mesh() -> Vec<HalfEdge> {
        // Two triangles sharing only a vertex — open mesh, boundary darts present.
        build_he(5, &[[0, 1, 2], [0, 3, 4]])
    }

    // ======================================================================
    // 1–4. Round-trip byte-identity on golden meshes
    // ======================================================================

    #[test]
    fn round_trip_single_triangle_byte_identical() {
        let edges = single_triangle();
        let back = round_trip(&edges);
        assert_byte_identical(&edges, &back);
    }

    #[test]
    fn round_trip_two_triangles_shared_edge_byte_identical() {
        let edges = two_triangles_shared_edge();
        // Sanity: the shared edge really is twinned.
        assert!(edges.iter().any(|e| e.twin != INVALID_INDEX));
        let back = round_trip(&edges);
        assert_byte_identical(&edges, &back);
    }

    #[test]
    fn round_trip_tetrahedron_byte_identical() {
        let edges = tetrahedron();
        // Closed mesh: no boundary darts.
        assert!(
            edges.iter().all(|e| e.twin != INVALID_INDEX),
            "tetrahedron should be closed"
        );
        let back = round_trip(&edges);
        assert_byte_identical(&edges, &back);
    }

    #[test]
    fn round_trip_boundary_mesh_byte_identical() {
        let edges = boundary_mesh();
        // Open mesh: at least one boundary dart.
        assert!(edges.iter().any(|e| e.twin == INVALID_INDEX));
        let back = round_trip(&edges);
        assert_byte_identical(&edges, &back);
    }

    // ======================================================================
    // 5. Validate accepts the golden meshes
    // ======================================================================

    #[test]
    fn validate_accepts_single_triangle() {
        let edges = single_triangle();
        let mut darts = vec![Dart::default(); edges.len()];
        half_edges_to_darts(&edges, &mut darts).unwrap();
        validate_combinatorial_map(&darts).unwrap();
    }

    #[test]
    fn validate_accepts_two_triangles_shared_edge() {
        let edges = two_triangles_shared_edge();
        let mut darts = vec![Dart::default(); edges.len()];
        half_edges_to_darts(&edges, &mut darts).unwrap();
        validate_combinatorial_map(&darts).unwrap();
    }

    #[test]
    fn validate_accepts_tetrahedron() {
        let edges = tetrahedron();
        let mut darts = vec![Dart::default(); edges.len()];
        half_edges_to_darts(&edges, &mut darts).unwrap();
        validate_combinatorial_map(&darts).unwrap();
    }

    #[test]
    fn validate_accepts_boundary_mesh() {
        let edges = boundary_mesh();
        let mut darts = vec![Dart::default(); edges.len()];
        half_edges_to_darts(&edges, &mut darts).unwrap();
        validate_combinatorial_map(&darts).unwrap();
    }

    #[test]
    fn validate_accepts_empty_dart_array() {
        validate_combinatorial_map(&[]).unwrap();
    }

    // ======================================================================
    // 6. Reject broken β₂ involution
    // ======================================================================

    #[test]
    fn reject_broken_beta2_involution() {
        let edges = two_triangles_shared_edge();
        let mut darts = vec![Dart::default(); edges.len()];
        half_edges_to_darts(&edges, &mut darts).unwrap();

        // Find an interior dart (one with a twin) and corrupt its twin partner
        // so that β₂(β₂(d)) != d.
        let d_idx = darts.iter().position(|d| d.beta2 != INVALID_INDEX).unwrap() as u32;
        let partner = darts[d_idx as usize].beta2;
        // Point the partner's beta2 somewhere else (still in range, but not d_idx).
        let wrong = if partner == 0 { 1 } else { 0 };
        darts[partner as usize].beta2 = wrong;

        let err = validate_combinatorial_map(&darts).unwrap_err();
        match err {
            CMapError::Beta2NotInvolution {
                dart,
                beta2,
                beta2_beta2,
            } => {
                // The validator scans from index 0, so it finds d_idx first
                // (whose beta2=partner, but partner's beta2=wrong ≠ d_idx).
                assert_eq!(dart, d_idx);
                assert_eq!(beta2, partner);
                assert_eq!(beta2_beta2, wrong);
            }
            other => panic!("expected Beta2NotInvolution, got {other:?}"),
        }
    }

    // ======================================================================
    // 7. Reject broken β₁ permutation
    // ======================================================================

    #[test]
    fn reject_beta1_dangling_invalid_index() {
        let edges = single_triangle();
        let mut darts = vec![Dart::default(); edges.len()];
        half_edges_to_darts(&edges, &mut darts).unwrap();

        // Corrupt one beta1 to INVALID_INDEX mid-cycle.
        darts[1].beta1 = INVALID_INDEX;
        let err = validate_combinatorial_map(&darts).unwrap_err();
        assert!(
            matches!(err, CMapError::Beta1Dangling { dart, points_to }
                if dart == 1 && points_to == INVALID_INDEX),
            "expected Beta1Dangling, got {err:?}"
        );
    }

    #[test]
    fn reject_beta1_out_of_range() {
        let edges = single_triangle();
        let mut darts = vec![Dart::default(); edges.len()];
        half_edges_to_darts(&edges, &mut darts).unwrap();

        darts[0].beta1 = 999;
        let err = validate_combinatorial_map(&darts).unwrap_err();
        assert!(
            matches!(err, CMapError::DartOutOfRange { index } if index == 999),
            "expected DartOutOfRange, got {err:?}"
        );
    }

    #[test]
    fn reject_beta1_not_permutation() {
        let edges = single_triangle();
        let mut darts = vec![Dart::default(); edges.len()];
        half_edges_to_darts(&edges, &mut darts).unwrap();

        // Make a 2-cycle between dart 0 and dart 1, leaving dart 2 pointing at 0
        // — but 0 no longer points at 1's old target, so the cycle from dart 2
        // never returns. Easiest: 0→1, 1→0, 2→0. From dart 2: 2→0→1→0→1→... never
        // returns to 2 within len hops.
        darts[0].beta1 = 1;
        darts[1].beta1 = 0;
        darts[2].beta1 = 0;
        let err = validate_combinatorial_map(&darts).unwrap_err();
        assert!(
            matches!(err, CMapError::Beta1NotPermutation { dart } if dart == 2),
            "expected Beta1NotPermutation, got {err:?}"
        );
    }

    // ======================================================================
    // 8. Reject face inconsistency
    // ======================================================================

    #[test]
    fn reject_face_inconsistency() {
        let edges = two_triangles_shared_edge();
        let mut darts = vec![Dart::default(); edges.len()];
        half_edges_to_darts(&edges, &mut darts).unwrap();

        // Flip one dart's face to differ from its β₁-cycle.
        let original_face = darts[0].face;
        darts[0].face = if original_face == 0 { 1 } else { 0 };
        let err = validate_combinatorial_map(&darts).unwrap_err();
        assert!(
            matches!(err, CMapError::FaceInconsistent { dart, face_a, face_b }
                if dart == 0 && face_a == darts[0].face && face_b == original_face),
            "expected FaceInconsistent, got {err:?}"
        );
    }

    // ======================================================================
    // 9. Reject origin inconsistency
    // ======================================================================

    #[test]
    fn reject_origin_inconsistency() {
        let edges = two_triangles_shared_edge();
        let mut darts = vec![Dart::default(); edges.len()];
        half_edges_to_darts(&edges, &mut darts).unwrap();

        // Find an interior dart and change its origin so the connectivity
        // invariant origin(d) == origin(beta1(beta2(d))) breaks.
        let d_idx = darts.iter().position(|d| d.beta2 != INVALID_INDEX).unwrap();
        let twin = darts[d_idx].beta2;
        let after_twin = darts[twin as usize].beta1;
        let correct_origin = darts[after_twin as usize].origin;
        let wrong = if correct_origin == 0 { 7 } else { 0 };
        darts[d_idx].origin = wrong;

        let err = validate_combinatorial_map(&darts).unwrap_err();
        assert!(
            matches!(err, CMapError::OriginInconsistent { dart, expected, found }
                if dart == d_idx as u32 && expected == correct_origin && found == wrong),
            "expected OriginInconsistent, got {err:?}"
        );
    }

    // ======================================================================
    // 10. Buffer-too-small rejection
    // ======================================================================

    #[test]
    fn half_edges_to_darts_buffer_too_small() {
        let edges = single_triangle();
        let mut darts = [Dart::default(); 2]; // need 3
        let err = half_edges_to_darts(&edges, &mut darts).unwrap_err();
        assert_eq!(err, CMapError::DartBufferTooSmall { required: 3 });
    }

    #[test]
    fn darts_to_half_edges_buffer_too_small() {
        let edges = single_triangle();
        let mut darts = vec![Dart::default(); edges.len()];
        half_edges_to_darts(&edges, &mut darts).unwrap();
        let mut back = [HalfEdge::default(); 2]; // need 3
        let err = darts_to_half_edges(&darts, &mut back).unwrap_err();
        assert_eq!(err, CMapError::HalfEdgeBufferTooSmall { required: 3 });
    }

    #[test]
    fn conversion_exact_fit_succeeds() {
        let edges = single_triangle();
        let mut darts = [Dart::default(); 3]; // exact fit
        let n = half_edges_to_darts(&edges, &mut darts).unwrap();
        assert_eq!(n, 3);
        let mut back = [HalfEdge::default(); 3];
        let n2 = darts_to_half_edges(&darts, &mut back).unwrap();
        assert_eq!(n2, 3);
        assert_byte_identical(&edges, &back);
    }

    // ======================================================================
    // 11. Dart is a 16-byte POD
    // ======================================================================

    #[test]
    fn dart_is_16_byte_pod() {
        assert_eq!(std::mem::size_of::<Dart>(), 16);
        assert_eq!(std::mem::size_of::<Dart>(), std::mem::size_of::<HalfEdge>());
        // Pod/Zeroable are in scope via bytemuck; this cast exercises them.
        let d = Dart {
            beta1: 1,
            beta2: 2,
            face: 3,
            origin: 4,
        };
        let bytes: [u8; 16] = bytemuck::cast(d);
        let back: Dart = bytemuck::cast(bytes);
        assert_eq!(d, back);
    }

    #[test]
    fn dart_default_is_all_invalid() {
        let d = Dart::default();
        assert_eq!(d.beta1, INVALID_INDEX);
        assert_eq!(d.beta2, INVALID_INDEX);
        assert_eq!(d.face, INVALID_INDEX);
        assert_eq!(d.origin, INVALID_INDEX);
    }

    #[test]
    fn dart_field_offsets_match_half_edge_semantics() {
        // beta1 occupies the `next` slot, beta2 the `twin` slot, etc. The
        // conversion functions do field-by-field mapping (not byte cast)
        // because Dart and HalfEdge have different field orders.
        let he = HalfEdge {
            origin: 10,
            twin: 20,
            next: 30,
            face: 40,
        };
        let mut darts = [Dart::default()];
        half_edges_to_darts(&[he], &mut darts).unwrap();
        let d = &darts[0];
        assert_eq!(d.beta1, 30); // next
        assert_eq!(d.beta2, 20); // twin
        assert_eq!(d.face, 40);
        assert_eq!(d.origin, 10);
        let mut back = [HalfEdge::default()];
        darts_to_half_edges(&darts[..], &mut back).unwrap();
        assert_eq!(he, back[0]);
    }

    // ======================================================================
    // 12. Determinism — two round-trips from the same input are byte-identical
    // ======================================================================

    #[test]
    fn round_trip_is_deterministic() {
        let edges = tetrahedron();
        let back1 = round_trip(&edges);
        let back2 = round_trip(&edges);
        assert_byte_identical(&back1, &back2);
        assert_byte_identical(&back1, &edges);
    }

    #[test]
    fn half_edges_to_darts_is_deterministic() {
        let edges = two_triangles_shared_edge();
        let mut a = vec![Dart::default(); edges.len()];
        let mut b = vec![Dart::default(); edges.len()];
        half_edges_to_darts(&edges, &mut a).unwrap();
        half_edges_to_darts(&edges, &mut b).unwrap();
        assert_eq!(a, b);
    }

    // ======================================================================
    // Extra: β₂ out-of-range rejection + boundary β₂ stays invalid
    // ======================================================================

    #[test]
    fn reject_beta2_out_of_range() {
        let edges = single_triangle();
        let mut darts = vec![Dart::default(); edges.len()];
        half_edges_to_darts(&edges, &mut darts).unwrap();
        darts[0].beta2 = 100; // in-range check happens before involution
        let err = validate_combinatorial_map(&darts).unwrap_err();
        assert!(
            matches!(err, CMapError::DartOutOfRange { index } if index == 100),
            "expected DartOutOfRange, got {err:?}"
        );
    }

    #[test]
    fn boundary_darts_keep_invalid_beta2() {
        let edges = boundary_mesh();
        let mut darts = vec![Dart::default(); edges.len()];
        half_edges_to_darts(&edges, &mut darts).unwrap();
        // Every dart that was a boundary half-edge must have beta2 == INVALID_INDEX.
        for (he, d) in edges.iter().zip(darts.iter()) {
            assert_eq!(he.twin == INVALID_INDEX, d.beta2 == INVALID_INDEX);
        }
        // And the map still validates.
        validate_combinatorial_map(&darts).unwrap();
    }

    #[test]
    fn tetrahedron_has_no_boundary_darts() {
        let edges = tetrahedron();
        let mut darts = vec![Dart::default(); edges.len()];
        half_edges_to_darts(&edges, &mut darts).unwrap();
        assert!(darts.iter().all(|d| d.beta2 != INVALID_INDEX));
        validate_combinatorial_map(&darts).unwrap();
    }
}
