//! P21.5 — Anisotropic differential surface growth (Laplace–Beltrami driven).
//!
//! Each vertex carries a scalar growth rate `g_i`; every rest edge relaxes
//! toward `L_e · (1 + dt·ḡ_e)` where `ḡ_e` is the edge-mean growth. A Gauss–
//! Seidel relaxation pass moves vertices toward the new metric; overgrown
//! edges are bisected with local 1→2 triangle splits so leaf/petal/fungal
//! surfaces can ruffle without a fixed mesh resolution.
//!
//! `differential_growth_step` is Tier-1 `HotZeroHeap`; `split_stretched_edges`
//! writes into caller-bounded mesh buffers (`ColdBounded` capacity semantics).

use super::primitives::Point3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DifferentialGrowthError {
    EmptyInput,
    DimensionMismatch,
    InvalidParameters,
    OutputTooSmall { required: usize },
}

#[inline]
fn sub(a: Point3, b: Point3) -> Point3 {
    Point3::new(a.x - b.x, a.y - b.y, a.z - b.z)
}
#[inline]
fn dot(a: Point3, b: Point3) -> f64 {
    a.x * b.x + a.y * b.y + a.z * b.z
}

/// Fill `rest_lengths` with the current edge lengths (one per `edges` entry).
///
/// Tier-1 `HotZeroHeap`.
pub fn edge_rest_lengths(
    vertices: &[Point3],
    edges: &[(u32, u32)],
    rest_lengths: &mut [f64],
) -> Result<(), DifferentialGrowthError> {
    if rest_lengths.len() < edges.len() {
        return Err(DifferentialGrowthError::OutputTooSmall {
            required: edges.len(),
        });
    }
    for (i, &(a, b)) in edges.iter().enumerate() {
        let (ia, ib) = (a as usize, b as usize);
        if ia >= vertices.len() || ib >= vertices.len() {
            return Err(DifferentialGrowthError::DimensionMismatch);
        }
        let d = sub(vertices[ia], vertices[ib]);
        rest_lengths[i] = dot(d, d).sqrt();
    }
    Ok(())
}

/// One differential-growth relaxation step.
///
/// - `growth[i]` is the local growth rate at vertex i (unitless per dt).
/// - `rest_lengths[e]` is updated in place: `L_e ← L_e·(1 + dt·ḡ_e)`.
/// - Positions relax toward the new metric with `stiffness ∈ (0,1]`.
///
/// Deterministic Gauss–Seidel scan order over `edges`.
/// Tier-1 `HotZeroHeap`.
pub fn differential_growth_step(
    vertices: &mut [Point3],
    edges: &[(u32, u32)],
    growth: &[f64],
    rest_lengths: &mut [f64],
    stiffness: f64,
    dt: f64,
) -> Result<(), DifferentialGrowthError> {
    let n = vertices.len();
    if n == 0 {
        return Err(DifferentialGrowthError::EmptyInput);
    }
    if growth.len() < n
        || rest_lengths.len() < edges.len()
        || !(stiffness > 0.0 && stiffness <= 1.0)
        || !(dt > 0.0 && dt.is_finite())
    {
        return Err(DifferentialGrowthError::InvalidParameters);
    }
    let w = stiffness;
    for (ei, &(a, b)) in edges.iter().enumerate() {
        let (ia, ib) = (a as usize, b as usize);
        if ia >= n || ib >= n {
            return Err(DifferentialGrowthError::DimensionMismatch);
        }
        // Update rest length toward the grown metric.
        rest_lengths[ei] *= 1.0 + dt * 0.5 * (growth[ia] + growth[ib]);
        let d = sub(vertices[ib], vertices[ia]);
        let len = dot(d, d).sqrt();
        if len < 1e-12 {
            continue;
        }
        // Half the length residual applied symmetrically to both ends.
        let corr = 0.5 * w * (len - rest_lengths[ei]) / len;
        vertices[ia].x += d.x * corr;
        vertices[ia].y += d.y * corr;
        vertices[ia].z += d.z * corr;
        vertices[ib].x -= d.x * corr;
        vertices[ib].y -= d.y * corr;
        vertices[ib].z -= d.z * corr;
    }
    Ok(())
}

