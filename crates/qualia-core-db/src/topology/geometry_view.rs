//! Zero-copy host geometry complex view (`PoetGeometryView`) harbouring $M$ graphs.
//!
//! Tier-1 Zero-Heap (`AGENTS.md` §0 & §0-A): inspects cellular counts ($N_0 \dots N_3$),
//! resolves any of the $M$ harboured graphs in $O(1)$, and executes $O(1)$ cross-graph
//! functorial hops (`hop_via_cell_anchor`) across shared geometric $k$-cells or `NQuin` hashes.

use crate::topology::csr_view::{slice_pod, HarbouredCsrView};
use crate::topology::header::{
    AnchorDomain, GraphEmbedRole, HarbouredGraphHeader, PoetGeometryHeader, TopologyError,
};

/// Zero-copy view over a `GTOP` Geometry-Topology Complex segment harbouring $M$ graphs.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct PoetGeometryView<'a> {
    segment: &'a [u8],
    header: &'a PoetGeometryHeader,
    graph_headers: &'a [HarbouredGraphHeader],
}

impl<'a> PoetGeometryView<'a> {
    /// Construct a `PoetGeometryView` over a 64-byte-aligned `GTOP` segment buffer.
    pub fn from_segment_bytes(segment: &'a [u8]) -> Result<Self, TopologyError> {
        let hdr_size = core::mem::size_of::<PoetGeometryHeader>();
        if segment.len() < hdr_size {
            return Err(TopologyError::BufferTooSmall);
        }
        let header: &'a PoetGeometryHeader = bytemuck::try_from_bytes(&segment[..hdr_size])
            .map_err(|_| TopologyError::UnalignedBuffer)?;

        if header.complex_kind != 0 && header.complex_kind_enum().is_none() {
            return Err(TopologyError::InvalidComplexKind(header.complex_kind));
        }

        let count = header.harboured_graph_count as usize;
        let graph_headers =
            slice_pod::<HarbouredGraphHeader>(segment, header.graph_dir_offset, count)?;

        Ok(Self {
            segment,
            header,
            graph_headers,
        })
    }

