//! In-process Kademlia k-buckets over 48-byte [`StrongDigest`] keys (E07.8).
//!
//! XOR-distance buckets, K=8, bootstrap, iterative lookup, and a last-seen
//! maintenance tick. This is **not** networked Kademlia: no transport, crypto
//! suite, replication budget, asynchronous RPCs, or two-host overlay. Do not
//! treat results as a superiority study.

use super::key::keys_equal;
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::StrongDigest;

/// Contacts kept per XOR-distance bucket (classic k-bucket width).
pub const K: usize = 8;
/// Parallel probes per iterative-lookup round (Kademlia α).
pub const ALPHA: u8 = 3;
/// Bit length of [`StrongDigest`]; one bucket per `floor(log2(xor))`.
pub const BUCKET_COUNT: usize = 384;
/// In-process contact ceiling. Fail closed; not a DHT size.
pub const CONTACT_CAP: usize = 64;
/// Generations of `maintain` a contact may age before it is dropped.
pub const STALE_GENERATIONS: u32 = 2;

#[repr(C)]
#[derive(Clone, Copy)]
struct Contact {
    id: StrongDigest,
    last_seen: u32,
}

impl Contact {
    const EMPTY: Self = Self {
        id: StrongDigest::ZERO,
        last_seen: 0,
    };
}

/// Bounded XOR k-bucket routing table. Caller-owned; lookup does not allocate.
pub struct KBucketTable {
    local: StrongDigest,
    clock: u32,
    bucket_len: [u8; BUCKET_COUNT],
    contacts: [Contact; CONTACT_CAP],
    len: u8,
}

impl KBucketTable {
    pub const fn new(local: StrongDigest) -> Self {
        Self {
            local,
            clock: 1,
            bucket_len: [0; BUCKET_COUNT],
            contacts: [Contact::EMPTY; CONTACT_CAP],
            len: 0,
        }
    }

    #[inline]
    pub const fn local_id(&self) -> StrongDigest {
        self.local
    }

    #[inline]
    pub const fn len(&self) -> usize {
        self.len as usize
    }

    /// Insert `id`. Duplicate IDs refresh last-seen. Distance-0 (self) is Range.
    /// A full k-bucket or a full table is Capacity.
    pub fn insert(&mut self, id: StrongDigest) -> Result<(), QdnfError> {
        let bucket = bucket_index(&self.local, &id)?;
        let mut i = 0usize;
        while i < self.len as usize {
            if keys_equal(&self.contacts[i].id, &id) {
                self.contacts[i].last_seen = self.clock;
                return Ok(());
            }
            i += 1;
        }
        if self.bucket_len[bucket] as usize >= K {
            return Err(QdnfError::Capacity);
        }
        if self.len as usize >= CONTACT_CAP {
            return Err(QdnfError::Capacity);
        }
        let slot = self.len as usize;
        self.contacts[slot] = Contact {
            id,
            last_seen: self.clock,
        };
        self.len = self.len + 1;
        self.bucket_len[bucket] = self.bucket_len[bucket] + 1;
        Ok(())
    }

    /// Insert known node IDs. Stops and fails closed on the first Capacity/Range.
    pub fn bootstrap(&mut self, known: &[StrongDigest]) -> Result<u8, QdnfError> {
        let mut n: u8 = 0;
        let mut i = 0usize;
        while i < known.len() {
            self.insert(known[i])?;
            n = n + 1;
            i += 1;
        }
        Ok(n)
    }

    /// Up to K XOR-nearest contacts. Empty `out` with contacts present is Capacity.
    pub fn closest(&self, target: &StrongDigest, out: &mut [StrongDigest]) -> Result<usize, QdnfError> {
        if out.is_empty() {
            return Err(QdnfError::Capacity);
        }
        let mut ids = [StrongDigest::ZERO; K];
        let mut dists = [[0xffu8; 48]; K];
        let n = self.collect_closest(target, &mut ids, &mut dists);
        let mut w = 0usize;
        while w < n && w < out.len() {
            out[w] = ids[w];
            w += 1;
        }
        Ok(w)
    }

