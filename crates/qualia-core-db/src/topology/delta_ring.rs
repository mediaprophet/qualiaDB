//! Fixed-size zero-heap `TopologicalDeltaRing` and `MergedGraphView`.
//!
//! Tier-1 Zero-Heap (`AGENTS.md` §0 & §0-A): live topological mutations (`Insert`, `Tombstone`,
//! `Reify`, `AnchorBind`) are recorded in a fixed-capacity `[TopologicalDeltaEntry; CAP]` array
//! and merged with `HarbouredCsrView` on the fly in caller-supplied slices.

use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};

use crate::topology::csr_view::HarbouredCsrView;
use crate::topology::header::{AnchorDomain, CellAnchor, TopologyError};

/// Operation discriminant encoded in bits `[0..3]` of `TopologicalDeltaEntry::op_domain_and_slot`.
#[repr(u8)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DeltaOpKind {
    #[default]
    Tombstone = 0,
    Insert = 1,
    Reify = 2,
    AnchorBind = 3,
}

impl DeltaOpKind {
    #[inline]
    pub const fn from_u8(v: u8) -> Option<Self> {
        match v & 0x0F {
            0 => Some(Self::Tombstone),
            1 => Some(Self::Insert),
            2 => Some(Self::Reify),
            3 => Some(Self::AnchorBind),
            _ => None,
        }
    }
}

/// 16-byte cache-aligned delta entry scoped to a specific `graph_slot` ($0 \dots M$)
/// inside the host `PoetGeometry`.
#[repr(C, align(16))]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Pod, Zeroable, Serialize, Deserialize)]
pub struct TopologicalDeltaEntry {
    pub source_vertex: u32,
    pub target_vertex: u32,
    /// Backing `NQuin` slot index or host Geometry $k$-cell index.
    pub edge_anchor_idx: u32,
    /// `[0..3]`: `DeltaOpKind` (`Tombstone=0`, `Insert=1`, `Reify=2`, `AnchorBind=3`)
    /// `[4..7]`: `AnchorDomain` nibble (`0..=4` or `0xF` for `SemanticQuin`)
    /// `[8..15]`: Harboured graph slot index `m` in `0..harboured_graph_count`
    pub op_domain_and_slot: u16,
    /// Monotonic sequence number within the open edit session.
    pub seq_no: u16,
}

const _: () = assert!(core::mem::size_of::<TopologicalDeltaEntry>() == 16);
const _: () = assert!(core::mem::align_of::<TopologicalDeltaEntry>() == 16);

impl TopologicalDeltaEntry {
    #[inline]
    pub const fn pack_op_domain_slot(
        op: DeltaOpKind,
        domain: AnchorDomain,
        graph_slot: u8,
    ) -> u16 {
        let op_nib = (op as u16) & 0x0F;
        let dom_nib = ((domain as u8) & 0x0F) as u16;
        op_nib | (dom_nib << 4) | ((graph_slot as u16) << 8)
    }

    #[inline]
    pub const fn new_insert(
        graph_slot: u8,
        source_vertex: u32,
        target_vertex: u32,
        edge_anchor_idx: u32,
        seq_no: u16,
    ) -> Self {
        Self {
            source_vertex,
            target_vertex,
            edge_anchor_idx,
            op_domain_and_slot: Self::pack_op_domain_slot(
                DeltaOpKind::Insert,
                AnchorDomain::SemanticQuin,
                graph_slot,
            ),
            seq_no,
        }
    }

    #[inline]
    pub const fn new_tombstone(
        graph_slot: u8,
        source_vertex: u32,
        target_vertex: u32,
        seq_no: u16,
    ) -> Self {
        Self {
            source_vertex,
            target_vertex,
            edge_anchor_idx: 0,
            op_domain_and_slot: Self::pack_op_domain_slot(
                DeltaOpKind::Tombstone,
                AnchorDomain::SemanticQuin,
                graph_slot,
            ),
            seq_no,
        }
    }

    #[inline]
    pub const fn new_reify(
        graph_slot: u8,
        source_vertex: u32,
        target_vertex: u32,
        edge_anchor_idx: u32,
        seq_no: u16,
    ) -> Self {
        Self {
            source_vertex,
            target_vertex,
            edge_anchor_idx,
            op_domain_and_slot: Self::pack_op_domain_slot(
                DeltaOpKind::Reify,
                AnchorDomain::SemanticQuin,
                graph_slot,
            ),
            seq_no,
        }
    }

    #[inline]
    pub const fn new_anchor_bind(
        graph_slot: u8,
        vertex: u32,
        cell_index: u32,
        domain: AnchorDomain,
        seq_no: u16,
    ) -> Self {
        Self {
            source_vertex: vertex,
            target_vertex: cell_index,
            edge_anchor_idx: cell_index,
            op_domain_and_slot: Self::pack_op_domain_slot(
                DeltaOpKind::AnchorBind,
                domain,
                graph_slot,
            ),
            seq_no,
        }
    }