/// Bisect every edge whose current length exceeds `max_stretch · rest_length`.
///
/// For each triangle adjacent to a stretched edge `(u, v)` the edge is split
/// once: the midpoint vertex is appended at `vertex_count + k` and the
/// triangle `(u, v, w)` is replaced by `(u, m, w)` plus a new triangle
/// `(m, v, w)` appended at the end of the buffer.
///
/// Returns `(new_vertex_count, new_triangle_count)`. Caller buffers bound the
/// output: midpoints are recorded into `edge_midpoint` (`edge_idx → vertex
/// index`, `u32::MAX` for unsplit).
///
/// `ColdBounded` (capacity semantics on caller slices).
#[allow(clippy::too_many_arguments)]
pub fn split_stretched_edges(
    vertices: &mut [Point3],
    triangles: &mut [[u32; 3]],
    vertex_count: usize,
    triangle_count: usize,
    edges: &[(u32, u32)],
    rest_lengths: &[f64],
    max_stretch: f64,
    edge_midpoint: &mut [u32],
) -> Result<(usize, usize), DifferentialGrowthError> {
    if vertex_count == 0
        || !(max_stretch > 1.0 && max_stretch.is_finite())
        || edges.len() > edge_midpoint.len()
        || rest_lengths.len() < edges.len()
    {
        return Err(DifferentialGrowthError::InvalidParameters);
    }
    let (mut nv, mut nt) = (vertex_count, triangle_count);
    edge_midpoint[..edges.len()].fill(u32::MAX);

    // Pass 1 — record midpoint indices for stretched edges.
    for (ei, &(a, b)) in edges.iter().enumerate() {
        let (ia, ib) = (a as usize, b as usize);
        if ia >= nv || ib >= nv {
            return Err(DifferentialGrowthError::DimensionMismatch);
        }
        let d = sub(vertices[ia], vertices[ib]);
        let len = dot(d, d).sqrt();
        if len > max_stretch * rest_lengths[ei] {
            if nv >= vertices.len() {
                return Err(DifferentialGrowthError::OutputTooSmall { required: nv + 1 });
            }
            vertices[nv] = Point3::new(
                0.5 * (vertices[ia].x + vertices[ib].x),
                0.5 * (vertices[ia].y + vertices[ib].y),
                0.5 * (vertices[ia].z + vertices[ib].z),
            );
            edge_midpoint[ei] = nv as u32;
            nv += 1;
        }
    }

    // Pass 2 — retriangulate: each triangle containing a split edge spawns
    // one extra triangle (1→2 split per split edge).
    let split_pos = |tri: &[u32; 3], u: u32, v: u32| -> Option<usize> {
        (0..3).find(|&e| {
            (tri[e] == u && tri[(e + 1) % 3] == v) || (tri[e] == v && tri[(e + 1) % 3] == u)
        })
    };
    for ti in 0..triangle_count {
        let tri = triangles[ti];
        // At most one split per pass per edge index; handle each edge once.
        let mut applied = false;
        for (ei, &(a, b)) in edges.iter().enumerate() {
            let m = edge_midpoint[ei];
            if m == u32::MAX {
                continue;
            }
            if let Some(e) = split_pos(&tri, a, b) {
                let (u, v) = (tri[e], tri[(e + 1) % 3]);
                let w = tri[(e + 2) % 3];
                if nt >= triangles.len() {
                    return Err(DifferentialGrowthError::OutputTooSmall { required: nt + 1 });
                }
                triangles[ti] = [u, m, w];
                triangles[nt] = [m, v, w];
                nt += 1;
                applied = true;
                break;
            }
        }
        let _ = applied;
    }
    Ok((nv, nt))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn growth_relax_stretches_edge_and_split_adds_vertex() {
        let mut verts = [Point3::default(); 8];
        verts[..3].copy_from_slice(&[
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.5, 1.0, 0.0),
        ]);
        let edges = [(0u32, 1u32), (1, 2), (2, 0)];
        let growth = [1.0, 1.0, 0.0];
        let mut rest = [0.0f64; 3];
        edge_rest_lengths(&verts[..3], &edges, &mut rest).unwrap();
        let r0 = rest[0];
        // Growing edge 0-1 pushes the endpoints apart.
        differential_growth_step(&mut verts[..3], &edges, &growth, &mut rest, 0.5, 0.5).unwrap();
        assert!(rest[0] > r0, "rest length should grow: {r0} → {}", rest[0]);
        let d = sub(verts[1], verts[0]);
        let len = dot(d, d).sqrt();
        assert!(len > 1.0, "edge should lengthen, got {len}");

        // A forced split: mark edge 0 as stretched.
        let mut tris = [[0u32, 1, 2], [0; 3], [0; 3]];
        let mut mids = [u32::MAX; 3];
        rest[0] = 0.1; // pretend the rest length is tiny → stretched
        let (nv, nt) =
            split_stretched_edges(&mut verts, &mut tris, 3, 1, &edges, &rest, 1.5, &mut mids)
                .unwrap();
        assert_eq!(nv, 4);
        assert_eq!(nt, 2);
        assert_ne!(mids[0], u32::MAX);
    }
}
