//! Polygon-soup ingestion + repair.
//!
//! Raw triangle input ("polygon soup") frequently contains duplicate vertices,
//! degenerate triangles, inconsistent winding, and non-manifold edges. This
//! module provides a deterministic, caller-buffered, zero-heap repair pipeline:
//!
//! 1. [`merge_vertices`] — collapse duplicate positions within a tolerance
//!    (first-occurrence-wins, byte-identical for identical input).
//! 2. [`filter_degenerate_faces`] — drop triangles whose indices collapse after
//!    remapping.
//! 3. [`orient_consistently`] — BFS from face 0 propagating winding across
//!    shared edges (deterministic: seed = face 0, BFS by face index).
//! 4. [`repair_polygon_soup`] — full pipeline that runs the three steps and then
//!    fail-closed validates the repaired index buffer through
//!    [`build_triangle_half_edges`](super::topology::build_triangle_half_edges).
//!
//! # Design notes
//!
//! - **Zero heap.** No `Vec`, `String`, or `Box` in any function. All workspace
//!   memory is caller-supplied (`&mut [T]`). Test helpers may use `vec!` for
//!   setup only.
//! - **Determinism.** Merge is first-occurrence-wins (linear scan, lowest
//!   surviving index keeps its position). Orientation BFS seeds at face 0 and
//!   visits neighbours in ascending face-index order. Identical input yields
//!   bit-identical output.
//! - **Fail-closed.** After repair, the index buffer is run through
//!   `build_triangle_half_edges`. A `NonManifoldEdge` outcome is surfaced as
//!   [`SoupError::NonManifoldRemainder`] — the repair never silently accepts
//!   non-manifold input (double fail-closed: repair rejects what it cannot fix,
//!   then the half-edge builder rejects what repair missed).
//!
//! The module is self-contained: it depends only on
//! [`super::topology`] types and `bytemuck`/`serde` derives already present in
//! the crate.

use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};

use super::topology::{
    build_triangle_half_edges, required_edge_slots, EdgeSlot, HalfEdge, TopologyError,
    INVALID_INDEX,
};

// ---------------------------------------------------------------------------
// Errors and report types
// ---------------------------------------------------------------------------

/// Errors raised by the polygon-soup repair pipeline. All variants carry the
/// information needed for a caller to grow buffers or report the offending
/// input; none allocate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoupError {
    /// `positions` did not contain `3 * vertex_count` f64s.
    PositionBufferTooSmall { required: usize },
    /// `repaired_triangles` (or an intermediate triangle buffer) was too small.
    IndexBufferTooSmall { required: usize },
    /// `remap` was shorter than `vertex_count`.
    RemapBufferTooSmall { required: usize },
    /// A triangle referenced a vertex index >= `vertex_count`.
    VertexOutOfRange { face: usize, vertex: u32 },
    /// A workspace buffer (adjacency / BFS queue / half-edge) was too small.
    WorkspaceTooSmall { required: usize },
    /// The repaired mesh still contains a non-manifold edge. The repair
    /// pipeline refuses to silently accept this; the offending directed edge
    /// is reported so the caller can surface the remainder.
    NonManifoldRemainder { from: u32, to: u32 },
    /// `build_triangle_half_edges` rejected the repaired mesh for a reason
    /// other than non-manifold edges (e.g. a duplicate directed edge survived
    /// orientation propagation, which indicates an internal invariant break).
    TopologyBuildFailed(TopologyError),
}

/// Summary of a completed repair pass. All counts are post-repair.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepairReport {
    /// Number of unique vertices after merging duplicates.
    pub merged_vertex_count: u32,
    /// Number of triangles surviving degenerate filtering.
    pub surviving_triangle_count: usize,
    /// Number of faces whose winding was flipped during orientation
    /// propagation.
    pub flipped_face_count: usize,
    /// Number of connected components touched by the orientation BFS. Faces
    /// not reachable from the seed (face 0) form additional components; each
    /// component is oriented independently with its lowest-index face as the
    /// local seed.
    pub oriented_component_count: usize,
    /// Half-edge topology summary of the repaired mesh, when the caller
    /// supplied half-edge workspace. `None` when the pipeline was run without
    /// the half-edge validation pass.
    pub topology: Option<super::topology::TopologySummary>,
}

// ---------------------------------------------------------------------------
// POD workspace records
// ---------------------------------------------------------------------------

/// One entry in the face-adjacency workspace. Records, for a directed edge of
/// a face, the neighbouring face that shares the opposite-directed edge (or
/// [`INVALID_INDEX`] when the edge is a boundary).
///
/// Stored as a flat POD array of length `3 * face_count` so the caller can
/// allocate it with a stack array or a fixed buffer.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Pod, Zeroable, Serialize, Deserialize)]
pub struct FaceAdjacencyEntry {
    /// Directed edge endpoint: `from` vertex (post-merge index).
    pub from: u32,
    /// Directed edge endpoint: `to` vertex (post-merge index).
    pub to: u32,
    /// Face index that owns the opposite-directed edge, or `INVALID_INDEX`.
    pub neighbour_face: u32,
}

impl Default for FaceAdjacencyEntry {
    fn default() -> Self {
        Self {
            from: INVALID_INDEX,
            to: INVALID_INDEX,
            neighbour_face: INVALID_INDEX,
        }
    }
}

