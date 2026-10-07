//! Zero-heap circular ring buffer for DNS and SDN Q42 records.
//!
//! Maintains a fast, deterministic cache of recent DNS resolution quins
//! without performing any heap allocations (Vec/String/Box).
//!
//! Provides explicit TTL-based pruning, clock-based pruning, and zero-heap
//! archival extraction into caller-supplied storage buffers.
//!
//! Fully conforms to QualiaDB Rule 0-A (zero heap in hot paths) and Rule 0-B.

#![allow(dead_code)]

use crate::NQuin;

/// Default capacity for the stack-resident DNS ring buffer.
pub const DEFAULT_DNS_CACHE_CAPACITY: usize = 256;

/// Zero-allocation ring buffer storing `NQuin` records.
#[derive(Debug, Clone)]
pub struct DnsCacheRing<const N: usize = DEFAULT_DNS_CACHE_CAPACITY> {
    slots: [NQuin; N],
    timestamps: [u32; N],
    head: usize,
    count: usize,
}

impl<const N: usize> Default for DnsCacheRing<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> DnsCacheRing<N> {
    pub const fn new() -> Self {
        Self {
            slots: [NQuin {
                subject: 0,
                predicate: 0,
                object: 0,
                context: 0,
                metadata: 0,
                parity: 0,
            }; N],
            timestamps: [0u32; N],
            head: 0,
            count: 0,
        }
    }

    /// Number of active items in the ring buffer.
    pub fn len(&self) -> usize {
        self.count
    }

    /// Check if the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Insert a quin into the ring buffer, evicting the oldest entry on wrap.
    pub fn insert(&mut self, quin: NQuin, current_clock: u32) {
        if !quin.verify_ecc_parity() {
            return;
        }
        let idx = self.head % N;
        self.slots[idx] = quin;
        self.timestamps[idx] = current_clock;
        self.head = self.head.wrapping_add(1);
        if self.count < N {
            self.count += 1;
        }
    }

    /// Find the most recent record matching `subject_hash` and `opcode`.
    /// Searches backwards from most recently inserted to oldest. Zero-allocation.
    pub fn lookup(&self, subject_hash: u64, opcode: u8) -> Option<NQuin> {
        let active = self.count;
        for i in 0..active {
            let offset = (self.head.wrapping_sub(1).wrapping_sub(i)) % N;
            let quin = self.slots[offset];
            if quin.subject == subject_hash && (quin.predicate & 0xFF) as u8 == opcode {
                return Some(quin);
            }
        }
        None
    }

    /// Find the most recent record matching `subject_hash` and `opcode` that has NOT expired.
    ///
    /// Verifies `current_clock < inserted_clock + ttl`. Zero-allocation.
    pub fn lookup_valid(&self, subject_hash: u64, opcode: u8, current_clock: u32) -> Option<NQuin> {
        let active = self.count;
        for i in 0..active {
            let offset = (self.head.wrapping_sub(1).wrapping_sub(i)) % N;
            let quin = self.slots[offset];
            if quin.subject == subject_hash && (quin.predicate & 0xFF) as u8 == opcode {
                let ttl = (quin.metadata & 0xFFFF_FFFF) as u32;
                let inserted_at = self.timestamps[offset];
                if current_clock < inserted_at.saturating_add(ttl) {
                    return Some(quin);
                }
            }
        }
        None
    }

    /// Collect all records matching `subject_hash` into a caller-supplied buffer.
    ///
    /// Returns the number of items written into `out`. Zero-allocation.
    pub fn lookup_all(&self, subject_hash: u64, out: &mut [NQuin]) -> usize {
        let mut written = 0;
        let active = self.count;
        for i in 0..active {
            if written >= out.len() {
                break;
            }
            let offset = (self.head.wrapping_sub(1).wrapping_sub(i)) % N;
            let quin = self.slots[offset];
            if quin.subject == subject_hash {
                out[written] = quin;
                written += 1;
            }
        }
        written
    }

    /// Prune all expired records from the ring buffer based on `current_clock`.
    ///
    /// Reclaims slots and compacts active unexpired entries in place.
    /// Returns the count of pruned records. Zero heap allocation.
    pub fn prune_expired(&mut self, current_clock: u32) -> usize {
        let mut retained_slots = [NQuin::default(); N];
        let mut retained_times = [0u32; N];
        let mut retained_count = 0;

        let active = self.count;
        for i in 0..active {
            let offset = (self.head.wrapping_sub(active).wrapping_add(i)) % N;
            let quin = self.slots[offset];
            let ttl = (quin.metadata & 0xFFFF_FFFF) as u32;
            let inserted_at = self.timestamps[offset];
            let is_expired = current_clock >= inserted_at.saturating_add(ttl);

            if !is_expired {
                retained_slots[retained_count] = quin;
                retained_times[retained_count] = inserted_at;
                retained_count += 1;
            }
        }

        let pruned = self.count - retained_count;
        self.slots = retained_slots;
        self.timestamps = retained_times;
        self.head = retained_count;
        self.count = retained_count;
        pruned
    }

