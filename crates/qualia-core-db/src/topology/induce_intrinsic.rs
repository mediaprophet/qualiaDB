//! Tier-2 cold induction of intrinsic graphs (`Primal1Skeleton`, `PoincareDual`, `BoundaryIncidence`,
//! `CombinatorialDartMap`, `MetricProximity`) from `computational_geometry` complexes.
//!
//! Tier-2 Cold Bounded (`AGENTS.md` §0-A): every builder enforces an explicit caller-supplied
//! `budget_bytes` before allocating scratch buffers and fails closed with [`TopologyError::BudgetExhausted`].

use bytemuck::Zeroable;

use crate::specialized_libs::computational_geometry::ddg_operators::{
    boundary_of_boundary_is_zero, boundary_of_simplex, ChainTerm,
};
use crate::specialized_libs::computational_geometry::mixed_cell_topology::{
    extract_boundary_faces, BoundaryFace, MixedCell, MixedTopologyError, CELL_KIND_HEX,
    CELL_KIND_QUAD, CELL_KIND_TETRA, CELL_KIND_TRIANGLE,
};
use crate::specialized_libs::computational_geometry::{
    build_face_adjacency_csr, build_vertex_adjacency_csr, half_edges_to_darts,
    validate_combinatorial_map, CsrError, Dart, HalfEdge, INVALID_INDEX,
};
use crate::tensor::topology::TopologyClass;
use crate::tensor::Tensor10D;
use crate::topology::compactor::UnsealedGraphSpec;
use crate::topology::header::{
    AnchorDomain, CellAnchor, GraphEmbedRole, GraphMorphology, TopologyError, TopologyInvariants,
};

#[inline]
fn derive_graph_id(geometry_id: u64, role: GraphEmbedRole) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325u64 ^ geometry_id;
    h = (h ^ (role as u64)).wrapping_mul(0x0100_0000_01b3);
    h
}

#[inline]
fn derive_cell_hash(geometry_id: u64, domain: AnchorDomain, cell_index: u32) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325u64 ^ geometry_id;
    h = (h ^ (domain as u64)).wrapping_mul(0x0100_0000_01b3);
    h = (h ^ (cell_index as u64)).wrapping_mul(0x0100_0000_01b3);
    h
}

#[inline]
fn map_csr_err(err: CsrError, count: u32, he_count: u32) -> TopologyError {
    match err {
        CsrError::OffsetBufferTooSmall { .. } | CsrError::NeighbourBufferTooSmall { .. } => TopologyError::BufferTooSmall,
        CsrError::HalfEdgeOutOfRange { index } => TopologyError::CellOutOfRange { domain: AnchorDomain::Cell1Edge as u8, index, count: he_count },
        CsrError::EntityOutOfRange { index } => TopologyError::VertexOutOfRange { vertex: index, count },
    }
}

#[inline]
fn make_cell_anchors(geometry_id: u64, count: usize, domain: AnchorDomain) -> Vec<CellAnchor> {
    (0..count)
        .map(|idx| CellAnchor {
            entity_hash: derive_cell_hash(geometry_id, domain, idx as u32),
            cell_index: idx as u32,
            anchor_domain: domain as u8,
            orientation_sign: 1,
            material_or_stratum: 0,
        })
        .collect()
}

