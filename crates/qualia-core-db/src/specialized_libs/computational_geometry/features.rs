//! Geometry/topology feature bridge for ML, retrieval, and P64 inference paths.

use crate::tensor::Tensor10D;

use super::{HalfEdge, INVALID_INDEX};
use super::connectivity::ConnectivitySummary;
use super::csr_adjacency::{CsrHeader, CsrSummary};

// ---------------------------------------------------------------------------
// Compile-time alignment / Pod assertions for GPU staging
// ---------------------------------------------------------------------------

const _: () = assert!(std::mem::size_of::<CsrHeader>() == 16);
const _: () = assert!(std::mem::align_of::<CsrHeader>() == 4);
const _: () = assert!(std::mem::size_of::<CsrSummary>() == 12);
const _: () = assert!(std::mem::align_of::<CsrSummary>() == 4);
const _: () = assert!(std::mem::size_of::<HalfEdge>() == 16);
const _: () = assert!(std::mem::align_of::<HalfEdge>() == 4);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeatureError {
    OutputTooSmall { required: usize },
    VertexOutOfRange { half_edge: usize, vertex: u32 },
}

/// Encode mesh-graph vertex features as 10D manifold records.
///
/// Convenience wrapper that calls [`encode_topology_features_10d_with_connectivity`]
/// with `connectivity = None` (no component/genus enrichment).
///
/// Mapping:
/// - `x/y/z`: geometric position
/// - `q/w/t`: caller-provided epistemic world, domain head, and time
/// - `v`: `3` for a boundary-clique vertex, `0` for an interior Euclidean vertex
/// - `alpha`: degree normalized by the maximum degree in this mesh
/// - `mu`: boundary flag (`1` or `0`)
/// - `sigma`: raw directed half-edge degree
///
/// The operation is allocation-free and leaves all semantic dimensions
/// attached to the original vertex, making the result directly usable by
/// graph-ML, retrieval, P64 projection, and the renderer's tensor projector.
pub fn encode_topology_features_10d(
    positions: &[[f32; 3]],
    half_edges: &[HalfEdge],
    q: f32,
    w: f32,
    t: f32,
    out: &mut [Tensor10D],
) -> Result<usize, FeatureError> {
    encode_topology_features_10d_with_connectivity(positions, half_edges, q, w, t, None, out)
}

