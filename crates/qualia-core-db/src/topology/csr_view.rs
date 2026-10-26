//! Zero-copy CSR/CSC and `CellAnchor` view for a `HarbouredGraphHeader`.
//!
//! Tier-1 Zero-Heap (`AGENTS.md` §0 & §0-A): all operations borrow directly from
//! the caller's byte segment or slice arrays and write into caller-supplied `&mut [u32]` buffers.

use bytemuck::Pod;

use crate::topology::header::{
    AnchorDomain, CellAnchor, HarbouredGraphHeader, TopologyError, TopologyInvariants,
};

/// Zero-copy view over a single harboured graph's `CellAnchor`, forward CSR,
/// backward CSC, and optional `f32` edge weight tables.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct HarbouredCsrView<'a> {
    header: &'a HarbouredGraphHeader,
    anchors: &'a [CellAnchor],
    csr_row_ptr: &'a [u32],
    csr_col_idx: &'a [u32],
    edge_anchor: &'a [u32],
    csc_col_ptr: &'a [u32],
    csc_row_idx: &'a [u32],
    edge_weights: Option<&'a [f32]>,
}

impl<'a> HarbouredCsrView<'a> {
    /// Construct and validate a `HarbouredCsrView` from pre-sliced tables.
    #[allow(clippy::too_many_arguments)]
    pub fn from_slices(
        header: &'a HarbouredGraphHeader,
        anchors: &'a [CellAnchor],
        csr_row_ptr: &'a [u32],
        csr_col_idx: &'a [u32],
        edge_anchor: &'a [u32],
        csc_col_ptr: &'a [u32],
        csc_row_idx: &'a [u32],
        edge_weights: Option<&'a [f32]>,
    ) -> Result<Self, TopologyError> {
        if header.embed_role != 0 && header.embed_role_enum().is_none() {
            return Err(TopologyError::InvalidEmbedRole(header.embed_role));
        }
        if header.morphology != 0 && header.morphology_enum().is_none() {
            return Err(TopologyError::InvalidMorphology(header.morphology));
        }

        let v_count = header.vertex_count as usize;
        let e_count = header.edge_count as usize;

        if anchors.len() != v_count
            || csr_row_ptr.len() != v_count + 1
            || csr_col_idx.len() != e_count
            || edge_anchor.len() != e_count
            || csc_col_ptr.len() != v_count + 1
            || csc_row_idx.len() != e_count
        {
            return Err(TopologyError::BufferTooSmall);
        }

        if let Some(weights) = edge_weights {
            if weights.len() != e_count {
                return Err(TopologyError::BufferTooSmall);
            }
        }

        if csr_row_ptr[0] != 0 {
            return Err(TopologyError::NonMonotonicRowPtr { vertex: 0 });
        }
        for u in 0..v_count {
            if csr_row_ptr[u] > csr_row_ptr[u + 1] {
                return Err(TopologyError::NonMonotonicRowPtr { vertex: u as u32 });
            }
        }
        if csr_row_ptr[v_count] != header.edge_count {
            return Err(TopologyError::NonMonotonicRowPtr {
                vertex: header.vertex_count,
            });
        }

        if csc_col_ptr[0] != 0 {
            return Err(TopologyError::NonMonotonicRowPtr { vertex: 0 });
        }
        for v in 0..v_count {
            if csc_col_ptr[v] > csc_col_ptr[v + 1] {
                return Err(TopologyError::NonMonotonicRowPtr { vertex: v as u32 });
            }
        }
        if csc_col_ptr[v_count] != header.edge_count {
            return Err(TopologyError::NonMonotonicRowPtr {
                vertex: header.vertex_count,
            });
        }

        for &col in csr_col_idx {
            if col >= header.vertex_count {
                return Err(TopologyError::VertexOutOfRange {
                    vertex: col,
                    count: header.vertex_count,
                });
            }
        }

        for &row in csc_row_idx {
            if row >= header.vertex_count {
                return Err(TopologyError::VertexOutOfRange {
                    vertex: row,
                    count: header.vertex_count,
                });
            }
        }

        Ok(Self {
            header,
            anchors,
            csr_row_ptr,
            csr_col_idx,
            edge_anchor,
            csc_col_ptr,
            csc_row_idx,
            edge_weights,
        })
    }