/// Induce both the `Primal1Skeleton` (`Cell0Vertex ↔ Cell0Vertex`) and the `PoincareDual`
/// (`Cell2Face ↔ Cell2Face`) harboured graphs from a half-edge surface mesh.
pub fn induce_primal_and_dual_from_half_edges(
    geometry_id: u64,
    half_edges: &[HalfEdge],
    num_vertices: usize,
    num_faces: usize,
    budget_bytes: usize,
) -> Result<(UnsealedGraphSpec, UnsealedGraphSpec), TopologyError> {
    let he_len = half_edges.len();
    let scratch_bytes = (num_vertices + num_faces + 2).saturating_mul(4).saturating_add(he_len.saturating_mul(8));
    let anchor_bytes = (num_vertices + num_faces).saturating_mul(core::mem::size_of::<CellAnchor>());
    let requested = scratch_bytes.saturating_add(anchor_bytes).saturating_add(he_len.saturating_mul(36));
    if requested > budget_bytes {
        return Err(TopologyError::BudgetExhausted { requested, budget: budget_bytes });
    }

    let mut v_offsets = vec![0u32; num_vertices + 1];
    let mut v_neighbours = vec![0u32; he_len];
    build_vertex_adjacency_csr(num_vertices as u32, half_edges, &mut v_offsets, &mut v_neighbours)
        .map_err(|e| map_csr_err(e, num_vertices as u32, he_len as u32))?;
    let primal_anchors = make_cell_anchors(geometry_id, num_vertices, AnchorDomain::Cell0Vertex);

    let mut primal_edges = Vec::with_capacity(he_len * 2);
    for (he_idx, he) in half_edges.iter().enumerate() {
        let u = he.origin;
        let dst = half_edges.get(he.next as usize).ok_or(TopologyError::CellOutOfRange {
            domain: AnchorDomain::Cell1Edge as u8, index: he.next, count: he_len as u32,
        })?.origin;
        primal_edges.push((u, dst, he_idx as u32));
        if he.twin == INVALID_INDEX {
            primal_edges.push((dst, u, he_idx as u32));
        }
    }
    primal_edges.sort_unstable_by_key(|&(u, v, a)| (u, v, a));
    primal_edges.dedup_by(|a, b| a.0 == b.0 && a.1 == b.1);

    let mut f_offsets = vec![0u32; num_faces + 1];
    let mut f_neighbours = vec![0u32; he_len];
    build_face_adjacency_csr(num_faces as u32, half_edges, &mut f_offsets, &mut f_neighbours)
        .map_err(|e| map_csr_err(e, num_faces as u32, he_len as u32))?;
    let dual_anchors = make_cell_anchors(geometry_id, num_faces, AnchorDomain::Cell2Face);

    let mut dual_edges = Vec::with_capacity(he_len);
    let mut has_boundary = false;
    for (he_idx, he) in half_edges.iter().enumerate() {
        if he.twin == INVALID_INDEX {
            has_boundary = true;
        } else {
            let nbr_face = half_edges[he.twin as usize].face;
            dual_edges.push((he.face, nbr_face, he_idx as u32));
        }
    }
    dual_edges.sort_unstable_by_key(|&(u, v, a)| (u, v, a));
    dual_edges.dedup_by(|a, b| a.0 == b.0 && a.1 == b.1);

    let dual_invariants = if !has_boundary && !half_edges.is_empty() {
        TopologyInvariants::SYMMETRIC | TopologyInvariants::MANIFOLD_CLOSED
    } else {
        TopologyInvariants::SYMMETRIC
    };

    let primal = UnsealedGraphSpec {
        graph_id: derive_graph_id(geometry_id, GraphEmbedRole::Primal1Skeleton),
        embed_role: GraphEmbedRole::Primal1Skeleton,
        morphology: GraphMorphology::SymmetricGraph,
        invariants: TopologyInvariants::SYMMETRIC,
        context_and_state_mask: 0,
        anchors: primal_anchors,
        edges: primal_edges,
        edge_weights: None,
    };
    let dual = UnsealedGraphSpec {
        graph_id: derive_graph_id(geometry_id, GraphEmbedRole::PoincareDual),
        embed_role: GraphEmbedRole::PoincareDual,
        morphology: GraphMorphology::SymmetricGraph,
        invariants: dual_invariants,
        context_and_state_mask: 0,
        anchors: dual_anchors,
        edges: dual_edges,
        edge_weights: None,
    };
    Ok((primal, dual))
}