    #[inline]
    pub const fn segment(&self) -> &'a [u8] {
        self.segment
    }

    #[inline]
    pub const fn header(&self) -> &'a PoetGeometryHeader {
        self.header
    }

    #[inline]
    pub const fn graph_headers(&self) -> &'a [HarbouredGraphHeader] {
        self.graph_headers
    }

    #[inline]
    pub const fn harboured_graph_count(&self) -> usize {
        self.graph_headers.len()
    }

    /// Resolve the currently active harboured graph (`header.active_graph_slot`) in $O(1)$.
    #[inline]
    pub fn active_graph(&self) -> Result<HarbouredCsrView<'a>, TopologyError> {
        self.graph_at_slot(self.header.active_graph_slot)
    }

    /// Resolve the harboured graph at `slot` ($m \in [0, M)$) in $O(1)$.
    #[inline]
    pub fn graph_at_slot(&self, slot: u16) -> Result<HarbouredCsrView<'a>, TopologyError> {
        let gh =
            self.graph_headers
                .get(slot as usize)
                .ok_or(TopologyError::GraphSlotOutOfRange {
                    slot,
                    count: self.header.harboured_graph_count,
                })?;
        HarbouredCsrView::from_segment_bytes(self.segment, gh)
    }

    /// Resolve a harboured graph by its 64-bit `graph_id` (`q_hash`), returning `(slot, view)`.
    pub fn graph_by_id(
        &self,
        graph_id: u64,
    ) -> Result<(u16, HarbouredCsrView<'a>), TopologyError> {
        let mut idx = 0usize;
        while idx < self.graph_headers.len() {
            let gh = &self.graph_headers[idx];
            if gh.graph_id == graph_id {
                let view = HarbouredCsrView::from_segment_bytes(self.segment, gh)?;
                return Ok((idx as u16, view));
            }
            idx += 1;
        }
        Err(TopologyError::GraphSlotOutOfRange {
            slot: u16::MAX,
            count: self.header.harboured_graph_count,
        })
    }

    /// Resolve the first harboured graph matching `role`, returning `(slot, view)`.
    pub fn first_graph_by_role(
        &self,
        role: GraphEmbedRole,
    ) -> Result<(u16, HarbouredCsrView<'a>), TopologyError> {
        let role_u8 = role as u8;
        let mut idx = 0usize;
        while idx < self.graph_headers.len() {
            let gh = &self.graph_headers[idx];
            if gh.embed_role == role_u8 {
                let view = HarbouredCsrView::from_segment_bytes(self.segment, gh)?;
                return Ok((idx as u16, view));
            }
            idx += 1;
        }
        Err(TopologyError::GraphSlotOutOfRange {
            slot: u16::MAX,
            count: self.header.harboured_graph_count,
        })
    }

    /// Functorial cross-graph hop: reads the `CellAnchor` of `from_vertex` in `from_slot`
    /// and collects all vertices in `to_slot` sharing the same `(anchor_domain, cell_index)`
    /// (or matching `entity_hash` when `anchor_domain == AnchorDomain::SemanticQuin`).
    pub fn hop_via_cell_anchor(
        &self,
        from_slot: u16,
        from_vertex: u32,
        to_slot: u16,
        out_vertices: &mut [u32],
    ) -> Result<usize, TopologyError> {
        let from_graph = self.graph_at_slot(from_slot)?;
        let to_graph = self.graph_at_slot(to_slot)?;
        let src_anchor = from_graph.cell_anchor(from_vertex)?;

        let domain = src_anchor
            .domain()
            .ok_or(TopologyError::CellOutOfRange {
                domain: src_anchor.anchor_domain,
                index: src_anchor.cell_index,
                count: 0,
            })?;

        if domain == AnchorDomain::SemanticQuin {
            if src_anchor.entity_hash == 0 {
                return Ok(to_graph.find_vertices_by_cell(
                    domain,
                    src_anchor.cell_index,
                    out_vertices,
                ));
            }
            return Ok(collect_by_entity_hash(
                to_graph.anchors(),
                src_anchor.entity_hash,
                out_vertices,
            ));
        }

        let written =
            to_graph.find_vertices_by_cell(domain, src_anchor.cell_index, out_vertices);
        if written == 0 && src_anchor.entity_hash != 0 {
            return Ok(collect_by_entity_hash(
                to_graph.anchors(),
                src_anchor.entity_hash,
                out_vertices,
            ));
        }
        Ok(written)
    }
}

