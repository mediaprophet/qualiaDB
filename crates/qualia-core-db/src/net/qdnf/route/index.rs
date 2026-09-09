//! E08.2 — caller-backed adjacency index. Production is not capped at 16 nodes.

use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::Generation;

use super::forwarding::ForwardingGeneration;
use super::spf::SpfTable;
use super::validate::ValidatedEdge;

/// Fixed production table. Callers size work by this capacity, not by SPF's 16.
pub const MAX_ADMITTED_EDGES: usize = 64;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Tombstone {
    origin: u8,
    from: u8,
    to: u8,
    sequence: u32,
}

/// Admitted adjacency plus unpublished recompute state.
pub struct AdjacencyIndex {
    scope: u64,
    slots: [Option<ValidatedEdge>; MAX_ADMITTED_EDGES],
    tombs: [Option<Tombstone>; MAX_ADMITTED_EDGES],
    last_seq: [u32; 256],
    seq_present: [bool; 256],
    dirty: bool,
    topology_gen: Generation,
    live: ForwardingGeneration,
    draft: ForwardingGeneration,
}

impl AdjacencyIndex {
    pub const fn new(scope: u64) -> Self {
        Self {
            scope,
            slots: [None; MAX_ADMITTED_EDGES],
            tombs: [None; MAX_ADMITTED_EDGES],
            last_seq: [0; 256],
            seq_present: [false; 256],
            dirty: false,
            topology_gen: Generation(1),
            live: ForwardingGeneration::empty(0),
            draft: ForwardingGeneration::empty(0),
        }
    }

    #[inline]
    pub const fn scope(&self) -> u64 {
        self.scope
    }

    #[inline]
    pub const fn is_dirty(&self) -> bool {
        self.dirty
    }

    #[inline]
    pub const fn topology_generation(&self) -> Generation {
        self.topology_gen
    }

    #[inline]
    pub const fn published(&self) -> &ForwardingGeneration {
        &self.live
    }

    #[inline]
    pub const fn draft(&self) -> &ForwardingGeneration {
        &self.draft
    }

    pub fn last_sequence(&self, origin: u8) -> Option<u32> {
        if self.seq_present[origin as usize] {
            Some(self.last_seq[origin as usize])
        } else {
            None
        }
    }

    pub fn live_count(&self, now_unix: u64) -> usize {
        let mut n = 0usize;
        let mut i = 0usize;
        while i < MAX_ADMITTED_EDGES {
            if let Some(e) = self.slots[i] {
                if !e.withdrawn && now_unix < e.expiry_unix {
                    n += 1;
                }
            }
            i += 1;
        }
        n
    }

    pub fn is_withdrawn(&self, from: u8, to: u8) -> bool {
        let mut i = 0usize;
        while i < MAX_ADMITTED_EDGES {
            if let Some(t) = self.tombs[i] {
                if (t.from == from && t.to == to) || (t.from == to && t.to == from) {
                    return true;
                }
            }
            i += 1;
        }
        false
    }

    /// Copy live, unexpired, non-withdrawn edges into `out`.
    pub fn live_edges(&self, now_unix: u64, out: &mut [ValidatedEdge]) -> usize {
        let mut n = 0usize;
        let mut i = 0usize;
        while i < MAX_ADMITTED_EDGES && n < out.len() {
            if let Some(e) = self.slots[i] {
                if !e.withdrawn && now_unix < e.expiry_unix {
                    out[n] = e;
                    n += 1;
                }
            }
            i += 1;
        }
        n
    }

    pub fn edge_at(&self, slot: usize) -> Option<ValidatedEdge> {
        if slot >= MAX_ADMITTED_EDGES {
            return None;
        }
        self.slots[slot]
    }

    /// Store a completed table in `draft` only. Does not replace published `live`.
    pub fn store_recompute(&mut self, table: SpfTable) -> Result<(), QdnfError> {
        self.draft = ForwardingGeneration::empty(self.topology_gen.0);
        self.draft.table = table;
        self.draft.published = false;
        self.dirty = false;
        Ok(())
    }

    /// Publish only after computation completed into `draft`.
    pub fn publish_recompute(&mut self) -> Result<(), QdnfError> {
        self.live.publish(self.draft.table)?;
        self.live.id = self.draft.id;
        self.draft = ForwardingGeneration::empty(0);
        Ok(())
    }