// ---------------------------------------------------------------------------
// Step 1 — merge duplicate vertices
// ---------------------------------------------------------------------------

/// Merge duplicate vertices within `tolerance`.
///
/// `positions` is a flat `f64` array of length `3 * vertex_count`. `remap`
/// receives the old-index → merged-index mapping (length `vertex_count`).
/// `merged_positions` receives the compacted positions (length `3 *
/// merged_count`); only the first `3 * merged_count` entries are written.
///
/// Strategy is **first-occurrence-wins**: the first time a position is seen it
/// becomes a merged vertex and keeps its original index slot; every later
/// position within `tolerance` (compared component-wise, i.e. Chebyshev /
/// L-infinity distance) maps back to that first index. This is deterministic
/// and produces byte-identical output for byte-identical input.
///
/// Returns the merged vertex count.
pub fn merge_vertices(
    positions: &[f64],
    vertex_count: u32,
    tolerance: f64,
    remap: &mut [u32],
    merged_positions: &mut [f64],
) -> Result<u32, SoupError> {
    let needed =
        (vertex_count as usize)
            .checked_mul(3)
            .ok_or(SoupError::PositionBufferTooSmall {
                required: usize::MAX,
            })?;
    if positions.len() < needed {
        return Err(SoupError::PositionBufferTooSmall { required: needed });
    }
    if remap.len() < vertex_count as usize {
        return Err(SoupError::RemapBufferTooSmall {
            required: vertex_count as usize,
        });
    }
    if merged_positions.len() < needed {
        return Err(SoupError::PositionBufferTooSmall { required: needed });
    }

    let tol = tolerance.abs();
    let mut merged_count: u32 = 0;

    for old in 0..vertex_count as usize {
        let base = old * 3;
        let px = positions[base];
        let py = positions[base + 1];
        let pz = positions[base + 2];

        // Linear scan over already-merged vertices (first-occurrence-wins).
        let mut found: Option<u32> = None;
        for cand in 0..merged_count as usize {
            let c = cand * 3;
            let dx = (positions[c] - px).abs();
            if dx > tol {
                continue;
            }
            let dy = (positions[c + 1] - py).abs();
            if dy > tol {
                continue;
            }
            let dz = (positions[c + 2] - pz).abs();
            if dz > tol {
                continue;
            }
            found = Some(cand as u32);
            break;
        }

        let target = match found {
            Some(idx) => idx,
            None => {
                let idx = merged_count;
                let dst = idx as usize * 3;
                merged_positions[dst] = px;
                merged_positions[dst + 1] = py;
                merged_positions[dst + 2] = pz;
                merged_count += 1;
                idx
            }
        };
        remap[old] = target;
    }

    Ok(merged_count)
}

// ---------------------------------------------------------------------------
// Step 2 — filter degenerate triangles
// ---------------------------------------------------------------------------

/// Apply `remap` to each triangle and copy the survivors into `out`.
///
/// A triangle is dropped when any two of its remapped indices are equal (i.e.
/// the triangle collapsed to a segment or point after vertex merging). The
/// surviving triangles are written compactly to the front of `out`; their
/// indices are the **merged** vertex indices.
///
/// Returns the number of surviving triangles. Returns
/// [`SoupError::IndexBufferTooSmall`] when `out` cannot hold every input
/// triangle (the caller is expected to size `out >= triangles.len()`).
pub fn filter_degenerate_faces(
    triangles: &[[u32; 3]],
    remap: &[u32],
    out: &mut [[u32; 3]],
) -> Result<usize, SoupError> {
    if out.len() < triangles.len() {
        return Err(SoupError::IndexBufferTooSmall {
            required: triangles.len(),
        });
    }

    let mut kept = 0usize;
    for triangle in triangles.iter().copied() {
        let a = remap
            .get(triangle[0] as usize)
            .copied()
            .ok_or(SoupError::VertexOutOfRange {
                face: kept,
                vertex: triangle[0],
            })?;
        let b = remap
            .get(triangle[1] as usize)
            .copied()
            .ok_or(SoupError::VertexOutOfRange {
                face: kept,
                vertex: triangle[1],
            })?;
        let c = remap
            .get(triangle[2] as usize)
            .copied()
            .ok_or(SoupError::VertexOutOfRange {
                face: kept,
                vertex: triangle[2],
            })?;
        if a == b || b == c || a == c {
            continue;
        }
        out[kept] = [a, b, c];
        kept += 1;
    }
    Ok(kept)
}

// ---------------------------------------------------------------------------
// Step 3 — orient consistently
// ---------------------------------------------------------------------------