    /// Prune records inserted strictly before `cutoff_clock`.
    ///
    /// Returns the count of pruned records. Zero heap allocation.
    pub fn prune_older_than(&mut self, cutoff_clock: u32) -> usize {
        let mut retained_slots = [NQuin::default(); N];
        let mut retained_times = [0u32; N];
        let mut retained_count = 0;

        let active = self.count;
        for i in 0..active {
            let offset = (self.head.wrapping_sub(active).wrapping_add(i)) % N;
            let quin = self.slots[offset];
            let inserted_at = self.timestamps[offset];

            if inserted_at >= cutoff_clock {
                retained_slots[retained_count] = quin;
                retained_times[retained_count] = inserted_at;
                retained_count += 1;
            }
        }

        let pruned = self.count - retained_count;
        self.slots = retained_slots;
        self.timestamps = retained_times;
        self.head = retained_count;
        self.count = retained_count;
        pruned
    }

    /// Archive all expired records into `archive_sink` and remove them from the ring buffer.
    ///
    /// Returns the count of archived records written into `archive_sink`. Zero heap allocation.
    pub fn archive_expired_into(
        &mut self,
        current_clock: u32,
        archive_sink: &mut [NQuin],
    ) -> usize {
        let mut archived = 0;
        let mut retained_count = 0;
        let active = self.count;

        let mut retained_slots = [NQuin::default(); N];
        let mut retained_times = [0u32; N];

        for i in 0..active {
            let offset = (self.head.wrapping_sub(active).wrapping_add(i)) % N;
            let quin = self.slots[offset];
            let ttl = (quin.metadata & 0xFFFF_FFFF) as u32;
            let inserted_at = self.timestamps[offset];
            let is_expired = current_clock >= inserted_at.saturating_add(ttl);

            if is_expired {
                if archived < archive_sink.len() {
                    archive_sink[archived] = quin;
                    archived += 1;
                }
            } else {
                retained_slots[retained_count] = quin;
                retained_times[retained_count] = inserted_at;
                retained_count += 1;
            }
        }

        self.slots = retained_slots;
        self.timestamps = retained_times;
        self.head = retained_count;
        self.count = retained_count;
        archived
    }

    /// Archive records inserted before `cutoff_clock` into `archive_sink` and remove them.
    ///
    /// Returns the count of archived records written into `archive_sink`. Zero heap allocation.
    pub fn archive_older_than(&mut self, cutoff_clock: u32, archive_sink: &mut [NQuin]) -> usize {
        let mut archived = 0;
        let mut retained_count = 0;
        let active = self.count;

        let mut retained_slots = [NQuin::default(); N];
        let mut retained_times = [0u32; N];

        for i in 0..active {
            let offset = (self.head.wrapping_sub(active).wrapping_add(i)) % N;
            let quin = self.slots[offset];
            let inserted_at = self.timestamps[offset];
            let is_older = inserted_at < cutoff_clock;

            if is_older {
                if archived < archive_sink.len() {
                    archive_sink[archived] = quin;
                    archived += 1;
                }
            } else {
                retained_slots[retained_count] = quin;
                retained_times[retained_count] = inserted_at;
                retained_count += 1;
            }
        }

        self.slots = retained_slots;
        self.timestamps = retained_times;
        self.head = retained_count;
        self.count = retained_count;
        archived
    }