    /// Iterative FIND_NODE over the local table. Each shortlist probe costs one
    /// work unit. Shared-table respondents cannot introduce unknown contacts.
    /// Exhausting `work_budget` before every shortlist contact is probed is
    /// [`QdnfError::BudgetExhausted`].
    pub fn iterative_lookup(
        &self,
        target: &StrongDigest,
        work_budget: u8,
        out: &mut [StrongDigest],
    ) -> Result<u8, QdnfError> {
        if self.len == 0 {
            return Err(QdnfError::Incomplete);
        }
        if out.is_empty() {
            return Err(QdnfError::Capacity);
        }
        let mut ids = [StrongDigest::ZERO; K];
        let mut dists = [[0xffu8; 48]; K];
        let n = self.collect_closest(target, &mut ids, &mut dists);
        if n == 0 {
            return Err(QdnfError::Incomplete);
        }
        let mut queried = [false; K];
        let mut used: u8 = 0;
        loop {
            let mut probed: u8 = 0;
            let mut i = 0usize;
            while i < n && probed < ALPHA {
                if !queried[i] {
                    if used == work_budget {
                        return Err(QdnfError::BudgetExhausted);
                    }
                    used = used + 1;
                    queried[i] = true;
                    probed = probed + 1;
                }
                i += 1;
            }
            if probed == 0 {
                break;
            }
        }
        let mut w = 0usize;
        while w < n && w < out.len() {
            out[w] = ids[w];
            w += 1;
        }
        Ok(used)
    }

    /// Advance the last-seen clock and drop contacts older than
    /// [`STALE_GENERATIONS`]. Returns the number of dropped entries.
    pub fn maintain(&mut self) -> u16 {
        self.clock = self.clock.wrapping_add(1);
        let old_len = self.len as usize;
        let mut w = 0usize;
        let mut dropped: u16 = 0;
        let mut r = 0usize;
        while r < old_len {
            let age = self.clock.saturating_sub(self.contacts[r].last_seen);
            if age <= STALE_GENERATIONS {
                if w != r {
                    self.contacts[w] = self.contacts[r];
                }
                w += 1;
            } else {
                dropped = dropped + 1;
            }
            r += 1;
        }
        let mut t = w;
        while t < old_len {
            self.contacts[t] = Contact::EMPTY;
            t += 1;
        }
        self.len = w as u8;
        self.rebuild_bucket_len();
        dropped
    }

    fn rebuild_bucket_len(&mut self) {
        self.bucket_len = [0; BUCKET_COUNT];
        let mut i = 0usize;
        while i < self.len as usize {
            if let Ok(b) = bucket_index(&self.local, &self.contacts[i].id) {
                self.bucket_len[b] = self.bucket_len[b] + 1;
            }
            i += 1;
        }
    }

    fn collect_closest(
        &self,
        target: &StrongDigest,
        ids: &mut [StrongDigest; K],
        dists: &mut [[u8; 48]; K],
    ) -> usize {
        let mut n = 0usize;
        let mut i = 0usize;
        while i < self.len as usize {
            let id = self.contacts[i].id;
            let mut dist = [0u8; 48];
            xor_bytes(target, &id, &mut dist);
            consider_candidate(id, &dist, ids, dists, &mut n);
            i += 1;
        }
        n
    }
}

/// `floor(log2(local XOR id))`. Identical IDs (distance 0) are Range.
pub fn bucket_index(local: &StrongDigest, id: &StrongDigest) -> Result<usize, QdnfError> {
    let mut i = 0usize;
    while i < 48 {
        let x = local.0[i] ^ id.0[i];
        if x != 0 {
            let from_msb = i * 8 + x.leading_zeros() as usize;
            return Ok((BUCKET_COUNT - 1) - from_msb);
        }
        i += 1;
    }
    Err(QdnfError::Range)
}

fn xor_bytes(a: &StrongDigest, b: &StrongDigest, out: &mut [u8; 48]) {
    let mut i = 0usize;
    while i < 48 {
        out[i] = a.0[i] ^ b.0[i];
        i += 1;
    }
}

fn dist_less(a: &[u8; 48], b: &[u8; 48]) -> bool {
    let mut i = 0usize;
    while i < 48 {
        if a[i] < b[i] {
            return true;
        }
        if a[i] > b[i] {
            return false;
        }
        i += 1;
    }
    false
}