/// Build the face-adjacency workspace used by [`orient_consistently`].
///
/// `adjacency` must have length `3 * face_count`. For each directed edge of
/// each face we record its endpoints and the face index (if any) that owns the
/// opposite-directed edge. Boundary edges get `neighbour_face = INVALID_INDEX`.
///
/// The lookup is O(faces² · edges) and uses no heap; it is the simplest
/// deterministic construction and is adequate for the soup sizes the repair
/// pipeline targets. For very large soups a caller may pre-build the workspace
/// with a hash table and pass it directly to [`orient_consistently`].
pub fn build_face_adjacency(
    triangles: &[[u32; 3]],
    adjacency: &mut [FaceAdjacencyEntry],
) -> Result<(), SoupError> {
    let needed = triangles
        .len()
        .checked_mul(3)
        .ok_or(SoupError::WorkspaceTooSmall {
            required: usize::MAX,
        })?;
    if adjacency.len() < needed {
        return Err(SoupError::WorkspaceTooSmall { required: needed });
    }
    for entry in adjacency.iter_mut() {
        *entry = FaceAdjacencyEntry::default();
    }

    for (face, tri) in triangles.iter().copied().enumerate() {
        let base = face * 3;
        for local in 0..3 {
            let from = tri[local];
            let to = tri[(local + 1) % 3];
            adjacency[base + local] = FaceAdjacencyEntry {
                from,
                to,
                neighbour_face: INVALID_INDEX,
            };
        }
    }

    // Pair opposite-directed edges. O(faces² · 9) but deterministic and
    // allocation-free. We only record the *first* matching neighbour; a
    // second match would indicate a non-manifold edge, which the later
    // half-edge build catches fail-closed.
    let face_count = triangles.len();
    for face in 0..face_count {
        let base = face * 3;
        for local in 0..3 {
            let from = adjacency[base + local].from;
            let to = adjacency[base + local].to;
            // Search all other faces for the opposite-directed edge.
            for other in 0..face_count {
                if other == face {
                    continue;
                }
                let obase = other * 3;
                for olocal in 0..3 {
                    let ofrom = adjacency[obase + olocal].from;
                    let oto = adjacency[obase + olocal].to;
                    if (ofrom == to && oto == from) || (ofrom == from && oto == to) {
                        adjacency[base + local].neighbour_face = other as u32;
                        break;
                    }
                }
                if adjacency[base + local].neighbour_face != INVALID_INDEX {
                    break;
                }
            }
        }
    }
    Ok(())
}

/// Flip a triangle in place by swapping its second and third indices.
#[inline]
fn flip_triangle(tri: &mut [u32; 3]) {
    tri.swap(1, 2);
}

/// Orient faces consistently via BFS from face 0.
///
/// `triangles` is mutated in place: faces whose winding disagrees with the
/// seed are flipped. `adjacency` is the workspace produced by
/// [`build_face_adjacency`] (length `3 * face_count`). `visited` is a
/// caller-supplied scratch buffer of length `face_count` (cleared by this
/// function). `queue` is a caller-supplied scratch buffer of length
/// `face_count` used as the BFS frontier (cleared by this function).
///
/// Determinism: the seed is always face 0. When the mesh has multiple
/// connected components, each component is seeded at its lowest-index
/// unvisited face and oriented independently. BFS visits neighbours in
/// ascending face-index order, so identical input yields bit-identical output.
///
/// Returns the number of connected components oriented.
pub fn orient_consistently(
    triangles: &mut [[u32; 3]],
    adjacency: &mut [FaceAdjacencyEntry],
    visited: &mut [bool],
    queue: &mut [u32],
) -> Result<usize, SoupError> {
    let face_count = triangles.len();
    let needed = face_count
        .checked_mul(3)
        .ok_or(SoupError::WorkspaceTooSmall {
            required: usize::MAX,
        })?;
    if adjacency.len() < needed {
        return Err(SoupError::WorkspaceTooSmall { required: needed });
    }
    if visited.len() < face_count || queue.len() < face_count {
        return Err(SoupError::WorkspaceTooSmall {
            required: face_count,
        });
    }

    for v in visited.iter_mut() {
        *v = false;
    }

    let mut component_count = 0usize;
    let mut seed = 0u32;
    while (seed as usize) < face_count {
        // BFS from the lowest unvisited face.
        let mut head = 0usize;
        let mut tail = 0usize;
        queue[tail] = seed;
        tail += 1;
        visited[seed as usize] = true;
        component_count += 1;

        while head < tail {
            let face = queue[head];
            head += 1;
            let base = face as usize * 3;
            for local in 0..3 {
                let entry = adjacency[base + local];
                let neighbour = entry.neighbour_face;
                if neighbour == INVALID_INDEX {
                    continue;
                }
                let n = neighbour as usize;
                if visited[n] {
                    continue;
                }
                // Find the neighbour's edge that is opposite to ours.
                let nbase = n * 3;
                let mut matched = false;
                for olocal in 0..3 {
                    let oentry = adjacency[nbase + olocal];
                    if oentry.from == entry.to && oentry.to == entry.from {
                        // The two faces agree on this edge iff one owns
                        // (from->to) and the other owns (to->from), i.e. the
                        // directed edges are opposite. If instead the
                        // neighbour owns the *same* direction (from->to) we
                        // must flip it to agree.
                        // Here oentry is (to, from) — opposite — so winding
                        // already agrees. No flip needed.
                        let _ = olocal;
                        matched = true;
                        break;
                    }
                    if oentry.from == entry.from && oentry.to == entry.to {
                        // Same directed edge: the neighbour disagrees. Flip it
                        // and rebuild its adjacency row in place so later
                        // traversals see the corrected winding.
                        flip_triangle(&mut triangles[n]);
                        // Rebuild the neighbour's three adjacency entries from
                        // the flipped triangle. The `from`/`to` swap; the
                        // neighbour links stay valid because we only rewire
                        // endpoints, not partner faces.
                        let flipped = triangles[n];
                        for k in 0..3 {
                            adjacency[nbase + k].from = flipped[k];
                            adjacency[nbase + k].to = flipped[(k + 1) % 3];
                        }
                        matched = true;
                        break;
                    }
                }
                let _ = matched;
                visited[n] = true;
                queue[tail] = neighbour;
                tail += 1;
            }
        }

        // Advance seed to the next unvisited face (lowest index first).
        while (seed as usize) < face_count && visited[seed as usize] {
            seed += 1;
        }
    }

    Ok(component_count)
}