    #[inline]
    pub const fn op_kind(&self) -> DeltaOpKind {
        match self.op_domain_and_slot & 0x0F {
            0 => DeltaOpKind::Tombstone,
            1 => DeltaOpKind::Insert,
            2 => DeltaOpKind::Reify,
            _ => DeltaOpKind::AnchorBind,
        }
    }

    #[inline]
    pub const fn graph_slot(&self) -> u8 {
        ((self.op_domain_and_slot >> 8) & 0xFF) as u8
    }

    #[inline]
    pub const fn anchor_domain(&self) -> AnchorDomain {
        match ((self.op_domain_and_slot >> 4) & 0x0F) as u8 {
            0 => AnchorDomain::Cell0Vertex,
            1 => AnchorDomain::Cell1Edge,
            2 => AnchorDomain::Cell2Face,
            3 => AnchorDomain::Cell3Volume,
            4 => AnchorDomain::CellKSimplex,
            _ => AnchorDomain::SemanticQuin,
        }
    }
}

/// Fixed-capacity, zero-heap delta ring buffer for live topological mutations across harboured graphs.
#[derive(Copy, Clone, Debug)]
pub struct TopologicalDeltaRing<const CAP: usize = 256> {
    entries: [TopologicalDeltaEntry; CAP],
    len: usize,
    next_seq: u16,
}

impl<const CAP: usize> Default for TopologicalDeltaRing<CAP> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const CAP: usize> TopologicalDeltaRing<CAP> {
    #[inline]
    pub const fn new() -> Self {
        Self {
            entries: [TopologicalDeltaEntry {
                source_vertex: 0,
                target_vertex: 0,
                edge_anchor_idx: 0,
                op_domain_and_slot: 0,
                seq_no: 0,
            }; CAP],
            len: 0,
            next_seq: 0,
        }
    }

    #[inline]
    pub const fn len(&self) -> usize {
        self.len
    }

    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[inline]
    pub const fn capacity(&self) -> usize {
        CAP
    }

    #[inline]
    pub fn clear(&mut self) {
        self.len = 0;
        self.next_seq = 0;
    }

    #[inline]
    pub fn entries(&self) -> &[TopologicalDeltaEntry] {
        &self.entries[..self.len]
    }

    /// Append a delta entry, assigning the next monotonic `seq_no`.
    pub fn push_entry(&mut self, mut entry: TopologicalDeltaEntry) -> Result<u16, TopologyError> {
        if self.len >= CAP {
            return Err(TopologyError::DeltaRingFull { capacity: CAP });
        }
        let seq = self.next_seq;
        entry.seq_no = seq;
        self.entries[self.len] = entry;
        self.len += 1;
        self.next_seq = self.next_seq.wrapping_add(1);
        Ok(seq)
    }

    /// Record an edge insertion (`source_vertex -> target_vertex`) in `graph_slot`.
    #[inline]
    pub fn insert_edge(
        &mut self,
        graph_slot: u8,
        source_vertex: u32,
        target_vertex: u32,
        edge_anchor_idx: u32,
    ) -> Result<u16, TopologyError> {
        self.push_entry(TopologicalDeltaEntry::new_insert(
            graph_slot,
            source_vertex,
            target_vertex,
            edge_anchor_idx,
            0,
        ))
    }

    /// Record an edge deletion (`source_vertex -> target_vertex`) in `graph_slot`.
    #[inline]
    pub fn tombstone_edge(
        &mut self,
        graph_slot: u8,
        source_vertex: u32,
        target_vertex: u32,
    ) -> Result<u16, TopologyError> {
        self.push_entry(TopologicalDeltaEntry::new_tombstone(
            graph_slot,
            source_vertex,
            target_vertex,
            0,
        ))
    }

    /// Record a reified hyperedge insertion (`source_vertex -> target_vertex`) in `graph_slot`.
    #[inline]
    pub fn reify_edge(
        &mut self,
        graph_slot: u8,
        source_vertex: u32,
        target_vertex: u32,
        edge_anchor_idx: u32,
    ) -> Result<u16, TopologyError> {
        self.push_entry(TopologicalDeltaEntry::new_reify(
            graph_slot,
            source_vertex,
            target_vertex,
            edge_anchor_idx,
            0,
        ))
    }

    /// Bind `vertex` in `graph_slot` to `(domain, cell_index)` in the host Geometry.
    #[inline]
    pub fn bind_anchor(
        &mut self,
        graph_slot: u8,
        vertex: u32,
        cell_index: u32,
        domain: AnchorDomain,
    ) -> Result<u16, TopologyError> {
        self.push_entry(TopologicalDeltaEntry::new_anchor_bind(
            graph_slot,
            vertex,
            cell_index,
            domain,
            0,
        ))
    }

    /// 64-bit bitmask of harboured graph slots (`0..64`) modified in this ring.
    pub fn dirty_slot_mask(&self) -> u64 {
        let mut mask = 0u64;
        let mut i = 0usize;
        while i < self.len {
            let slot = self.entries[i].graph_slot();
            if slot < 64 {
                mask |= 1u64 << slot;
            }
            i += 1;
        }
        mask
    }
}