    fn bump(&mut self) -> Result<(), QdnfError> {
        self.topology_gen = self.topology_gen.next()?;
        self.dirty = true;
        Ok(())
    }

    fn remember_seq(&mut self, origin: u8, sequence: u32) {
        self.last_seq[origin as usize] = sequence;
        self.seq_present[origin as usize] = true;
    }

    fn find_live(&self, from: u8, to: u8) -> Option<usize> {
        let mut i = 0usize;
        while i < MAX_ADMITTED_EDGES {
            if let Some(e) = self.slots[i] {
                if !e.withdrawn && same_undirected(e.from, e.to, from, to) {
                    return Some(i);
                }
            }
            i += 1;
        }
        None
    }

    fn find_tomb(&self, origin: u8, from: u8, to: u8) -> Option<usize> {
        let mut i = 0usize;
        while i < MAX_ADMITTED_EDGES {
            if let Some(t) = self.tombs[i] {
                if t.origin == origin && same_undirected(t.from, t.to, from, to) {
                    return Some(i);
                }
            }
            i += 1;
        }
        None
    }

    fn free_slot(&self) -> Option<usize> {
        let mut i = 0usize;
        while i < MAX_ADMITTED_EDGES {
            if self.slots[i].is_none() {
                return Some(i);
            }
            i += 1;
        }
        None
    }

    fn free_tomb(&self) -> Option<usize> {
        let mut i = 0usize;
        while i < MAX_ADMITTED_EDGES {
            if self.tombs[i].is_none() {
                return Some(i);
            }
            i += 1;
        }
        None
    }

    fn clear_tomb(&mut self, origin: u8, from: u8, to: u8) {
        if let Some(i) = self.find_tomb(origin, from, to) {
            self.tombs[i] = None;
        }
    }
}

fn same_undirected(a: u8, b: u8, c: u8, d: u8) -> bool {
    (a == c && b == d) || (a == d && b == c)
}

/// Insert a validated edge. Withdrawals become tombstones, never cheap candidates.
pub fn insert_edge(index: &mut AdjacencyIndex, rec: ValidatedEdge) -> Result<(), QdnfError> {
    if rec.origin == 0 {
        return Err(QdnfError::Range);
    }
    if let Some(prev) = index.last_sequence(rec.origin) {
        if rec.sequence <= prev {
            return Err(QdnfError::StaleGeneration);
        }
    }
    if rec.withdrawn {
        return withdraw(index, rec);
    }
    if let Some(ti) = index.find_tomb(rec.origin, rec.from, rec.to) {
        let t = index.tombs[ti].unwrap();
        if rec.sequence <= t.sequence {
            return Err(QdnfError::StaleGeneration);
        }
        index.tombs[ti] = None;
    } else if index.is_withdrawn(rec.from, rec.to) {
        return Err(QdnfError::Conflict);
    }
    if let Some(slot) = index.find_live(rec.from, rec.to) {
        let existing = index.slots[slot].unwrap();
        if existing.origin != rec.origin {
            return Err(QdnfError::Conflict);
        }
        if existing.cost != rec.cost
            && existing.sequence == rec.sequence
            && existing.origin == rec.origin
        {
            return Err(QdnfError::Conflict);
        }
        index.slots[slot] = Some(rec);
        index.remember_seq(rec.origin, rec.sequence);
        index.bump()?;
        return Ok(());
    }
    let slot = index.free_slot().ok_or(QdnfError::Capacity)?;
    index.slots[slot] = Some(rec);
    index.remember_seq(rec.origin, rec.sequence);
    index.bump()
}