#[inline]
fn collect_by_entity_hash(
    anchors: &[crate::topology::header::CellAnchor],
    entity_hash: u64,
    out_vertices: &mut [u32],
) -> usize {
    let mut written = 0usize;
    let mut v = 0usize;
    while v < anchors.len() {
        if anchors[v].entity_hash == entity_hash {
            if written >= out_vertices.len() {
                break;
            }
            out_vertices[written] = v as u32;
            written += 1;
        }
        v += 1;
    }
    written
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::topology::header::{
        CellAnchor, GeometryComplexKind, GraphMorphology, TopologyInvariants,
    };

    #[repr(C, align(64))]
    struct AlignedSegment([u8; 512]);

    #[test]
    fn geometry_view_cross_graph_hop_via_cell_anchor() {
        let mut seg = AlignedSegment([0u8; 512]);

        // Layout in 512B segment:
        // [0..64]:    PoetGeometryHeader
        // [64..128]:  HarbouredGraphHeader[0] (PoincareDual on Cell2Face)
        // [128..192]: HarbouredGraphHeader[1] (DeonticGovernance on Cell2Face)
        // [192..224]: Graph 0 anchors (2 * 16B = 32B)
        // [224..256]: Graph 1 anchors (2 * 16B = 32B)
        // [256..320]: Shared CSR/CSC arrays for 2 vertices, 1 edge (0 -> 1)
        let geom_hdr = PoetGeometryHeader {
            geometry_id: 0xABCD,
            construct_schema_hash: 0x1234,
            manifold_id: 1,
            cell_0_count: 4,
            cell_1_count: 5,
            cell_2_count: 2,
            cell_3_count: 0,
            geometry_payload_offset: 0,
            ddg_operators_offset: 0,
            graph_dir_offset: 64,
            harboured_graph_count: 2,
            active_graph_slot: 0,
            euler_characteristic: 1,
            complex_kind: GeometryComplexKind::SurfaceManifold2D as u8,
            metric_kind: 0,
            ambient_dim: 3,
            intrinsic_dim: 2,
            crc32c: 0,
        };
        let g0 = HarbouredGraphHeader {
            graph_id: 10,
            parent_geometry_id: 0xABCD,
            anchor_table_offset: 192,
            csr_row_ptr_offset: 256,
            csr_col_idx_offset: 268,
            edge_anchor_offset: 272,
            csc_col_ptr_offset: 276,
            csc_row_idx_offset: 288,
            edge_weight_offset: 0,
            vertex_count: 2,
            edge_count: 1,
            context_and_state_mask: 0,
            embed_role: GraphEmbedRole::PoincareDual as u8,
            morphology: GraphMorphology::SymmetricGraph as u8,
            invariants: TopologyInvariants::EMPTY,
            crc32c: 0,
        };
        let mut g1 = g0;
        g1.graph_id = 20;
        g1.anchor_table_offset = 224;
        g1.embed_role = GraphEmbedRole::DeonticGovernance as u8;

        let g0_anchors = [
            CellAnchor {
                entity_hash: 0,
                cell_index: 42,
                anchor_domain: AnchorDomain::Cell2Face as u8,
                orientation_sign: 1,
                material_or_stratum: 0,
            },
            CellAnchor {
                entity_hash: 0,
                cell_index: 43,
                anchor_domain: AnchorDomain::Cell2Face as u8,
                orientation_sign: 1,
                material_or_stratum: 0,
            },
        ];
        let g1_anchors = [
            CellAnchor {
                entity_hash: 999,
                cell_index: 43,
                anchor_domain: AnchorDomain::Cell2Face as u8,
                orientation_sign: 1,
                material_or_stratum: 1,
            },
            CellAnchor {
                entity_hash: 888,
                cell_index: 42,
                anchor_domain: AnchorDomain::Cell2Face as u8,
                orientation_sign: 1,
                material_or_stratum: 1,
            },
        ];
        let row_ptr = [0u32, 1, 1];
        let col_idx = [1u32];
        let edge_anc = [0u32];
        let csc_ptr = [0u32, 0, 1];
        let csc_row = [0u32];

        seg.0[0..64].copy_from_slice(bytemuck::bytes_of(&geom_hdr));
        seg.0[64..128].copy_from_slice(bytemuck::bytes_of(&g0));
        seg.0[128..192].copy_from_slice(bytemuck::bytes_of(&g1));
        seg.0[192..224].copy_from_slice(bytemuck::cast_slice(&g0_anchors));
        seg.0[224..256].copy_from_slice(bytemuck::cast_slice(&g1_anchors));
        seg.0[256..268].copy_from_slice(bytemuck::cast_slice(&row_ptr));
        seg.0[268..272].copy_from_slice(bytemuck::cast_slice(&col_idx));
        seg.0[272..276].copy_from_slice(bytemuck::cast_slice(&edge_anc));
        seg.0[276..288].copy_from_slice(bytemuck::cast_slice(&csc_ptr));
        seg.0[288..292].copy_from_slice(bytemuck::cast_slice(&csc_row));

        let view = PoetGeometryView::from_segment_bytes(&seg.0).unwrap();
        assert_eq!(view.harboured_graph_count(), 2);
        assert_eq!(view.active_graph().unwrap().header().graph_id, 10);
        assert_eq!(view.graph_by_id(20).unwrap().0, 1);
        assert_eq!(
            view.first_graph_by_role(GraphEmbedRole::DeonticGovernance)
                .unwrap()
                .0,
            1
        );

        let mut out = [0u32; 4];
        let n = view.hop_via_cell_anchor(0, 0, 1, &mut out).unwrap();
        assert_eq!(n, 1);
        assert_eq!(out[0], 1); // vertex 0 in g0 (Cell2Face #42) -> vertex 1 in g1 (Cell2Face #42)
    }
}