    /// Construct a `HarbouredCsrView` by slicing a `GTOP` segment at the offsets in `header`.
    pub fn from_segment_bytes(
        segment: &'a [u8],
        header: &'a HarbouredGraphHeader,
    ) -> Result<Self, TopologyError> {
        let v_count = header.vertex_count as usize;
        let e_count = header.edge_count as usize;

        let anchors = slice_pod::<CellAnchor>(segment, header.anchor_table_offset, v_count)?;
        let csr_row_ptr = slice_pod::<u32>(segment, header.csr_row_ptr_offset, v_count + 1)?;
        let csr_col_idx = slice_pod::<u32>(segment, header.csr_col_idx_offset, e_count)?;
        let edge_anchor = slice_pod::<u32>(segment, header.edge_anchor_offset, e_count)?;
        let csc_col_ptr = slice_pod::<u32>(segment, header.csc_col_ptr_offset, v_count + 1)?;
        let csc_row_idx = slice_pod::<u32>(segment, header.csc_row_idx_offset, e_count)?;
        let edge_weights = if header.edge_weight_offset != 0 {
            Some(slice_pod::<f32>(
                segment,
                header.edge_weight_offset,
                e_count,
            )?)
        } else {
            None
        };

        Self::from_slices(
            header,
            anchors,
            csr_row_ptr,
            csr_col_idx,
            edge_anchor,
            csc_col_ptr,
            csc_row_idx,
            edge_weights,
        )
    }

