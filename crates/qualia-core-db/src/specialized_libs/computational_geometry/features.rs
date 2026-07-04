//! Geometry/topology feature bridge for ML, retrieval, and P64 inference paths.

use crate::tensor::Tensor10D;

use super::{HalfEdge, INVALID_INDEX};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeatureError {
    OutputTooSmall { required: usize },
    VertexOutOfRange { half_edge: usize, vertex: u32 },
}

/// Encode mesh-graph vertex features as 10D manifold records.
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
    Ok(positions.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::specialized_libs::computational_geometry::{build_triangle_half_edges, EdgeSlot};

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
}