/// Induce a 3-cell `PoincareDual` graph (`AnchorDomain::Cell3Volume`) from a `MixedCell` complex.
pub fn induce_mixed_cell_dual_graph(
    geometry_id: u64,
    cells: &[MixedCell],
    budget_bytes: usize,
) -> Result<UnsealedGraphSpec, TopologyError> {
    let max_facets = cells.len().saturating_mul(6);
    let requested = cells
        .len()
        .saturating_mul(core::mem::size_of::<CellAnchor>())
        .saturating_add(max_facets.saturating_mul(core::mem::size_of::<BoundaryFace>() + 24));
    if requested > budget_bytes {
        return Err(TopologyError::BudgetExhausted {
            requested,
            budget: budget_bytes,
        });
    }

    let mut boundary_scratch = vec![BoundaryFace::zeroed(); max_facets];
    extract_boundary_faces(cells, &mut boundary_scratch).map_err(|e| match e {
        MixedTopologyError::UnknownCellKind { kind, .. } => TopologyError::InvalidComplexKind(kind),
        _ => TopologyError::BufferTooSmall,
    })?;

    // Extract canonical facets (count, sorted_verts, owner_cell) for cell-to-cell dual adjacency.
    let mut facets: Vec<(u8, [u32; 4], u32)> = Vec::with_capacity(max_facets);
    for (ci, cell) in cells.iter().enumerate() {
        let owner = ci as u32;
        let v = cell.vertices;
        match cell.kind {
            CELL_KIND_TETRA => {
                for raw in [[v[0], v[2], v[1], 0], [v[0], v[1], v[3], 0], [v[1], v[2], v[3], 0], [v[2], v[0], v[3], 0]] {
                    let mut key = raw;
                    key[..3].sort_unstable();
                    facets.push((3, key, owner));
                }
            }
            CELL_KIND_HEX => {
                for raw in [[v[0], v[1], v[2], v[3]], [v[4], v[7], v[6], v[5]], [v[0], v[4], v[5], v[1]], [v[1], v[5], v[6], v[2]], [v[2], v[6], v[7], v[3]], [v[3], v[7], v[4], v[0]]] {
                    let mut key = raw;
                    key.sort_unstable();
                    facets.push((4, key, owner));
                }
            }
            CELL_KIND_TRIANGLE => {
                for (a, b) in [(v[0], v[1]), (v[1], v[2]), (v[2], v[0])] {
                    facets.push((2, [a.min(b), a.max(b), 0, 0], owner));
                }
            }
            CELL_KIND_QUAD => {
                for (a, b) in [(v[0], v[1]), (v[1], v[2]), (v[2], v[3]), (v[3], v[0])] {
                    facets.push((2, [a.min(b), a.max(b), 0, 0], owner));
                }
            }
            other => return Err(TopologyError::InvalidComplexKind(other)),
        }
    }
    facets.sort_unstable_by_key(|&(cnt, key, owner)| (cnt, key, owner));

    let mut edges = Vec::new();
    let mut i = 0usize;
    let mut shared_facet_id = 0u32;
    while i < facets.len() {
        let (cnt, key, _) = facets[i];
        let start = i;
        while i < facets.len() && facets[i].0 == cnt && facets[i].1 == key {
            i += 1;
        }
        if i - start >= 2 {
            for a in start..i {
                for b in (a + 1)..i {
                    let ca = facets[a].2;
                    let cb = facets[b].2;
                    if ca != cb {
                        edges.push((ca, cb, shared_facet_id));
                        edges.push((cb, ca, shared_facet_id));
                    }
                }
            }
            shared_facet_id += 1;
        }
    }
    edges.sort_unstable_by_key(|&(u, v, a)| (u, v, a));
    edges.dedup_by(|a, b| a.0 == b.0 && a.1 == b.1);

    let mut anchors = Vec::with_capacity(cells.len());
    for (ci, cell) in cells.iter().enumerate() {
        anchors.push(CellAnchor {
            entity_hash: derive_cell_hash(geometry_id, AnchorDomain::Cell3Volume, ci as u32),
            cell_index: ci as u32,
            anchor_domain: AnchorDomain::Cell3Volume as u8,
            orientation_sign: 1,
            material_or_stratum: cell.material,
        });
    }

    Ok(UnsealedGraphSpec {
        graph_id: derive_graph_id(geometry_id, GraphEmbedRole::PoincareDual),
        embed_role: GraphEmbedRole::PoincareDual,
        morphology: GraphMorphology::SymmetricGraph,
        invariants: TopologyInvariants::SYMMETRIC,
        context_and_state_mask: 0,
        anchors,
        edges,
        edge_weights: None,
    })
}