/// Encode mesh-graph vertex features as 10D manifold records with connectivity enrichment.
///
/// When `connectivity` is `Some`, the component count and genus are folded into
/// the feature vectors:
/// - `sigma` gets `component_count * 0.001` added (sub-integer encoding)
/// - `v` for interior vertices is set to `genus` (boundary vertices keep v=3)
///
/// See [`encode_topology_features_10d`] for the base mapping.
pub fn encode_topology_features_10d_with_connectivity(
    positions: &[[f32; 3]],
    half_edges: &[HalfEdge],
    q: f32,
    w: f32,
    t: f32,
    connectivity: Option<&ConnectivitySummary>,
    out: &mut [Tensor10D],
) -> Result<usize, FeatureError> {
    if out.len() < positions.len() {
        return Err(FeatureError::OutputTooSmall {
            required: positions.len(),
        });
    }
    for (index, position) in positions.iter().copied().enumerate() {
        out[index] = Tensor10D {
            q,
            v: 0.0,
            w,
            x: position[0],
            y: position[1],
            z: position[2],
            t,
            alpha: 0.0,
            mu: 0.0,
            sigma: 0.0,
        };
    }

    let mut max_degree = 0.0f32;
    for (edge_index, edge) in half_edges.iter().enumerate() {
        let vertex = edge.origin as usize;
        if vertex >= positions.len() {
            return Err(FeatureError::VertexOutOfRange {
                half_edge: edge_index,
                vertex: edge.origin,
            });
        }
        out[vertex].sigma += 1.0;
        max_degree = max_degree.max(out[vertex].sigma);
        if edge.twin == INVALID_INDEX {
            out[vertex].mu = 1.0;
            out[vertex].v = 3.0;
            let destination = half_edges
                .get(edge.next as usize)
                .map(|next| next.origin as usize);
            if let Some(destination) = destination.filter(|&v| v < positions.len()) {
                out[destination].mu = 1.0;
                out[destination].v = 3.0;
            }
        }
    }
    if max_degree > 0.0 {
        for feature in &mut out[..positions.len()] {
            feature.alpha = feature.sigma / max_degree;
        }
    }

    if let Some(summary) = connectivity {
        let component_class = summary.component_count as f32;
        let genus_class = summary.genus.unwrap_or(0) as f32;
        for feature in &mut out[..positions.len()] {
            feature.sigma += component_class * 0.001;
            if feature.mu == 0.0 {
                feature.v = genus_class;
            }
        }
    }

    Ok(positions.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::specialized_libs::computational_geometry::{
        build_triangle_half_edges, compute_connectivity, EdgeSlot,
    };

    #[test]
    fn mesh_graph_becomes_tensor_features() {
        let positions = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        let triangles = [[0, 1, 2]];
        let mut edges = [HalfEdge::default(); 3];
        let mut slots = [EdgeSlot::default(); 8];
        build_triangle_half_edges(3, &triangles, &mut edges, &mut slots).unwrap();

        let mut features = [Tensor10D::default(); 3];
        let n = encode_topology_features_10d(&positions, &edges, 2.0, 7.0, 11.0, &mut features)
            .unwrap();
        assert_eq!(n, 3);
        assert!(features.iter().all(|feature| feature.v == 3.0));
        assert!(features.iter().all(|feature| feature.mu == 1.0));
        assert!(features.iter().all(|feature| feature.alpha == 1.0));
        assert_eq!(features[1].x, 1.0);
        assert_eq!(features[2].w, 7.0);
    }

    #[test]
    fn connectivity_enrichment_tetrahedron() {
        // Tetrahedron: 4 vertices, 4 faces, genus 0, 1 component, no boundary.
        let positions: [[f32; 3]; 4] = [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
        ];
        let triangles = [[0, 1, 2], [0, 2, 3], [0, 3, 1], [1, 3, 2]];
        let mut edges = [HalfEdge::default(); 12];
        let mut slots = [EdgeSlot::default(); 32];
        build_triangle_half_edges(4, &triangles, &mut edges, &mut slots).unwrap();

        let mut labels = [0u32; 4];
        let mut queue = [0u32; 4];
        let mut visited = [false; 12];
        let summary =
            compute_connectivity(4, 4, &edges, &mut labels, &mut queue, &mut visited).unwrap();

        let mut features = [Tensor10D::default(); 4];
        encode_topology_features_10d_with_connectivity(
            &positions,
            &edges,
            1.0,
            2.0,
            3.0,
            Some(&summary),
            &mut features,
        )
        .unwrap();

        // All vertices are interior (no boundary) → v = genus = 0.
        assert!(features.iter().all(|f| f.v == 0.0));
        // sigma = degree + component_count * 0.001 = 3 + 0.001
        assert!(features.iter().all(|f| f.sigma > 3.0 && f.sigma < 3.002));
    }

    #[test]
    fn connectivity_enrichment_single_triangle() {
        let positions = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        let triangles = [[0, 1, 2]];
        let mut edges = [HalfEdge::default(); 3];
        let mut slots = [EdgeSlot::default(); 8];
        build_triangle_half_edges(3, &triangles, &mut edges, &mut slots).unwrap();

        let mut labels = [0u32; 1];
        let mut queue = [0u32; 1];
        let mut visited = [false; 3];
        let summary =
            compute_connectivity(3, 1, &edges, &mut labels, &mut queue, &mut visited).unwrap();

        let mut features = [Tensor10D::default(); 3];
        encode_topology_features_10d_with_connectivity(
            &positions,
            &edges,
            1.0,
            2.0,
            3.0,
            Some(&summary),
            &mut features,
        )
        .unwrap();

        // All vertices are boundary → v = 3 (not overwritten by genus).
        assert!(features.iter().all(|f| f.v == 3.0));
        // sigma = degree + component_count * 0.001 = 1 + 0.001
        assert!(features.iter().all(|f| f.sigma > 1.0 && f.sigma < 1.002));
    }

    #[test]
    fn differential_vs_naive_oracle_multi_component() {
        // Two disjoint triangles → 2 components, genus None.
        let positions: [[f32; 3]; 6] = [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [2.0, 0.0, 0.0],
            [3.0, 0.0, 0.0],
            [2.0, 1.0, 0.0],
        ];
        let triangles = [[0, 1, 2], [3, 4, 5]];
        let mut edges = [HalfEdge::default(); 6];
        let mut slots = [EdgeSlot::default(); 16];
        build_triangle_half_edges(6, &triangles, &mut edges, &mut slots).unwrap();

        let mut labels = [0u32; 2];
        let mut queue = [0u32; 2];
        let mut visited = [false; 6];
        let summary =
            compute_connectivity(6, 2, &edges, &mut labels, &mut queue, &mut visited).unwrap();
        assert_eq!(summary.component_count, 2);

        let mut features = [Tensor10D::default(); 6];
        encode_topology_features_10d_with_connectivity(
            &positions,
            &edges,
            1.0,
            2.0,
            3.0,
            Some(&summary),
            &mut features,
        )
        .unwrap();

        // Naive oracle: degree = 1 for all, component_count = 2.
        // sigma = 1 + 2 * 0.001 = 1.002
        assert!(features.iter().all(|f| (f.sigma - 1.002).abs() < 1e-5));
    }
}