/// Count how many faces differ from a reference triangle array (used by tests
/// and by [`repair_polygon_soup`] to populate `flipped_face_count`).
pub fn count_flipped(original: &[[u32; 3]], current: &[[u32; 3]]) -> usize {
    original
        .iter()
        .zip(current.iter())
        .filter(|(o, c)| o != c)
        .count()
}

// ---------------------------------------------------------------------------
// Step 4 — full pipeline
// ---------------------------------------------------------------------------

/// Run the full polygon-soup repair pipeline.
///
/// Buffers (all caller-supplied, zero heap):
/// - `remap`: length `vertex_count` (old → merged index).
/// - `merged_positions`: length `3 * vertex_count` (worst case: no merges).
/// - `repaired_triangles`: length `triangles.len()` (survivors written to
///   front).
/// - `original_triangles`: length `triangles.len()` workspace used to detect
///   flips and to keep a pre-orientation copy.
/// - `adjacency`: length `3 * triangles.len()` workspace.
/// - `visited`: length `triangles.len()` workspace.
/// - `queue`: length `triangles.len()` workspace.
/// - `half_edges`: length `3 * triangles.len()` workspace for the fail-closed
///   half-edge validation pass. May be empty if the caller does not want the
///   topology summary (in which case `report.topology` is `None`).
/// - `edge_slots`: length `required_edge_slots(triangles.len())` workspace for
///   the half-edge build. May be empty when `half_edges` is empty.
///
/// The pipeline:
/// 1. Merge duplicate vertices.
/// 2. Filter degenerate triangles.
/// 3. Orient consistently (BFS from face 0).
/// 4. Fail-closed: run `build_triangle_half_edges` on the repaired index
///    buffer. `NonManifoldEdge` is surfaced as
///    [`SoupError::NonManifoldRemainder`]; other topology errors as
///    [`SoupError::TopologyBuildFailed`].
///
/// On success, `repaired_triangles[..report.surviving_triangle_count]` holds
/// the repaired, consistently-oriented, manifold-valid triangle indices
/// (merged vertex space), and `merged_positions[..3 *
/// report.merged_vertex_count]` holds the compacted positions.
pub fn repair_polygon_soup(
    positions: &[f64],
    vertex_count: u32,
    triangles: &[[u32; 3]],
    tolerance: f64,
    remap: &mut [u32],
    merged_positions: &mut [f64],
    repaired_triangles: &mut [[u32; 3]],
    original_triangles: &mut [[u32; 3]],
    adjacency: &mut [FaceAdjacencyEntry],
    visited: &mut [bool],
    queue: &mut [u32],
    half_edges: &mut [HalfEdge],
    edge_slots: &mut [EdgeSlot],
) -> Result<RepairReport, SoupError> {
    if repaired_triangles.len() < triangles.len() || original_triangles.len() < triangles.len() {
        return Err(SoupError::IndexBufferTooSmall {
            required: triangles.len(),
        });
    }
    let adjacency_needed = triangles
        .len()
        .checked_mul(3)
        .ok_or(SoupError::WorkspaceTooSmall {
            required: usize::MAX,
        })?;
    if adjacency.len() < adjacency_needed {
        return Err(SoupError::WorkspaceTooSmall {
            required: adjacency_needed,
        });
    }
    if visited.len() < triangles.len() || queue.len() < triangles.len() {
        return Err(SoupError::WorkspaceTooSmall {
            required: triangles.len(),
        });
    }

    // Step 1 — merge vertices.
    let merged_count = merge_vertices(positions, vertex_count, tolerance, remap, merged_positions)?;

    // Step 2 — filter degenerate faces into repaired_triangles.
    let surviving = filter_degenerate_faces(triangles, remap, repaired_triangles)?;

    // Snapshot the filtered (pre-orientation) triangles so we can count flips.
    original_triangles[..surviving].copy_from_slice(&repaired_triangles[..surviving]);

    // Step 3 — orient consistently. Work on the surviving slice only.
    let surviving_slice = &mut repaired_triangles[..surviving];
    let adjacency_slice = &mut adjacency[..surviving * 3];
    let visited_slice = &mut visited[..surviving];
    let queue_slice = &mut queue[..surviving];
    build_face_adjacency(surviving_slice, adjacency_slice)?;
    let components =
        orient_consistently(surviving_slice, adjacency_slice, visited_slice, queue_slice)?;
    let flipped = count_flipped(&original_triangles[..surviving], surviving_slice);

    // Step 4 — fail-closed half-edge validation.
    let topology = if (half_edges.is_empty() && edge_slots.is_empty()) || surviving == 0 {
        None
    } else {
        let edge_count = surviving * 3;
        if half_edges.len() < edge_count {
            return Err(SoupError::WorkspaceTooSmall {
                required: edge_count,
            });
        }
        let required_slots = required_edge_slots(surviving);
        if edge_slots.len() < required_slots || !edge_slots.len().is_power_of_two() {
            return Err(SoupError::WorkspaceTooSmall {
                required: required_slots,
            });
        }
        let summary = build_triangle_half_edges(
            merged_count,
            &repaired_triangles[..surviving],
            &mut half_edges[..edge_count],
            edge_slots,
        )
        .map_err(|err| match err {
            TopologyError::NonManifoldEdge { from, to } => {
                SoupError::NonManifoldRemainder { from, to }
            }
            other => SoupError::TopologyBuildFailed(other),
        })?;
        Some(summary)
    };

    Ok(RepairReport {
        merged_vertex_count: merged_count,
        surviving_triangle_count: surviving,
        flipped_face_count: flipped,
        oriented_component_count: components,
        topology,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::specialized_libs::computational_geometry::required_edge_slots;

    // -- merge_vertices ------------------------------------------------------

    #[test]
    fn merge_collapses_duplicates_within_tolerance() {
        // Two pairs of duplicates + one unique.
        let positions: Vec<f64> = vec![
            0.0, 0.0, 0.0, // 0
            1.0, 0.0, 0.0, // 1
            0.0, 0.0, 1e-7, // 2 — within tol of 0
            1.0, 0.0, 1e-7, // 3 — within tol of 1
            5.0, 5.0, 5.0, // 4 — unique
        ];
        let mut remap = [0u32; 5];
        let mut merged = [0.0f64; 15];
        let count = merge_vertices(&positions, 5, 1e-6, &mut remap, &mut merged).unwrap();
        assert_eq!(count, 3);
        assert_eq!(remap, [0, 1, 0, 1, 2]);
        assert_eq!(&merged[..9], &[0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 5.0, 5.0, 5.0]);
    }

    #[test]
    fn merge_keeps_separate_vertices_outside_tolerance() {
        let positions: Vec<f64> = vec![0.0, 0.0, 0.0, 0.0, 0.0, 1e-3, 0.0, 0.0, 2e-3];
        let mut remap = [0u32; 3];
        let mut merged = [0.0f64; 9];
        let count = merge_vertices(&positions, 3, 1e-6, &mut remap, &mut merged).unwrap();
        assert_eq!(count, 3);
        assert_eq!(remap, [0, 1, 2]);
    }

    #[test]
    fn merge_is_deterministic_across_runs() {
        let positions: Vec<f64> = vec![
            0.1,
            0.2,
            0.3,
            0.1,
            0.2,
            0.3,
            0.4,
            0.5,
            0.6,
            0.1,
            0.2,
            0.300_000_1,
        ];
        let mut remap_a = [0u32; 4];
        let mut merged_a = [0.0f64; 12];
        let mut remap_b = [0u32; 4];
        let mut merged_b = [0.0f64; 12];
        let a = merge_vertices(&positions, 4, 1e-6, &mut remap_a, &mut merged_a).unwrap();
        let b = merge_vertices(&positions, 4, 1e-6, &mut remap_b, &mut merged_b).unwrap();
        assert_eq!(a, b);
        assert_eq!(remap_a, remap_b);
        assert_eq!(merged_a, merged_b);
    }

    #[test]
    fn merge_detects_bad_buffers() {
        let positions = [0.0; 6];
        let mut remap = [0u32; 1];
        let mut merged = [0.0f64; 6];
        // remap too small for 2 vertices.
        let err = merge_vertices(&positions, 2, 1e-6, &mut remap, &mut merged).unwrap_err();
        assert_eq!(err, SoupError::RemapBufferTooSmall { required: 2 });
    }

    // -- filter_degenerate_faces --------------------------------------------

    #[test]
    fn filter_drops_degenerate_triangles() {
        let triangles = [[0, 1, 2], [0, 0, 1], [2, 1, 0], [3, 3, 3]];
        // remap: identity
        let remap = [0u32, 1, 2, 3];
        let mut out = [[0u32; 3]; 4];
        let kept = filter_degenerate_faces(&triangles, &remap, &mut out).unwrap();
        assert_eq!(kept, 2);
        assert_eq!(out[..2], [[0, 1, 2], [2, 1, 0]]);
    }

    #[test]
    fn filter_applies_remap_before_checking() {
        // Triangles that become degenerate only after merge.
        let triangles = [[0, 1, 2], [0, 3, 2]];
        let remap = [0u32, 0, 2, 0]; // 1 and 3 both merge into 0
        let mut out = [[0u32; 3]; 2];
        let kept = filter_degenerate_faces(&triangles, &remap, &mut out).unwrap();
        assert_eq!(kept, 0);
    }

    #[test]
    fn filter_rejects_short_output_buffer() {
        let triangles = [[0, 1, 2], [3, 4, 5]];
        let remap = [0u32, 1, 2, 3, 4, 5];
        let mut out = [[0u32; 3]; 1];
        let err = filter_degenerate_faces(&triangles, &remap, &mut out).unwrap_err();
        assert_eq!(err, SoupError::IndexBufferTooSmall { required: 2 });
    }

    // -- orient_consistently -------------------------------------------------

    #[test]
    fn orient_flips_inconsistent_neighbour() {
        // Two triangles sharing edge (1,2). Face 0 is (0,1,2); face 1 is
        // (1,2,3) — same directed edge (1->2) appears in both, so they
        // disagree and face 1 must be flipped to (1,3,2).
        let mut triangles = [[0, 1, 2], [1, 2, 3]];
        let mut adjacency = [FaceAdjacencyEntry::default(); 6];
        build_face_adjacency(&triangles, &mut adjacency).unwrap();
        let mut visited = [false; 2];
        let mut queue = [0u32; 2];
        let components =
            orient_consistently(&mut triangles, &mut adjacency, &mut visited, &mut queue).unwrap();
        assert_eq!(components, 1);
        assert_eq!(triangles, [[0, 1, 2], [1, 3, 2]]);
    }

    #[test]
    fn orient_leaves_consistent_mesh_unchanged() {
        // Two triangles already consistently wound (opposite directed edges on
        // the shared edge).
        let mut triangles = [[0, 1, 2], [2, 1, 3]];
        let mut adjacency = [FaceAdjacencyEntry::default(); 6];
        build_face_adjacency(&triangles, &mut adjacency).unwrap();
        let mut visited = [false; 2];
        let mut queue = [0u32; 2];
        orient_consistently(&mut triangles, &mut adjacency, &mut visited, &mut queue).unwrap();
        assert_eq!(triangles, [[0, 1, 2], [2, 1, 3]]);
    }

    #[test]
    fn orient_tetrahedron_full_propagation() {
        // Tetrahedron: 4 vertices, 4 faces. Already consistently wound:
        // face 0's 0→1 is opposite face 2's 1→0; face 0's 1→2 is opposite
        // face 3's 2→1; face 0's 2→0 is opposite face 1's 0→2.
        let mut triangles = [
            [0, 1, 2], // face 0 — seed
            [0, 2, 3], // shares edge (0,2) with face 0's (2,0): opposite -> ok
            [0, 3, 1], // shares edge (0,1) with face 0's (0,1) via 1→0: opposite -> ok
            [1, 3, 2], // shares edge (1,2) with face 0's (1,2) via 2→1: opposite -> ok
        ];
        let mut adjacency = [FaceAdjacencyEntry::default(); 12];
        build_face_adjacency(&triangles, &mut adjacency).unwrap();
        let mut visited = [false; 4];
        let mut queue = [0u32; 4];
        let components =
            orient_consistently(&mut triangles, &mut adjacency, &mut visited, &mut queue).unwrap();
        assert_eq!(components, 1);
        // No flips needed — already consistent.
        assert_eq!(triangles[0], [0, 1, 2]);
        assert_eq!(triangles[1], [0, 2, 3]);
        assert_eq!(triangles[2], [0, 3, 1]);
        assert_eq!(triangles[3], [1, 3, 2]);
    }

    #[test]
    fn orient_is_deterministic() {
        let mut a = [[0, 1, 2], [1, 2, 3], [2, 3, 0], [3, 0, 1]];
        let mut b = a;
        let mut adj_a = [FaceAdjacencyEntry::default(); 12];
        let mut adj_b = [FaceAdjacencyEntry::default(); 12];
        build_face_adjacency(&a, &mut adj_a).unwrap();
        build_face_adjacency(&b, &mut adj_b).unwrap();
        let mut vis_a = [false; 4];
        let mut vis_b = [false; 4];
        let mut q_a = [0u32; 4];
        let mut q_b = [0u32; 4];
        orient_consistently(&mut a, &mut adj_a, &mut vis_a, &mut q_a).unwrap();
        orient_consistently(&mut b, &mut adj_b, &mut vis_b, &mut q_b).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn orient_handles_disconnected_components() {
        // Two disjoint triangles. Each is its own component; neither has a
        // shared edge so no flips happen.
        let mut triangles = [[0, 1, 2], [3, 4, 5]];
        let mut adjacency = [FaceAdjacencyEntry::default(); 6];
        build_face_adjacency(&triangles, &mut adjacency).unwrap();
        let mut visited = [false; 2];
        let mut queue = [0u32; 2];
        let components =
            orient_consistently(&mut triangles, &mut adjacency, &mut visited, &mut queue).unwrap();
        assert_eq!(components, 2);
        assert_eq!(triangles, [[0, 1, 2], [3, 4, 5]]);
    }

    // -- full pipeline -------------------------------------------------------

    #[allow(dead_code)]
    fn run_pipeline(
        positions: &[f64],
        vertex_count: u32,
        triangles: &[[u32; 3]],
        tolerance: f64,
    ) -> Result<RepairReport, SoupError> {
        let n = triangles.len();
        let mut remap = vec![0u32; vertex_count as usize];
        let mut merged_positions = vec![0.0f64; (vertex_count as usize) * 3];
        let mut repaired = vec![[0u32; 3]; n];
        let mut original = vec![[0u32; 3]; n];
        let mut adjacency = vec![FaceAdjacencyEntry::default(); n * 3];
        let mut visited = vec![false; n];
        let mut queue = vec![0u32; n];
        let mut half_edges = vec![HalfEdge::default(); n * 3];
        let slots = required_edge_slots(n);
        let mut edge_slots = vec![EdgeSlot::default(); slots];
        repair_polygon_soup(
            positions,
            vertex_count,
            triangles,
            tolerance,
            &mut remap,
            &mut merged_positions,
            &mut repaired,
            &mut original,
            &mut adjacency,
            &mut visited,
            &mut queue,
            &mut half_edges,
            &mut edge_slots,
        )?;
        // Stash repaired triangles on the report via a side channel for the
        // caller — here we just re-run to inspect via a thread-local. For the
        // test we instead return the report and let the caller re-derive.
        Ok(report_only(repaired, merged_positions, n, vertex_count))
    }

    // Helper that re-runs the pipeline and returns the repaired buffer too, so
    // individual tests can inspect the index buffer. Uses thread-local storage
    // to avoid changing the public API.
    thread_local! {
        static LAST_REPAIRED: std::cell::RefCell<Vec<[u32;3]>> = std::cell::RefCell::new(Vec::new());
        static LAST_MERGED: std::cell::RefCell<Vec<f64>> = std::cell::RefCell::new(Vec::new());
    }

    #[allow(dead_code)]
    fn report_only(
        repaired: Vec<[u32; 3]>,
        merged_positions: Vec<f64>,
        n: usize,
        _vc: u32,
    ) -> RepairReport {
        LAST_REPAIRED.with(|c| *c.borrow_mut() = repaired);
        LAST_MERGED.with(|c| *c.borrow_mut() = merged_positions);
        // The real report is computed inside repair_polygon_soup; this helper
        // is only used by the test harness which re-derives the fields it
        // needs. We return a placeholder that tests overwrite.
        let _ = n;
        RepairReport {
            merged_vertex_count: 0,
            surviving_triangle_count: 0,
            flipped_face_count: 0,
            oriented_component_count: 0,
            topology: None,
        }
    }

    // The above helper approach is awkward; replace with a direct in-test
    // driver that returns the real report plus buffers.
    #[derive(Debug)]
    struct PipelineOutput {
        report: RepairReport,
        repaired: Vec<[u32; 3]>,
        merged_positions: Vec<f64>,
        remap: Vec<u32>,
    }

    fn run_pipeline_full(
        positions: &[f64],
        vertex_count: u32,
        triangles: &[[u32; 3]],
        tolerance: f64,
    ) -> Result<PipelineOutput, SoupError> {
        let n = triangles.len();
        let mut remap = vec![0u32; vertex_count as usize];
        let mut merged_positions = vec![0.0f64; (vertex_count as usize) * 3];
        let mut repaired = vec![[0u32; 3]; n];
        let mut original = vec![[0u32; 3]; n];
        let mut adjacency = vec![FaceAdjacencyEntry::default(); n * 3];
        let mut visited = vec![false; n];
        let mut queue = vec![0u32; n];
        let mut half_edges = vec![HalfEdge::default(); n * 3];
        let slots = required_edge_slots(n);
        let mut edge_slots = vec![EdgeSlot::default(); slots];
        let report = repair_polygon_soup(
            positions,
            vertex_count,
            triangles,
            tolerance,
            &mut remap,
            &mut merged_positions,
            &mut repaired,
            &mut original,
            &mut adjacency,
            &mut visited,
            &mut queue,
            &mut half_edges,
            &mut edge_slots,
        )?;
        Ok(PipelineOutput {
            report,
            repaired,
            merged_positions,
            remap,
        })
    }

    #[test]
    fn pipeline_repairs_duplicates_degenerates_and_orientation() {
        // Build a soup: two triangles forming a square, with a duplicate
        // vertex and a degenerate triangle, plus inconsistent winding.
        // Vertices (with a duplicate of vertex 1 as index 4):
        //   0: (0,0,0)
        //   1: (1,0,0)
        //   2: (0,1,0)
        //   3: (1,1,0)
        //   4: (1,0,0)  <- duplicate of 1
        let positions: Vec<f64> = vec![
            0.0, 0.0, 0.0, // 0
            1.0, 0.0, 0.0, // 1
            0.0, 1.0, 0.0, // 2
            1.0, 1.0, 0.0, // 3
            1.0, 0.0, 0.0, // 4 (dup of 1)
        ];
        let triangles = [
            [0, 1, 2], // good
            [2, 4, 3], // good after merge (4->1): winding disagrees with face 0 on shared edge (0,1,2)/(2,1,3)? check below
            [0, 0, 2], // degenerate
        ];
        let out = run_pipeline_full(&positions, 5, &triangles, 1e-6).unwrap();
        // Merged vertices: 0,1,2,3 -> 4 unique (vertex 4 merges into 1).
        assert_eq!(out.report.merged_vertex_count, 4);
        // One degenerate dropped -> 2 survivors.
        assert_eq!(out.report.surviving_triangle_count, 2);
        // Repaired indices use merged space.
        let repaired = &out.repaired[..2];
        // Face 0 unchanged: [0,1,2].
        assert_eq!(repaired[0], [0, 1, 2]);
        // Face 1 originally [2,4,3] -> remapped [2,1,3]. Shared edge with
        // face 0 is (1,2) vs (2,1): opposite directed edges -> already
        // consistent, no flip.
        assert_eq!(repaired[1], [2, 1, 3]);
        // Half-edge build succeeded (topology present).
        assert!(out.report.topology.is_some());
        let topo = out.report.topology.unwrap();
        assert_eq!(topo.face_count, 2);
        assert_eq!(topo.boundary_half_edges, 4); // 6 edges - 2 twinned = 4 boundary
    }

    #[test]
    fn pipeline_is_bit_identical_across_runs() {
        let positions: Vec<f64> = vec![
            0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0, 0.0, 0.0,
        ];
        let triangles = [[0, 1, 2], [2, 4, 3], [0, 1, 0]];
        let a = run_pipeline_full(&positions, 5, &triangles, 1e-6).unwrap();
        let b = run_pipeline_full(&positions, 5, &triangles, 1e-6).unwrap();
        assert_eq!(a.report, b.report);
        assert_eq!(a.repaired, b.repaired);
        assert_eq!(a.merged_positions, b.merged_positions);
        assert_eq!(a.remap, b.remap);
        // Bit-level check on the index buffer.
        for (x, y) in a.repaired.iter().zip(b.repaired.iter()) {
            for (xi, yi) in x.iter().zip(y.iter()) {
                assert_eq!(*xi, *yi);
            }
        }
    }

    #[test]
    fn pipeline_fail_closed_on_nonmanifold_remainder() {
        // Three faces all sharing the directed edge (0,1) — non-manifold.
        let positions: Vec<f64> = vec![
            0.0, 0.0, 0.0, // 0
            1.0, 0.0, 0.0, // 1
            0.0, 1.0, 0.0, // 2
            0.0, -1.0, 0.0, // 3
            -1.0, 0.0, 0.0, // 4
        ];
        // Each face uses edge (0,1) in the same direction -> after orientation
        // at most two can be made consistent; the third remains a duplicate
        // directed edge / non-manifold edge.
        let triangles = [[0, 1, 2], [0, 1, 3], [0, 1, 4]];
        let err = run_pipeline_full(&positions, 5, &triangles, 1e-6).unwrap_err();
        match err {
            SoupError::NonManifoldRemainder { from, to } => {
                assert_eq!(from, 0);
                assert_eq!(to, 1);
            }
            SoupError::TopologyBuildFailed(TopologyError::DuplicateDirectedEdge { from, to }) => {
                assert_eq!(from, 0);
                assert_eq!(to, 1);
            }
            other => panic!("expected non-manifold/duplicate edge, got {:?}", other),
        }
    }

    #[test]
    fn pipeline_empty_soup() {
        let positions: Vec<f64> = vec![];
        let triangles: Vec<[u32; 3]> = vec![];
        let out = run_pipeline_full(&positions, 0, &triangles, 1e-6).unwrap();
        assert_eq!(out.report.merged_vertex_count, 0);
        assert_eq!(out.report.surviving_triangle_count, 0);
        assert_eq!(out.report.flipped_face_count, 0);
        assert_eq!(out.report.oriented_component_count, 0);
        // No half-edge workspace needed for empty input; topology is None.
        assert!(out.report.topology.is_none());
    }

    #[test]
    fn pipeline_single_triangle_passes_through() {
        let positions: Vec<f64> = vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0];
        let triangles = [[0, 1, 2]];
        let out = run_pipeline_full(&positions, 3, &triangles, 1e-6).unwrap();
        assert_eq!(out.report.merged_vertex_count, 3);
        assert_eq!(out.report.surviving_triangle_count, 1);
        assert_eq!(out.report.flipped_face_count, 0);
        assert_eq!(out.repaired[0], [0, 1, 2]);
        assert!(out.report.topology.is_some());
        let topo = out.report.topology.unwrap();
        assert_eq!(topo.face_count, 1);
        assert_eq!(topo.boundary_half_edges, 3);
    }

    #[test]
    fn pipeline_flips_inconsistent_winding() {
        // Two triangles sharing edge (1,2) with the same directed edge in both
        // -> face 1 must be flipped and the result must build half-edges.
        let positions: Vec<f64> = vec![
            0.0, 0.0, 0.0, // 0
            1.0, 0.0, 0.0, // 1
            0.0, 1.0, 0.0, // 2
            1.0, 1.0, 0.0, // 3
        ];
        let triangles = [[0, 1, 2], [1, 2, 3]];
        let out = run_pipeline_full(&positions, 4, &triangles, 1e-6).unwrap();
        assert_eq!(out.report.flipped_face_count, 1);
        assert_eq!(out.repaired[0], [0, 1, 2]);
        assert_eq!(out.repaired[1], [1, 3, 2]); // flipped
        assert!(out.report.topology.is_some());
        let topo = out.report.topology.unwrap();
        assert_eq!(topo.boundary_half_edges, 4);
    }

    #[test]
    fn count_flipped_detects_changes() {
        let original = [[0, 1, 2], [3, 4, 5]];
        let current = [[0, 1, 2], [3, 5, 4]];
        assert_eq!(count_flipped(&original, &current), 1);
    }
}
