//! Tier-2 cold `GTOP` segment compiler and delta-ring compaction engine.
//!
//! Tier-2 Cold Bounded (`AGENTS.md` §0-A): [`compile_geometry_segment`] checks the caller's
//! `budget_bytes` before allocating scratch buffers and emits a 64-byte-aligned `GTOP` binary
//! segment directly into the caller's `out_buf` with Castagnoli CRC-32C receipts.

use crate::container_10d::crc32c::{crc32c_update, crc32c};
use crate::topology::delta_ring::{DeltaOpKind, TopologicalDeltaEntry};
use crate::topology::header::{
    CellAnchor, GeometryComplexKind, GeometryMetricKind, GraphEmbedRole, GraphMorphology,
    HarbouredGraphHeader, PoetGeometryHeader, TopologyError, TopologyInvariants,
};

/// Unsealed specification of one harboured graph prior to CSR/CSC compaction.
#[derive(Clone, Debug, PartialEq)]
pub struct UnsealedGraphSpec {
    pub graph_id: u64,
    pub embed_role: GraphEmbedRole,
    pub morphology: GraphMorphology,
    pub invariants: TopologyInvariants,
    pub context_and_state_mask: u32,
    pub anchors: Vec<CellAnchor>,
    /// Directed edges `(source_v, target_v, edge_anchor_idx)`.
    pub edges: Vec<(u32, u32, u32)>,
    pub edge_weights: Option<Vec<f32>>,
}

/// Unsealed specification of a host Geometry Complex and its $M$ harboured graphs.
#[derive(Clone, Debug, PartialEq)]
pub struct UnsealedGeometrySpec {
    pub geometry_id: u64,
    pub construct_schema_hash: u64,
    pub manifold_id: u32,
    pub complex_kind: GeometryComplexKind,
    pub metric_kind: GeometryMetricKind,
    pub ambient_dim: u8,
    pub intrinsic_dim: u8,
    /// Cellular counts `[N_0, N_1, N_2, N_3]`.
    pub cell_counts: [u32; 4],
    pub active_graph_slot: u16,
    pub graphs: Vec<UnsealedGraphSpec>,
}

#[inline]
const fn align64(offset: usize) -> usize {
    (offset + 63) & !63
}

/// Compute the exact 64-byte-aligned byte length required to encode `spec` into a `GTOP` segment.
pub fn required_geometry_segment_bytes(spec: &UnsealedGeometrySpec) -> Result<usize, TopologyError> {
    let m = spec.graphs.len();
    let mut cursor = 64usize
        .checked_add(m.checked_mul(64).ok_or(TopologyError::BufferTooSmall)?)
        .ok_or(TopologyError::BufferTooSmall)?;

    for g in &spec.graphs {
        let v = g.anchors.len();
        let e = g.edges.len();
        let anchors_bytes = v.checked_mul(16).ok_or(TopologyError::BufferTooSmall)?;
        let ptr_bytes = (v + 1).checked_mul(4).ok_or(TopologyError::BufferTooSmall)?;
        let edge_u32_bytes = e.checked_mul(4).ok_or(TopologyError::BufferTooSmall)?;

        for section_len in [
            anchors_bytes,
            ptr_bytes,
            edge_u32_bytes,
            edge_u32_bytes,
            ptr_bytes,
            edge_u32_bytes,
        ] {
            cursor = align64(cursor)
                .checked_add(align64(section_len))
                .ok_or(TopologyError::BufferTooSmall)?;
        }
        if g.edge_weights.is_some() {
            cursor = align64(cursor)
                .checked_add(align64(edge_u32_bytes))
                .ok_or(TopologyError::BufferTooSmall)?;
        }
    }
    Ok(align64(cursor))
}

