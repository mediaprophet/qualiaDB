//! 10D manifold frustum pullback and T16 `Epistemic10D` topological projection.
//!
//! Tier-1 Zero-Heap (`AGENTS.md` §0 & §0-A): bounded $k$-hop BFS filtered by a 10D coordinate
//! frustum and ambient metric radius, plus the host-internal T16 morphism projecting
//! `PoetGeometryHeader` + `HarbouredGraphHeader` + `HomologySummary` into `ManifoldCoordinate10D`.

use crate::modalities::manifold::ManifoldCoordinate10D;
use crate::tensor::topology::TopologyClass;
use crate::tensor::Tensor10D;
use crate::topology::csr_view::HarbouredCsrView;
use crate::topology::header::{HarbouredGraphHeader, PoetGeometryHeader, TopologyError};
use crate::topology::invariants::HomologySummary;

/// 10-dimensional axis-aligned bounding frustum and step-metric gate over `Tensor10D`
/// (`[q, v, w, x, y, z, t, α, μ, σ]`).
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Frustum10D {
    pub min: [f32; 10],
    pub max: [f32; 10],
    pub max_metric_radius: f32,
    pub topology_class: TopologyClass,
}

impl Default for Frustum10D {
    fn default() -> Self {
        Self::full_range(f32::INFINITY, TopologyClass::Euclidean)
    }
}

impl Frustum10D {
    /// Construct an unbounded 10D frustum gated only by `max_metric_radius` and `topology_class`.
    #[inline]
    pub const fn full_range(max_metric_radius: f32, topology_class: TopologyClass) -> Self {
        Self {
            min: [f32::NEG_INFINITY; 10],
            max: [f32::INFINITY; 10],
            max_metric_radius,
            topology_class,
        }
    }

    /// Returns `true` when all 10 axes `[q, v, w, x, y, z, t, α, μ, σ]` of `t`
    /// lie within `[self.min[i], self.max[i]]`.
    #[inline]
    pub fn contains_tensor(&self, t: &Tensor10D) -> bool {
        let axes = [
            t.q, t.v, t.w, t.x, t.y, t.z, t.t, t.alpha, t.mu, t.sigma,
        ];
        let mut i = 0usize;
        while i < 10 {
            let val = axes[i];
            if val < self.min[i] || val > self.max[i] {
                return false;
            }
            i += 1;
        }
        true
    }
}

/// Non-recursive bounded $k$-hop BFS starting at `start_vertex`, pruning any neighbour `w`
/// outside `frustum` or exceeding `frustum.max_metric_radius` from `u` under `frustum.topology_class`.
#[allow(clippy::too_many_arguments)]
pub fn traverse_10d_pullback(
    csr: &HarbouredCsrView<'_>,
    coords_10d: &[Tensor10D],
    start_vertex: u32,
    max_hops: u8,
    frustum: &Frustum10D,
    visited_bits: &mut [u64],
    queue_v: &mut [u32],
    queue_depth: &mut [u8],
    out_vertices: &mut [u32],
) -> Result<usize, TopologyError> {
    let v_count = csr.vertex_count();
    if start_vertex >= v_count {
        return Err(TopologyError::VertexOutOfRange {
            vertex: start_vertex,
            count: v_count,
        });
    }

    let n = v_count as usize;
    let words = (n + 63) / 64;
    if coords_10d.len() < n
        || visited_bits.len() < words
        || queue_v.is_empty()
        || queue_depth.len() < queue_v.len()
        || out_vertices.is_empty()
    {
        return Err(TopologyError::BufferTooSmall);
    }

    visited_bits[..words].fill(0);

    if !frustum.contains_tensor(&coords_10d[start_vertex as usize]) {
        return Ok(0);
    }

    set_bit(visited_bits, start_vertex as usize);
    queue_v[0] = start_vertex;
    queue_depth[0] = 0;
    out_vertices[0] = start_vertex;

    let mut head = 0usize;
    let mut tail = 1usize;
    let mut written = 1usize;

    while head < tail {
        let u = queue_v[head];
        let depth = queue_depth[head];
        head += 1;

        if depth >= max_hops {
            continue;
        }

        let cu = &coords_10d[u as usize];
        let p_u = (cu.x, cu.y, cu.z);
        let nbrs = csr.out_neighbors(u)?;

        let mut i = 0usize;
        while i < nbrs.len() {
            let w = nbrs[i];
            i += 1;
            let w_idx = w as usize;
            if test_bit(visited_bits, w_idx) {
                continue;
            }
            let cw = &coords_10d[w_idx];
            if !frustum.contains_tensor(cw) {
                continue;
            }
            let dist = frustum
                .topology_class
                .calculate_distance(p_u, (cw.x, cw.y, cw.z));
            if dist > frustum.max_metric_radius {
                continue;
            }

            set_bit(visited_bits, w_idx);
            if tail >= queue_v.len() || written >= out_vertices.len() {
                return Err(TopologyError::BufferTooSmall);
            }
            queue_v[tail] = w;
            queue_depth[tail] = depth + 1;
            tail += 1;
            out_vertices[written] = w;
            written += 1;
        }
    }

    Ok(written)
}