/// Induce a Levi / Hasse `BoundaryIncidence` (`BipartiteFactor`) graph (`Cell2Face → Cell1Edge`)
/// while verifying $\partial \circ \partial = 0$ on every 2-simplex via [`boundary_of_boundary_is_zero`].
pub fn induce_simplicial_boundary_incidence(
    geometry_id: u64,
    triangles: &[[u32; 3]],
    num_vertices: u32,
    budget_bytes: usize,
) -> Result<UnsealedGraphSpec, TopologyError> {
    let f_count = triangles.len();
    let max_edges = f_count.saturating_mul(3);
    let requested = (f_count.saturating_add(max_edges))
        .saturating_mul(core::mem::size_of::<CellAnchor>())
        .saturating_add(max_edges.saturating_mul(16));
    if requested > budget_bytes {
        return Err(TopologyError::BudgetExhausted {
            requested,
            budget: budget_bytes,
        });
    }

    let mut raw_incidences: Vec<(usize, (u32, u32), i8)> = Vec::with_capacity(max_edges);
    let mut unique_edges: Vec<(u32, u32)> = Vec::with_capacity(max_edges);

    for (f_idx, tri) in triangles.iter().enumerate() {
        for &v in tri {
            if v >= num_vertices {
                return Err(TopologyError::VertexOutOfRange {
                    vertex: v,
                    count: num_vertices,
                });
            }
        }
        if tri[0] == tri[1] || tri[1] == tri[2] || tri[2] == tri[0] {
            return Err(TopologyError::ChainBoundaryNotZero { non_zero_terms: 1 });
        }
        let simplex = ChainTerm {
            dim: 2,
            vertex_count: 3,
            sign: 1,
            vertices: [tri[0], tri[1], tri[2], 0],
        };
        if !boundary_of_boundary_is_zero(simplex) {
            return Err(TopologyError::ChainBoundaryNotZero { non_zero_terms: 1 });
        }
        let mut faces_1 = [ChainTerm::zeroed(); 4];
        let n = boundary_of_simplex(simplex, &mut faces_1)
            .map_err(|_| TopologyError::BufferTooSmall)?;
        for term in &faces_1[..n] {
            let a = term.vertices[0];
            let b = term.vertices[1];
            let canon = (a.min(b), a.max(b));
            let oriented_sign = if a < b {
                term.sign as i8
            } else {
                -(term.sign as i8)
            };
            unique_edges.push(canon);
            raw_incidences.push((f_idx, canon, oriented_sign));
        }
    }

    unique_edges.sort_unstable();
    unique_edges.dedup();
    let e_count = unique_edges.len();

    let mut anchors = make_cell_anchors(geometry_id, f_count, AnchorDomain::Cell2Face);
    anchors.extend(make_cell_anchors(geometry_id, e_count, AnchorDomain::Cell1Edge));

    let mut weighted_edges: Vec<(u32, u32, u32, f32)> = Vec::with_capacity(raw_incidences.len());
    for (f_idx, canon, sign) in raw_incidences {
        let e_idx = unique_edges.binary_search(&canon).unwrap();
        let target_v = (f_count + e_idx) as u32;
        anchors[target_v as usize].orientation_sign = sign;
        weighted_edges.push((f_idx as u32, target_v, e_idx as u32, sign as f32));
    }
    weighted_edges.sort_unstable_by_key(|&(u, v, a, _)| (u, v, a));

    let edges = weighted_edges
        .iter()
        .map(|&(u, v, a, _)| (u, v, a))
        .collect();
    let weights = weighted_edges.iter().map(|&(_, _, _, w)| w).collect();

    Ok(UnsealedGraphSpec {
        graph_id: derive_graph_id(geometry_id, GraphEmbedRole::BoundaryIncidence),
        embed_role: GraphEmbedRole::BoundaryIncidence,
        morphology: GraphMorphology::BipartiteFactor,
        invariants: TopologyInvariants::CHAIN_BOUNDARY_EXACT | TopologyInvariants::ACYCLIC,
        context_and_state_mask: 0,
        anchors,
        edges,
        edge_weights: Some(weights),
    })
}