/// Compile `spec` into a 64-byte-aligned `GTOP` binary segment inside `out_buf`.
///
/// Fails closed with [`TopologyError::BudgetExhausted`] if the segment + scratch bytes exceed
/// `budget_bytes`, and with [`TopologyError::BufferTooSmall`] if `out_buf` cannot hold the segment.
pub fn compile_geometry_segment(
    spec: &UnsealedGeometrySpec,
    budget_bytes: usize,
    out_buf: &mut [u8],
) -> Result<usize, TopologyError> {
    if !spec.graphs.is_empty() && (spec.active_graph_slot as usize) >= spec.graphs.len() {
        return Err(TopologyError::GraphSlotOutOfRange {
            slot: spec.active_graph_slot,
            count: spec.graphs.len() as u16,
        });
    }

    let total_bytes = required_geometry_segment_bytes(spec)?;
    let max_scratch = spec
        .graphs
        .iter()
        .map(|g| (g.anchors.len() + 1) * 8 + g.edges.len() * 32)
        .max()
        .unwrap_or(0);
    let requested = total_bytes.saturating_add(max_scratch);
    if requested > budget_bytes {
        return Err(TopologyError::BudgetExhausted {
            requested,
            budget: budget_bytes,
        });
    }
    if out_buf.len() < total_bytes {
        return Err(TopologyError::BufferTooSmall);
    }

    for g in &spec.graphs {
        let v_count = g.anchors.len() as u32;
        if let Some(ref w) = g.edge_weights {
            if w.len() != g.edges.len() {
                return Err(TopologyError::BufferTooSmall);
            }
        }
        for &(u, v, _) in &g.edges {
            if u >= v_count {
                return Err(TopologyError::VertexOutOfRange {
                    vertex: u,
                    count: v_count,
                });
            }
            if v >= v_count {
                return Err(TopologyError::VertexOutOfRange {
                    vertex: v,
                    count: v_count,
                });
            }
        }
    }

    out_buf[..total_bytes].fill(0);

    let m = spec.graphs.len();
    let graph_dir_offset = 64usize;
    let mut cursor = align64(graph_dir_offset + m * 64);

    for (slot, g) in spec.graphs.iter().enumerate() {
        let v_count = g.anchors.len();
        let e_count = g.edges.len();

        // Sort CSR edges by (u, v, edge_anchor) carrying optional weight.
        let mut csr_tuples: Vec<(u32, u32, u32, f32)> = Vec::with_capacity(e_count);
        for (idx, &(u, v, anc)) in g.edges.iter().enumerate() {
            let w = g.edge_weights.as_ref().map_or(0.0, |ws| ws[idx]);
            csr_tuples.push((u, v, anc, w));
        }
        csr_tuples.sort_unstable_by(|a, b| (a.0, a.1, a.2).cmp(&(b.0, b.1, b.2)));

        let mut csr_row_ptr = vec![0u32; v_count + 1];
        let mut csr_col_idx = Vec::with_capacity(e_count);
        let mut edge_anchor = Vec::with_capacity(e_count);
        let mut sorted_weights = Vec::with_capacity(if g.edge_weights.is_some() { e_count } else { 0 });

        for &(u, v, anc, w) in &csr_tuples {
            csr_row_ptr[u as usize + 1] += 1;
            csr_col_idx.push(v);
            edge_anchor.push(anc);
            if g.edge_weights.is_some() {
                sorted_weights.push(w);
            }
        }
        for u in 0..v_count {
            csr_row_ptr[u + 1] += csr_row_ptr[u];
        }

        // Sort CSC edges by (v, u) for backward incoming lookups.
        let mut csc_tuples: Vec<(u32, u32)> = csr_tuples.iter().map(|&(u, v, _, _)| (v, u)).collect();
        csc_tuples.sort_unstable();

        let mut csc_col_ptr = vec![0u32; v_count + 1];
        let mut csc_row_idx = Vec::with_capacity(e_count);
        for &(v, u) in &csc_tuples {
            csc_col_ptr[v as usize + 1] += 1;
            csc_row_idx.push(u);
        }
        for v in 0..v_count {
            csc_col_ptr[v + 1] += csc_col_ptr[v];
        }

        let anchor_table_offset = align64(cursor);
        let anchor_bytes: &[u8] = bytemuck::cast_slice(&g.anchors);
        out_buf[anchor_table_offset..anchor_table_offset + anchor_bytes.len()]
            .copy_from_slice(anchor_bytes);
        cursor = align64(anchor_table_offset + align64(anchor_bytes.len()));

        let csr_row_ptr_offset = align64(cursor);
        let row_ptr_bytes: &[u8] = bytemuck::cast_slice(&csr_row_ptr);
        out_buf[csr_row_ptr_offset..csr_row_ptr_offset + row_ptr_bytes.len()]
            .copy_from_slice(row_ptr_bytes);
        cursor = align64(csr_row_ptr_offset + align64(row_ptr_bytes.len()));

        let csr_col_idx_offset = align64(cursor);
        let col_idx_bytes: &[u8] = bytemuck::cast_slice(&csr_col_idx);
        out_buf[csr_col_idx_offset..csr_col_idx_offset + col_idx_bytes.len()]
            .copy_from_slice(col_idx_bytes);
        cursor = align64(csr_col_idx_offset + align64(col_idx_bytes.len()));

        let edge_anchor_offset = align64(cursor);
        let edge_anc_bytes: &[u8] = bytemuck::cast_slice(&edge_anchor);
        out_buf[edge_anchor_offset..edge_anchor_offset + edge_anc_bytes.len()]
            .copy_from_slice(edge_anc_bytes);
        cursor = align64(edge_anchor_offset + align64(edge_anc_bytes.len()));

        let csc_col_ptr_offset = align64(cursor);
        let csc_ptr_bytes: &[u8] = bytemuck::cast_slice(&csc_col_ptr);
        out_buf[csc_col_ptr_offset..csc_col_ptr_offset + csc_ptr_bytes.len()]
            .copy_from_slice(csc_ptr_bytes);
        cursor = align64(csc_col_ptr_offset + align64(csc_ptr_bytes.len()));

        let csc_row_idx_offset = align64(cursor);
        let csc_row_bytes: &[u8] = bytemuck::cast_slice(&csc_row_idx);
        out_buf[csc_row_idx_offset..csc_row_idx_offset + csc_row_bytes.len()]
            .copy_from_slice(csc_row_bytes);
        cursor = align64(csc_row_idx_offset + align64(csc_row_bytes.len()));

        let edge_weight_offset = if g.edge_weights.is_some() {
            let off = align64(cursor);
            let w_bytes: &[u8] = bytemuck::cast_slice(&sorted_weights);
            out_buf[off..off + w_bytes.len()].copy_from_slice(w_bytes);
            cursor = align64(off + align64(w_bytes.len()));
            off as u32
        } else {
            0
        };

        let mut gh = HarbouredGraphHeader {
            graph_id: g.graph_id,
            parent_geometry_id: spec.geometry_id,
            anchor_table_offset: anchor_table_offset as u32,
            csr_row_ptr_offset: csr_row_ptr_offset as u32,
            csr_col_idx_offset: csr_col_idx_offset as u32,
            edge_anchor_offset: edge_anchor_offset as u32,
            csc_col_ptr_offset: csc_col_ptr_offset as u32,
            csc_row_idx_offset: csc_row_idx_offset as u32,
            edge_weight_offset,
            vertex_count: v_count as u32,
            edge_count: e_count as u32,
            context_and_state_mask: g.context_and_state_mask,
            embed_role: g.embed_role as u8,
            morphology: g.morphology as u8,
            invariants: g.invariants,
            crc32c: 0,
        };
        let gh_bytes = bytemuck::bytes_of(&gh);
        let mut crc_state = crc32c_update(0xFFFF_FFFF, &gh_bytes[..60]);
        crc_state = crc32c_update(crc_state, anchor_bytes);
        crc_state = crc32c_update(crc_state, row_ptr_bytes);
        crc_state = crc32c_update(crc_state, col_idx_bytes);
        crc_state = crc32c_update(crc_state, edge_anc_bytes);
        crc_state = crc32c_update(crc_state, csc_ptr_bytes);
        crc_state = crc32c_update(crc_state, csc_row_bytes);
        if g.edge_weights.is_some() {
            crc_state = crc32c_update(crc_state, bytemuck::cast_slice(&sorted_weights));
        }
        gh.crc32c = !crc_state;

        let hdr_off = graph_dir_offset + slot * 64;
        out_buf[hdr_off..hdr_off + 64].copy_from_slice(bytemuck::bytes_of(&gh));
    }

    let euler = (spec.cell_counts[0] as i32)
        .wrapping_sub(spec.cell_counts[1] as i32)
        .wrapping_add(spec.cell_counts[2] as i32)
        .wrapping_sub(spec.cell_counts[3] as i32);

    let mut geom_hdr = PoetGeometryHeader {
        geometry_id: spec.geometry_id,
        construct_schema_hash: spec.construct_schema_hash,
        manifold_id: spec.manifold_id,
        cell_0_count: spec.cell_counts[0],
        cell_1_count: spec.cell_counts[1],
        cell_2_count: spec.cell_counts[2],
        cell_3_count: spec.cell_counts[3],
        geometry_payload_offset: 0,
        ddg_operators_offset: 0,
        graph_dir_offset: graph_dir_offset as u32,
        harboured_graph_count: m as u16,
        active_graph_slot: spec.active_graph_slot,
        euler_characteristic: euler,
        complex_kind: spec.complex_kind as u8,
        metric_kind: spec.metric_kind as u8,
        ambient_dim: spec.ambient_dim,
        intrinsic_dim: spec.intrinsic_dim,
        crc32c: 0,
    };
    let geom_bytes = bytemuck::bytes_of(&geom_hdr);
    let mut state = crc32c_update(0xFFFF_FFFF, &geom_bytes[..60]);
    state = crc32c_update(state, &out_buf[64..total_bytes]);
    geom_hdr.crc32c = !state;
    out_buf[0..64].copy_from_slice(bytemuck::bytes_of(&geom_hdr));

    let _ = crc32c;
    Ok(total_bytes)
}