/// T16 host-internal morphism projecting the active harboured graph's topological state
/// and cellular homology onto `ManifoldCoordinate10D` (`Epistemic10D`):
/// - `d[3]` (`topological_spin`): derived from `graph_header.graph_id` and `homology.betti_1`
/// - `d[9]` (`manifold_curvature`): derived from `geom_header.euler_characteristic` and `homology.euler_characteristic`
pub fn project_t16_epistemic_10d(
    geom_header: &PoetGeometryHeader,
    graph_header: &HarbouredGraphHeader,
    homology: &HomologySummary,
    epistemic: &mut ManifoldCoordinate10D,
) {
    let folded_id = (graph_header.graph_id ^ (graph_header.graph_id >> 32)) & 0xFFFF;
    let id_spin_bias = if graph_header.graph_id == 0 {
        0.0
    } else {
        ((folded_id.max(1)) as f32) / 65536.0
    };

    epistemic.topological_spin = (homology.betti_1 as f32) + id_spin_bias;
    epistemic.manifold_curvature =
        (geom_header.euler_characteristic as f32) - 0.5 * (homology.euler_characteristic as f32);

    if graph_header.vertex_count > 0 {
        epistemic.recurrence_frequency =
            (homology.betti_1 as f32) / (graph_header.vertex_count as f32);
    }
}

#[inline]
fn test_bit(bits: &[u64], idx: usize) -> bool {
    (bits[idx >> 6] & (1u64 << (idx & 63))) != 0
}

#[inline]
fn set_bit(bits: &mut [u64], idx: usize) {
    bits[idx >> 6] |= 1u64 << (idx & 63);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::topology::header::CellAnchor;

    #[test]
    fn pullback_traversal_filters_by_frustum_and_metric_radius() {
        // 0 -> 1 -> 2, and 0 -> 3 (where 3 is too far in metric distance)
        let header = HarbouredGraphHeader {
            graph_id: 0x1234_5678,
            vertex_count: 4,
            edge_count: 3,
            ..Default::default()
        };
        let anchors = [CellAnchor::default(); 4];
        let row_ptr = [0u32, 2, 3, 3, 3];
        let col_idx = [1u32, 3, 2];
        let edge_anc = [0u32, 1, 2];
        let csc_ptr = [0u32, 0, 1, 2, 3];
        let csc_row = [0u32, 1, 0];
        let csr = HarbouredCsrView::from_slices(
            &header, &anchors, &row_ptr, &col_idx, &edge_anc, &csc_ptr, &csc_row, None,
        )
        .unwrap();

        let coords = [
            Tensor10D::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0),
            Tensor10D::new(0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0),
            Tensor10D::new(0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 2.0, 1.0, 0.0, 0.0),
            Tensor10D::new(0.0, 0.0, 0.0, 10.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0),
        ];

        let frustum = Frustum10D::full_range(2.0, TopologyClass::Euclidean);
        let mut bits = [0u64; 1];
        let mut q_v = [0u32; 4];
        let mut q_d = [0u8; 4];
        let mut out = [0u32; 4];

        let n = traverse_10d_pullback(
            &csr, &coords, 0, 2, &frustum, &mut bits, &mut q_v, &mut q_d, &mut out,
        )
        .unwrap();
        assert_eq!(n, 3);
        assert_eq!(&out[..3], &[0, 1, 2]);

        let geom_hdr = PoetGeometryHeader {
            euler_characteristic: 2,
            ..Default::default()
        };
        let homology = HomologySummary {
            euler_characteristic: 1,
            betti_0: 1,
            betti_1: 2,
            root_count: 1,
            sink_count: 2,
        };
        let mut ep = ManifoldCoordinate10D::default();
        project_t16_epistemic_10d(&geom_hdr, &header, &homology, &mut ep);
        assert!(ep.topological_spin > 2.0);
        assert!((ep.manifold_curvature - 1.5).abs() < 1e-5);
    }
}