    /// Clear all cached records.
    pub fn clear(&mut self) {
        self.head = 0;
        self.count = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::dns::quin_records::*;
    use crate::PermissiveRoutingLane;

    #[test]
    fn ring_insert_and_lookup() {
        let mut ring = DnsCacheRing::<16>::new();
        assert!(ring.is_empty());

        let q1 = encode_a_record(
            "host1.lan",
            [10, 0, 0, 1],
            60,
            PermissiveRoutingLane::PassthroughStandard,
        );
        let q2 = encode_a_record(
            "host2.lan",
            [10, 0, 0, 2],
            60,
            PermissiveRoutingLane::PassthroughStandard,
        );

        ring.insert(q1, 100);
        ring.insert(q2, 101);
        assert_eq!(ring.len(), 2);

        let found = ring.lookup(q1.subject, OP_DNS_A).unwrap();
        assert_eq!(found.object, q1.object);

        assert!(ring.lookup(99999, OP_DNS_A).is_none());
        assert!(ring.lookup(q1.subject, OP_DNS_TXT).is_none());
    }

    #[test]
    fn ring_lookup_valid_ttl_expiration() {
        let mut ring = DnsCacheRing::<16>::new();
        let q1 = encode_a_record(
            "expiring.lan",
            [192, 168, 1, 1],
            10,
            PermissiveRoutingLane::PassthroughStandard,
        );
        ring.insert(q1, 100); // inserted at clock 100 with TTL 10 -> valid until < 110

        // At clock 105: still valid
        assert!(ring.lookup_valid(q1.subject, OP_DNS_A, 105).is_some());

        // At clock 110: expired
        assert!(ring.lookup_valid(q1.subject, OP_DNS_A, 110).is_none());
    }

    #[test]
    fn ring_prune_expired() {
        let mut ring = DnsCacheRing::<16>::new();
        let q_short = encode_a_record(
            "short.lan",
            [1, 1, 1, 1],
            5,
            PermissiveRoutingLane::PassthroughStandard,
        );
        let q_long = encode_a_record(
            "long.lan",
            [2, 2, 2, 2],
            100,
            PermissiveRoutingLane::PassthroughStandard,
        );

        ring.insert(q_short, 50); // expires at 55
        ring.insert(q_long, 50); // expires at 150
        assert_eq!(ring.len(), 2);

        // At clock 60, q_short has expired
        let pruned = ring.prune_expired(60);
        assert_eq!(pruned, 1);
        assert_eq!(ring.len(), 1);

        assert!(ring.lookup(q_short.subject, OP_DNS_A).is_none());
        assert!(ring.lookup(q_long.subject, OP_DNS_A).is_some());
    }

    #[test]
    fn ring_archive_expired_into() {
        let mut ring = DnsCacheRing::<16>::new();
        let q_expired1 = encode_a_record(
            "exp1.lan",
            [10, 0, 0, 1],
            10,
            PermissiveRoutingLane::PassthroughStandard,
        );
        let q_expired2 = encode_a_record(
            "exp2.lan",
            [10, 0, 0, 2],
            15,
            PermissiveRoutingLane::PassthroughStandard,
        );
        let q_alive = encode_a_record(
            "alive.lan",
            [10, 0, 0, 3],
            300,
            PermissiveRoutingLane::PassthroughStandard,
        );

        ring.insert(q_expired1, 100); // expires at 110
        ring.insert(q_expired2, 100); // expires at 115
        ring.insert(q_alive, 100); // expires at 400

        let mut archive = [NQuin::default(); 4];
        let count = ring.archive_expired_into(120, &mut archive);

        assert_eq!(count, 2);
        assert_eq!(archive[0].subject, q_expired1.subject);
        assert_eq!(archive[1].subject, q_expired2.subject);
        assert_eq!(ring.len(), 1);
        assert!(ring.lookup(q_alive.subject, OP_DNS_A).is_some());
    }

    #[test]
    fn ring_wrap_eviction() {
        let mut ring = DnsCacheRing::<4>::new();
        for i in 0..6 {
            let q = encode_a_record(
                "wrap.lan",
                [10, 0, 0, i as u8],
                60,
                PermissiveRoutingLane::PassthroughStandard,
            );
            ring.insert(q, i as u32);
        }
        assert_eq!(ring.len(), 4);
        let found = ring
            .lookup(
                encode_a_record(
                    "wrap.lan",
                    [0, 0, 0, 0],
                    60,
                    PermissiveRoutingLane::PassthroughStandard,
                )
                .subject,
                OP_DNS_A,
            )
            .unwrap();
        assert_eq!(
            found.object,
            encode_a_record(
                "wrap.lan",
                [10, 0, 0, 5],
                60,
                PermissiveRoutingLane::PassthroughStandard
            )
            .object
        );
    }

    #[test]
    fn lookup_all_zero_heap() {
        let mut ring = DnsCacheRing::<16>::new();
        let q1 = encode_a_record(
            "multi.lan",
            [1, 2, 3, 4],
            60,
            PermissiveRoutingLane::PassthroughStandard,
        );
        let q2 = encode_txt_record(
            "multi.lan",
            "v=spf1",
            60,
            PermissiveRoutingLane::PassthroughStandard,
        );

        ring.insert(q1, 1);
        ring.insert(q2, 2);

        let mut out = [NQuin::default(); 4];
        let n = ring.lookup_all(q1.subject, &mut out);
        assert_eq!(n, 2);
    }
}