/// Apply live `TopologicalDeltaEntry` mutations targeting `slot` onto `base_spec` and deduplicate.
pub fn seal_harboured_graph_deltas(
    base_spec: &mut UnsealedGraphSpec,
    deltas: &[TopologicalDeltaEntry],
    slot: u16,
) {
    let mut slot_deltas: Vec<TopologicalDeltaEntry> = deltas
        .iter()
        .copied()
        .filter(|d| d.graph_slot() as u16 == slot)
        .collect();
    slot_deltas.sort_unstable_by_key(|d| d.seq_no);

    for d in slot_deltas {
        match d.op_kind() {
            DeltaOpKind::Tombstone => {
                let mut i = 0usize;
                while i < base_spec.edges.len() {
                    if base_spec.edges[i].0 == d.source_vertex
                        && base_spec.edges[i].1 == d.target_vertex
                    {
                        base_spec.edges.remove(i);
                        if let Some(ref mut ws) = base_spec.edge_weights {
                            if i < ws.len() {
                                ws.remove(i);
                            }
                        }
                    } else {
                        i += 1;
                    }
                }
            }
            DeltaOpKind::Insert | DeltaOpKind::Reify => {
                if d.op_kind() == DeltaOpKind::Reify {
                    base_spec.invariants |= TopologyInvariants::REIFIED_EDGES;
                }
                if let Some(pos) = base_spec
                    .edges
                    .iter()
                    .position(|&(u, v, _)| u == d.source_vertex && v == d.target_vertex)
                {
                    base_spec.edges[pos].2 = d.edge_anchor_idx;
                } else {
                    base_spec
                        .edges
                        .push((d.source_vertex, d.target_vertex, d.edge_anchor_idx));
                    if let Some(ref mut ws) = base_spec.edge_weights {
                        ws.push(1.0);
                    }
                }
            }
            DeltaOpKind::AnchorBind => {
                if let Some(anchor) = base_spec.anchors.get_mut(d.source_vertex as usize) {
                    anchor.cell_index = d.target_vertex;
                    anchor.anchor_domain = d.anchor_domain() as u8;
                }
            }
        }
    }

    if let Some(ref mut ws) = base_spec.edge_weights {
        let mut combined: Vec<((u32, u32, u32), f32)> = base_spec
            .edges
            .iter()
            .copied()
            .zip(ws.iter().copied())
            .collect();
        combined.sort_unstable_by(|a, b| a.0.cmp(&b.0));
        combined.dedup_by(|a, b| a.0 .0 == b.0 .0 && a.0 .1 == b.0 .1);
        base_spec.edges = combined.iter().map(|&(e, _)| e).collect();
        *ws = combined.iter().map(|&(_, w)| w).collect();
    } else {
        base_spec.edges.sort_unstable();
        base_spec.edges.dedup_by(|a, b| a.0 == b.0 && a.1 == b.1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::specialized_libs::computational_geometry::{
        build_triangle_half_edges, required_edge_slots, EdgeSlot, HalfEdge,
    };
    use crate::topology::delta_ring::TopologicalDeltaRing;
    use crate::topology::geometry_view::PoetGeometryView;
    use crate::topology::header::AnchorDomain;
    use crate::topology::induce_intrinsic::{
        induce_combinatorial_dart_graph, induce_primal_and_dual_from_half_edges,
        induce_simplicial_boundary_incidence,
    };
    use crate::topology::invariants::verify_geometry_complex_homology;

    #[repr(C, align(64))]
    struct AlignedBuf([u8; 8192]);

    #[test]
    fn induce_and_compile_multi_graph_geometry_segment_roundtrip() {
        let triangles = [[0u32, 1, 2], [2, 1, 3]];
        let mut half_edges = [HalfEdge::default(); 6];
        let mut slots = vec![EdgeSlot::default(); required_edge_slots(triangles.len())];
        build_triangle_half_edges(4, &triangles, &mut half_edges, &mut slots).unwrap();

        let budget = 64 * 1024;
        let (mut primal, dual) =
            induce_primal_and_dual_from_half_edges(0x9001, &half_edges, 4, 2, budget).unwrap();
        let incidence =
            induce_simplicial_boundary_incidence(0x9001, &triangles, 4, budget).unwrap();
        let darts = induce_combinatorial_dart_graph(0x9001, &half_edges, budget).unwrap();

        // Seal a delta on primal graph (slot 0)
        let mut ring: TopologicalDeltaRing<8> = TopologicalDeltaRing::new();
        ring.insert_edge(0, 0, 3, 99).unwrap();
        ring.bind_anchor(0, 3, 3, AnchorDomain::Cell0Vertex)
            .unwrap();
        seal_harboured_graph_deltas(&mut primal, ring.entries(), 0);

        let geom_spec = UnsealedGeometrySpec {
            geometry_id: 0x9001,
            construct_schema_hash: 0xABCD,
            manifold_id: 3,
            complex_kind: GeometryComplexKind::SurfaceManifold2D,
            metric_kind: GeometryMetricKind::Euclidean,
            ambient_dim: 3,
            intrinsic_dim: 2,
            cell_counts: [4, 5, 2, 0],
            active_graph_slot: 0,
            graphs: vec![primal, dual, incidence, darts],
        };

        // Tight budget fails closed with BudgetExhausted
        let mut out = AlignedBuf([0u8; 8192]);
        assert!(matches!(
            compile_geometry_segment(&geom_spec, 128, &mut out.0),
            Err(TopologyError::BudgetExhausted { .. })
        ));

        let written = compile_geometry_segment(&geom_spec, budget, &mut out.0).unwrap();
        let view = PoetGeometryView::from_segment_bytes(&out.0[..written]).unwrap();
        assert_eq!(view.harboured_graph_count(), 4);
        assert_eq!(verify_geometry_complex_homology(&view).unwrap(), 1);

        let g0 = view.graph_at_slot(0).unwrap();
        assert!(g0.out_neighbors(0).unwrap().contains(&3));
        assert_ne!(g0.header().crc32c, 0);
        assert_ne!(view.header().crc32c, 0);
    }
}