    #[inline]
    pub const fn header(&self) -> &'a HarbouredGraphHeader {
        self.header
    }

    #[inline]
    pub const fn vertex_count(&self) -> u32 {
        self.header.vertex_count
    }

    #[inline]
    pub const fn edge_count(&self) -> u32 {
        self.header.edge_count
    }

    #[inline]
    pub const fn anchors(&self) -> &'a [CellAnchor] {
        self.anchors
    }

    #[inline]
    pub const fn csr_row_ptr(&self) -> &'a [u32] {
        self.csr_row_ptr
    }

    #[inline]
    pub const fn csr_col_idx(&self) -> &'a [u32] {
        self.csr_col_idx
    }

    #[inline]
    pub const fn edge_anchor(&self) -> &'a [u32] {
        self.edge_anchor
    }

    #[inline]
    pub const fn edge_anchors(&self) -> &'a [u32] {
        self.edge_anchor
    }

    #[inline]
    pub const fn csc_col_ptr(&self) -> &'a [u32] {
        self.csc_col_ptr
    }

    #[inline]
    pub const fn csc_row_idx(&self) -> &'a [u32] {
        self.csc_row_idx
    }

    #[inline]
    pub const fn edge_weights(&self) -> Option<&'a [f32]> {
        self.edge_weights
    }

    /// Look up the 16-byte `CellAnchor` for vertex `v`.
    #[inline]
    pub fn cell_anchor(&self, v: u32) -> Result<&'a CellAnchor, TopologyError> {
        self.anchors
            .get(v as usize)
            .ok_or(TopologyError::VertexOutOfRange {
                vertex: v,
                count: self.header.vertex_count,
            })
    }

    /// Outgoing neighbour slice for vertex `u` in $O(1)$.
    #[inline]
    pub fn out_neighbors(&self, u: u32) -> Result<&'a [u32], TopologyError> {
        if u >= self.header.vertex_count {
            return Err(TopologyError::VertexOutOfRange {
                vertex: u,
                count: self.header.vertex_count,
            });
        }
        let idx = u as usize;
        let start = self.csr_row_ptr[idx] as usize;
        let end = self.csr_row_ptr[idx + 1] as usize;
        Ok(&self.csr_col_idx[start..end])
    }

    /// Outgoing edge anchor indices (1-cell ID or `NQuin` slot) for vertex `u` in $O(1)$.
    #[inline]
    pub fn out_edge_anchors(&self, u: u32) -> Result<&'a [u32], TopologyError> {
        if u >= self.header.vertex_count {
            return Err(TopologyError::VertexOutOfRange {
                vertex: u,
                count: self.header.vertex_count,
            });
        }
        let idx = u as usize;
        let start = self.csr_row_ptr[idx] as usize;
        let end = self.csr_row_ptr[idx + 1] as usize;
        Ok(&self.edge_anchor[start..end])
    }

    /// Optional outgoing edge weights for vertex `u` in $O(1)$.
    #[inline]
    pub fn out_edge_weights(&self, u: u32) -> Result<Option<&'a [f32]>, TopologyError> {
        if u >= self.header.vertex_count {
            return Err(TopologyError::VertexOutOfRange {
                vertex: u,
                count: self.header.vertex_count,
            });
        }
        let idx = u as usize;
        let start = self.csr_row_ptr[idx] as usize;
        let end = self.csr_row_ptr[idx + 1] as usize;
        Ok(self.edge_weights.map(|w| &w[start..end]))
    }

    /// Incoming neighbour slice for vertex `v` in $O(1)$ via backward CSC.
    #[inline]
    pub fn in_neighbors(&self, v: u32) -> Result<&'a [u32], TopologyError> {
        if v >= self.header.vertex_count {
            return Err(TopologyError::VertexOutOfRange {
                vertex: v,
                count: self.header.vertex_count,
            });
        }
        let idx = v as usize;
        let start = self.csc_col_ptr[idx] as usize;
        let end = self.csc_col_ptr[idx + 1] as usize;
        Ok(&self.csc_row_idx[start..end])
    }

    /// Out-degree of vertex `u` in $O(1)$.
    #[inline]
    pub fn out_degree(&self, u: u32) -> Result<u32, TopologyError> {
        if u >= self.header.vertex_count {
            return Err(TopologyError::VertexOutOfRange {
                vertex: u,
                count: self.header.vertex_count,
            });
        }
        let idx = u as usize;
        Ok(self.csr_row_ptr[idx + 1] - self.csr_row_ptr[idx])
    }

    /// In-degree of vertex `v` in $O(1)$ via backward CSC.
    #[inline]
    pub fn in_degree(&self, v: u32) -> Result<u32, TopologyError> {
        if v >= self.header.vertex_count {
            return Err(TopologyError::VertexOutOfRange {
                vertex: v,
                count: self.header.vertex_count,
            });
        }
        let idx = v as usize;
        Ok(self.csc_col_ptr[idx + 1] - self.csc_col_ptr[idx])
    }

    /// Find vertex index by 64-bit `entity_hash`.
    /// Uses binary search when sorted by `entity_hash` and falls back to a deterministic linear scan.
    pub fn find_vertex_by_entity_hash(&self, entity_hash: u64) -> Option<u32> {
        if let Ok(idx) = self
            .anchors
            .binary_search_by_key(&entity_hash, |a| a.entity_hash)
        {
            return Some(idx as u32);
        }
        let mut i = 0usize;
        while i < self.anchors.len() {
            if self.anchors[i].entity_hash == entity_hash {
                return Some(i as u32);
            }
            i += 1;
        }
        None
    }

    /// Collect all vertices anchored to `(domain, cell_index)` into caller buffer `out`.
    pub fn find_vertices_by_cell(
        &self,
        domain: AnchorDomain,
        cell_index: u32,
        out: &mut [u32],
    ) -> usize {
        let domain_u8 = domain as u8;
        let mut written = 0usize;
        let mut v = 0usize;
        while v < self.anchors.len() {
            let a = &self.anchors[v];
            if a.anchor_domain == domain_u8 && a.cell_index == cell_index {
                if written >= out.len() {
                    break;
                }
                out[written] = v as u32;
                written += 1;
            }
            v += 1;
        }
        written
    }

    /// Collect all source/root vertices (`in_degree(v) == 0`) into `out`.
    pub fn collect_roots(&self, out: &mut [u32]) -> usize {
        let mut written = 0usize;
        let v_count = self.header.vertex_count as usize;
        let mut v = 0usize;
        while v < v_count {
            if self.csc_col_ptr[v + 1] == self.csc_col_ptr[v] {
                if written >= out.len() {
                    break;
                }
                out[written] = v as u32;
                written += 1;
            }
            v += 1;
        }
        written
    }

    /// Collect all sink/leaf vertices (`out_degree(u) == 0`) into `out`.
    pub fn collect_sinks(&self, out: &mut [u32]) -> usize {
        let mut written = 0usize;
        let v_count = self.header.vertex_count as usize;
        let mut u = 0usize;
        while u < v_count {
            if self.csr_row_ptr[u + 1] == self.csr_row_ptr[u] {
                if written >= out.len() {
                    break;
                }
                out[written] = u as u32;
                written += 1;
            }
            u += 1;
        }
        written
    }

    /// When `TopologyInvariants::REIFIED_EDGES` is active, resolves the edge at `edge_pos`
    /// to its RDF-Star `TAG_EMBEDDED` virtual vertex in the graph.
    pub fn resolve_reified_edge_vertex(
        &self,
        quins: &[crate::NQuin],
        edge_pos: usize,
    ) -> Option<u32> {
        if !self
            .header
            .invariants
            .contains(TopologyInvariants::REIFIED_EDGES)
        {
            return None;
        }
        let quin_idx = *self.edge_anchor.get(edge_pos)? as usize;
        let q = quins.get(quin_idx)?;
        let virtual_id = crate::query::rdf_star::create_virtual_id(q.subject, q.predicate, q.object);
        self.find_vertex_by_entity_hash(virtual_id)
    }
}

#[inline]
pub(crate) fn slice_pod<'a, T: Pod>(
    segment: &'a [u8],
    offset: u32,
    count: usize,
) -> Result<&'a [T], TopologyError> {
    let start = offset as usize;
    if count == 0 {
        if start > segment.len() {
            return Err(TopologyError::BufferTooSmall);
        }
        return Ok(&[]);
    }
    let byte_len = count
        .checked_mul(core::mem::size_of::<T>())
        .ok_or(TopologyError::BufferTooSmall)?;
    let end = start
        .checked_add(byte_len)
        .ok_or(TopologyError::BufferTooSmall)?;
    let bytes = segment
        .get(start..end)
        .ok_or(TopologyError::BufferTooSmall)?;
    bytemuck::try_cast_slice(bytes).map_err(|_| TopologyError::UnalignedBuffer)
}