/// Induce a `CombinatorialDartMap` (`InvolutionMap`, `AnchorDomain::Cell1Edge`) with $\beta_1$ and $\beta_2$ links.
pub fn induce_combinatorial_dart_graph(
    geometry_id: u64,
    half_edges: &[HalfEdge],
    budget_bytes: usize,
) -> Result<UnsealedGraphSpec, TopologyError> {
    let n = half_edges.len();
    let requested = n
        .saturating_mul(core::mem::size_of::<Dart>() + core::mem::size_of::<CellAnchor>() + 24);
    if requested > budget_bytes {
        return Err(TopologyError::BudgetExhausted {
            requested,
            budget: budget_bytes,
        });
    }

    let mut darts = vec![Dart::default(); n];
    half_edges_to_darts(half_edges, &mut darts).map_err(|_| TopologyError::BufferTooSmall)?;
    validate_combinatorial_map(&darts).map_err(|_| TopologyError::CellOutOfRange {
        domain: AnchorDomain::Cell1Edge as u8,
        index: 0,
        count: n as u32,
    })?;

    let mut anchors = Vec::with_capacity(n);
    let mut edges = Vec::with_capacity(n * 2);
    let mut all_twinned = !darts.is_empty();

    for (d_idx, dart) in darts.iter().enumerate() {
        anchors.push(CellAnchor {
            entity_hash: derive_cell_hash(geometry_id, AnchorDomain::Cell1Edge, d_idx as u32),
            cell_index: d_idx as u32,
            anchor_domain: AnchorDomain::Cell1Edge as u8,
            orientation_sign: 1,
            material_or_stratum: 0,
        });
        edges.push((d_idx as u32, dart.beta1, 1));
        if dart.beta2 != INVALID_INDEX {
            edges.push((d_idx as u32, dart.beta2, 2));
        } else {
            all_twinned = false;
        }
    }
    edges.sort_unstable_by_key(|&(u, v, a)| (u, v, a));

    let invariants = if all_twinned {
        TopologyInvariants::MANIFOLD_CLOSED
    } else {
        TopologyInvariants::EMPTY
    };

    Ok(UnsealedGraphSpec {
        graph_id: derive_graph_id(geometry_id, GraphEmbedRole::CombinatorialDartMap),
        embed_role: GraphEmbedRole::CombinatorialDartMap,
        morphology: GraphMorphology::InvolutionMap,
        invariants,
        context_and_state_mask: 0,
        anchors,
        edges,
        edge_weights: None,
    })
}

/// Induce a 10D `MetricProximity` (`SymmetricGraph`) connecting `Tensor10D` nodes within `max_radius`.
pub fn induce_metric_proximity_10d(
    geometry_id: u64,
    points: &[Tensor10D],
    entity_hashes: &[u64],
    max_radius: f32,
    topology_class: TopologyClass,
    budget_bytes: usize,
) -> Result<UnsealedGraphSpec, TopologyError> {
    let n = points.len();
    let base_bytes = n.saturating_mul(core::mem::size_of::<CellAnchor>());
    if base_bytes > budget_bytes {
        return Err(TopologyError::BudgetExhausted {
            requested: base_bytes,
            budget: budget_bytes,
        });
    }

    let mut anchors = Vec::with_capacity(n);
    for i in 0..n {
        let hash = entity_hashes.get(i).copied().unwrap_or(0);
        let (entity_hash, domain) = if hash != 0 {
            (hash, AnchorDomain::SemanticQuin)
        } else {
            (
                derive_cell_hash(geometry_id, AnchorDomain::Cell0Vertex, i as u32),
                AnchorDomain::Cell0Vertex,
            )
        };
        anchors.push(CellAnchor {
            entity_hash,
            cell_index: i as u32,
            anchor_domain: domain as u8,
            orientation_sign: 1,
            material_or_stratum: 0,
        });
    }

    let mut weighted_edges: Vec<(u32, u32, u32, f32)> = Vec::new();
    let mut edge_id = 0u32;
    for i in 0..n {
        let pi = (points[i].x, points[i].y, points[i].z);
        for j in (i + 1)..n {
            let pj = (points[j].x, points[j].y, points[j].z);
            let dist = topology_class.calculate_distance(pi, pj);
            if dist.is_finite() && dist <= max_radius {
                let needed = base_bytes.saturating_add((weighted_edges.len() + 2).saturating_mul(16));
                if needed > budget_bytes {
                    return Err(TopologyError::BudgetExhausted {
                        requested: needed,
                        budget: budget_bytes,
                    });
                }
                weighted_edges.push((i as u32, j as u32, edge_id, dist));
                weighted_edges.push((j as u32, i as u32, edge_id, dist));
                edge_id += 1;
            }
        }
    }
    weighted_edges.sort_unstable_by_key(|&(u, v, a, _)| (u, v, a));

    let edges = weighted_edges
        .iter()
        .map(|&(u, v, a, _)| (u, v, a))
        .collect();
    let weights = weighted_edges.iter().map(|&(_, _, _, w)| w).collect();

    Ok(UnsealedGraphSpec {
        graph_id: derive_graph_id(geometry_id, GraphEmbedRole::MetricProximity),
        embed_role: GraphEmbedRole::MetricProximity,
        morphology: GraphMorphology::SymmetricGraph,
        invariants: TopologyInvariants::SYMMETRIC | TopologyInvariants::SPATIAL_METRIC,
        context_and_state_mask: 0,
        anchors,
        edges,
        edge_weights: Some(weights),
    })
}