fn withdraw(index: &mut AdjacencyIndex, rec: ValidatedEdge) -> Result<(), QdnfError> {
    if let Some(slot) = index.find_live(rec.from, rec.to) {
        let existing = index.slots[slot].unwrap();
        if existing.origin != rec.origin {
            return Err(QdnfError::Conflict);
        }
        index.slots[slot] = None;
    }
    index.clear_tomb(rec.origin, rec.from, rec.to);
    let ti = index.free_tomb().ok_or(QdnfError::Capacity)?;
    index.tombs[ti] = Some(Tombstone {
        origin: rec.origin,
        from: rec.from,
        to: rec.to,
        sequence: rec.sequence,
    });
    index.remember_seq(rec.origin, rec.sequence);
    index.bump()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::qdnf::route::spf::{compute_spf, LinkMetric, MAX_NODES};
    use crate::net::qdnf::route::validate::{insert, TopologyRecord, PROFILE_P0};
    use crate::net::qdnf::types::StrongDigest;

    fn rec(origin: u8, from: u8, to: u8, seq: u32) -> TopologyRecord {
        TopologyRecord {
            origin,
            origin_digest: {
                let mut d = StrongDigest::ZERO;
                d.0[0] = origin;
                d.0[47] = 1;
                d
            },
            scope: 1,
            from,
            to,
            sequence: seq,
            expiry_unix: 100,
            withdrawn: false,
            bidirectional: true,
            cost: 1,
            latency_ms: 1,
            energy_uj: 1,
            energy_known: true,
            realm_bit: 0,
            profile: PROFILE_P0,
            failure_domain: origin,
            digest: StrongDigest::ZERO,
        }
    }

    #[test]
    fn route_a_seventeenth_edge_admitted_spf_still_range() {
        let mut index = AdjacencyIndex::new(1);
        let mut seq = 1u32;
        for i in 0..16u8 {
            let from = i + 1;
            let to = i + 2;
            insert(&rec(from, from, to, seq), 1, 1, None, &mut index).unwrap();
            seq += 1;
        }
        assert_eq!(index.live_count(1), 16);
        insert(&rec(17, 17, 18, seq), 1, 1, None, &mut index).unwrap();
        assert_eq!(index.live_count(1), 17);

        let oversize = [LinkMetric {
            from: 15,
            to: 16,
            cost: 1,
            bidirectional: true,
        }];
        let mut table = SpfTable::EMPTY;
        // Origin 15 is in the demo table; neighbor 16 is not.
        assert_eq!(compute_spf(15, &oversize, &mut table), Err(QdnfError::Range));
        assert_eq!(compute_spf(MAX_NODES as u8, &[], &mut table), Err(QdnfError::Range));
        assert!(MAX_ADMITTED_EDGES > MAX_NODES);
    }

    #[test]
    fn unpublished_recompute_does_not_replace_published() {
        let mut index = AdjacencyIndex::new(1);
        insert(&rec(1, 1, 2, 1), 1, 1, None, &mut index).unwrap();
        let mut table = SpfTable::EMPTY;
        table.nodes = 2;
        table.dest[0] = 1;
        table.dest[1] = 2;
        table.hop_counts[1] = 1;
        table.hops[1][0].node = 2;
        index.live.publish(table).unwrap();
        assert_eq!(index.published().lookup_next(2), Ok(2));

        let mut draft_table = SpfTable::EMPTY;
        draft_table.nodes = 2;
        draft_table.dest[0] = 1;
        draft_table.dest[1] = 3;
        index.store_recompute(draft_table).unwrap();
        assert_eq!(index.draft().lookup_next(3), Err(QdnfError::Incomplete));
        assert_eq!(index.published().lookup_next(2), Ok(2));
        assert_ne!(index.published().table.dest[1], 3);
    }

    #[test]
    fn unpublished_forwarding_cannot_lookup() {
        let index = AdjacencyIndex::new(1);
        assert_eq!(index.published().lookup_next(1), Err(QdnfError::Incomplete));
        assert_eq!(index.draft().lookup_next(1), Err(QdnfError::Incomplete));
    }

    #[test]
    fn duplicate_origin_conflicting_adjacency_is_conflict() {
        let mut index = AdjacencyIndex::new(1);
        insert(&rec(1, 1, 2, 1), 1, 1, None, &mut index).unwrap();
        assert_eq!(
            insert(&rec(3, 1, 2, 1), 1, 1, None, &mut index),
            Err(QdnfError::Conflict)
        );
    }

    #[test]
    fn capacity_is_table_size_not_sixteen() {
        let mut index = AdjacencyIndex::new(1);
        let mut seq = 1u32;
        for i in 0..MAX_ADMITTED_EDGES as u8 {
            let from = (i % 200) + 1;
            let to = ((i + 1) % 200) + 1;
            if from == to {
                continue;
            }
            insert(&rec(from, from, to, seq), 1, 1, None, &mut index).unwrap();
            seq += 1;
        }
        assert_eq!(index.live_count(1), MAX_ADMITTED_EDGES);
        assert_eq!(
            insert(&rec(50, 50, 60, seq), 1, 1, None, &mut index),
            Err(QdnfError::Capacity)
        );
    }
}