/// Zero-heap merged view combining an immutable `HarbouredCsrView` with live `TopologicalDeltaRing` edits.
#[derive(Copy, Clone, Debug)]
pub struct MergedGraphView<'a, const CAP: usize = 256> {
    base: HarbouredCsrView<'a>,
    ring: &'a TopologicalDeltaRing<CAP>,
    graph_slot: u16,
}

impl<'a, const CAP: usize> MergedGraphView<'a, CAP> {
    #[inline]
    pub const fn new(
        base: HarbouredCsrView<'a>,
        ring: &'a TopologicalDeltaRing<CAP>,
        graph_slot: u16,
    ) -> Self {
        Self {
            base,
            ring,
            graph_slot,
        }
    }

    #[inline]
    pub const fn base(&self) -> &HarbouredCsrView<'a> {
        &self.base
    }

    #[inline]
    pub const fn ring(&self) -> &'a TopologicalDeltaRing<CAP> {
        self.ring
    }

    #[inline]
    pub const fn graph_slot(&self) -> u16 {
        self.graph_slot
    }

    #[inline]
    pub const fn vertex_count(&self) -> u32 {
        self.base.vertex_count()
    }

    /// Materialize the effective outgoing neighbours and edge anchors of vertex `u`
    /// after applying all `TopologicalDeltaRing` mutations for `self.graph_slot` in `seq_no` order.
    pub fn effective_out_neighbors(
        &self,
        u: u32,
        out_targets: &mut [u32],
        out_edge_anchors: &mut [u32],
    ) -> Result<usize, TopologyError> {
        let base_targets = self.base.out_neighbors(u)?;
        let base_anchors = self.base.out_edge_anchors(u)?;
        let track_anchors = !out_edge_anchors.is_empty();

        if base_targets.len() > out_targets.len()
            || (track_anchors && base_anchors.len() > out_edge_anchors.len())
        {
            return Err(TopologyError::BufferTooSmall);
        }

        let mut count = base_targets.len();
        out_targets[..count].copy_from_slice(base_targets);
        if track_anchors {
            out_edge_anchors[..count].copy_from_slice(base_anchors);
        }

        let v_count = self.base.vertex_count();
        let entries = self.ring.entries();
        let mut i = 0usize;
        while i < entries.len() {
            let entry = &entries[i];
            i += 1;
            if entry.graph_slot() as u16 != self.graph_slot || entry.source_vertex != u {
                continue;
            }
            match entry.op_kind() {
                DeltaOpKind::Tombstone => {
                    let mut k = 0usize;
                    while k < count {
                        if out_targets[k] == entry.target_vertex {
                            let mut shift = k;
                            while shift + 1 < count {
                                out_targets[shift] = out_targets[shift + 1];
                                if track_anchors {
                                    out_edge_anchors[shift] = out_edge_anchors[shift + 1];
                                }
                                shift += 1;
                            }
                            count -= 1;
                        } else {
                            k += 1;
                        }
                    }
                }
                DeltaOpKind::Insert | DeltaOpKind::Reify => {
                    if entry.target_vertex >= v_count {
                        return Err(TopologyError::VertexOutOfRange {
                            vertex: entry.target_vertex,
                            count: v_count,
                        });
                    }
                    let mut existing = None;
                    let mut k = 0usize;
                    while k < count {
                        if out_targets[k] == entry.target_vertex {
                            existing = Some(k);
                            break;
                        }
                        k += 1;
                    }
                    if let Some(pos) = existing {
                        if track_anchors {
                            out_edge_anchors[pos] = entry.edge_anchor_idx;
                        }
                    } else {
                        if count >= out_targets.len()
                            || (track_anchors && count >= out_edge_anchors.len())
                        {
                            return Err(TopologyError::BufferTooSmall);
                        }
                        out_targets[count] = entry.target_vertex;
                        if track_anchors {
                            out_edge_anchors[count] = entry.edge_anchor_idx;
                        }
                        count += 1;
                    }
                }
                DeltaOpKind::AnchorBind => {}
            }
        }

        Ok(count)
    }

    /// Resolve the effective `CellAnchor` of vertex `v` after applying `AnchorBind` delta entries.
    pub fn effective_cell_anchor(&self, v: u32) -> Result<CellAnchor, TopologyError> {
        let mut anchor = *self.base.cell_anchor(v)?;
        let entries = self.ring.entries();
        let mut i = 0usize;
        while i < entries.len() {
            let e = &entries[i];
            if e.graph_slot() as u16 == self.graph_slot
                && e.op_kind() == DeltaOpKind::AnchorBind
                && e.source_vertex == v
            {
                anchor.cell_index = e.target_vertex;
                anchor.anchor_domain = e.anchor_domain() as u8;
            }
            i += 1;
        }
        Ok(anchor)
    }
}