fn consider_candidate(
    id: StrongDigest,
    dist: &[u8; 48],
    ids: &mut [StrongDigest; K],
    dists: &mut [[u8; 48]; K],
    n: &mut usize,
) {
    let mut i = 0usize;
    while i < *n {
        if keys_equal(&ids[i], &id) {
            return;
        }
        i += 1;
    }
    if *n < K {
        let mut pos = *n;
        while pos > 0 && dist_less(dist, &dists[pos - 1]) {
            ids[pos] = ids[pos - 1];
            dists[pos] = dists[pos - 1];
            pos -= 1;
        }
        ids[pos] = id;
        dists[pos] = *dist;
        *n += 1;
        return;
    }
    if !dist_less(dist, &dists[K - 1]) {
        return;
    }
    let mut pos = K - 1;
    while pos > 0 && dist_less(dist, &dists[pos - 1]) {
        ids[pos] = ids[pos - 1];
        dists[pos] = dists[pos - 1];
        pos -= 1;
    }
    ids[pos] = id;
    dists[pos] = *dist;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id_tail(last: u8) -> StrongDigest {
        let mut k = StrongDigest::ZERO;
        k.0[47] = last;
        k
    }

    #[test]
    fn insert_closest_returns_xor_nearest() {
        let mut table = KBucketTable::new(StrongDigest::ZERO);
        let a = id_tail(1);
        let b = id_tail(4);
        let c = id_tail(9);
        table.insert(a).unwrap();
        table.insert(c).unwrap();
        table.insert(b).unwrap();
        let mut out = [StrongDigest::ZERO; K];
        let n = table.closest(&id_tail(5), &mut out).unwrap();
        assert_eq!(n, 3);
        assert!(keys_equal(&out[0], &b));
        assert!(keys_equal(&out[1], &a) || keys_equal(&out[1], &c));
    }

    #[test]
    fn iterative_lookup_finds_present_key() {
        let mut table = KBucketTable::new(StrongDigest::ZERO);
        let a = id_tail(1);
        let b = id_tail(9);
        table.bootstrap(&[a, b]).unwrap();
        let mut out = [StrongDigest::ZERO; K];
        let used = table.iterative_lookup(&a, 8, &mut out).unwrap();
        assert_eq!(used, 2);
        assert!(keys_equal(&out[0], &a));
    }

    #[test]
    fn iterative_lookup_work_budget_exhaustion_is_budget_exhausted() {
        let mut table = KBucketTable::new(StrongDigest::ZERO);
        table.bootstrap(&[id_tail(1), id_tail(9)]).unwrap();
        let mut out = [StrongDigest::ZERO; K];
        assert_eq!(
            table.iterative_lookup(&id_tail(1), 1, &mut out),
            Err(QdnfError::BudgetExhausted)
        );
    }

    #[test]
    fn insert_fails_closed_when_k_bucket_is_full() {
        let mut table = KBucketTable::new(StrongDigest::ZERO);
        let mut last = 16u8;
        while last < 24 {
            table.insert(id_tail(last)).unwrap();
            last = last + 1;
        }
        assert_eq!(table.len(), 8);
        assert_eq!(table.insert(id_tail(24)), Err(QdnfError::Capacity));
    }

    #[test]
    fn maintain_drops_stale_entries() {
        let mut table = KBucketTable::new(StrongDigest::ZERO);
        table.insert(id_tail(1)).unwrap();
        assert_eq!(table.maintain(), 0);
        assert_eq!(table.maintain(), 0);
        assert_eq!(table.maintain(), 1);
        assert_eq!(table.len(), 0);
        table.insert(id_tail(2)).unwrap();
        table.insert(id_tail(2)).unwrap();
        assert_eq!(table.maintain(), 0);
        assert_eq!(table.len(), 1);
    }

    #[test]
    fn bucket_index_is_log2_xor_distance() {
        assert_eq!(bucket_index(&StrongDigest::ZERO, &id_tail(1)), Ok(0));
        assert_eq!(bucket_index(&StrongDigest::ZERO, &id_tail(16)), Ok(4));
        assert_eq!(
            bucket_index(&StrongDigest::ZERO, &StrongDigest::ZERO),
            Err(QdnfError::Range)
        );
    }

    #[test]
    fn empty_table_lookup_is_incomplete() {
        let table = KBucketTable::new(StrongDigest::ZERO);
        let mut out = [StrongDigest::ZERO; 1];
        assert_eq!(
            table.iterative_lookup(&id_tail(1), 8, &mut out),
            Err(QdnfError::Incomplete)
        );
    }
}
