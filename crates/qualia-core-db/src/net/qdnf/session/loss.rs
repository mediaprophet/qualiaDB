//! ACK ranges, PTO backoff, and sent-packet history (NET-05.08 partial).
//!
//! At most eight inclusive ACK ranges. Retransmits use fresh packet numbers
//! (`reuse_packet_number_on_retransmit` is false). Sent history is 32 in-flight
//! slots. Independent of `packet.rs` (plain `u64` packet numbers). Packages
//! remain open.

use crate::net::qdnf::errors::QdnfError;

/// Duplicate of credit.rs — do not import that module from here.
pub const MAX_ACK_RANGES: usize = 8;
/// In-flight packet-number slots (sent history bound).
pub const SENT_HISTORY: usize = 32;
/// `pto_count` saturates here: `pto_us = rtt_us * 2^min(pto_count, cap)`.
pub const PTO_BACKOFF_CAP: u32 = 8;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AckRange {
    pub start: u64, // inclusive
    pub end: u64,   // inclusive, end >= start
}

pub struct AckFrame {
    ranges: [AckRange; MAX_ACK_RANGES],
    count: u8,
    pub delay_us: u32,
}

impl AckFrame {
    pub const fn new() -> Self {
        Self {
            ranges: [AckRange { start: 0, end: 0 }; MAX_ACK_RANGES],
            count: 0,
            delay_us: 0,
        }
    }

    pub fn range_count(&self) -> usize {
        self.count as usize
    }

    pub fn range_at(&self, i: usize) -> Option<AckRange> {
        if i < self.range_count() {
            Some(self.ranges[i])
        } else {
            None
        }
    }

    pub fn contains(&self, pn: u64) -> bool {
        let n = self.range_count();
        let mut i = 0usize;
        while i < n {
            let other = self.ranges[i];
            if pn >= other.start && pn <= other.end {
                return true;
            }
            i += 1;
        }
        false
    }

    /// Insert inclusive range. Empty (`end < start`) → [`QdnfError::Range`].
    /// Overlap with an existing range (`end >= other.start && start <= other.end`)
    /// → [`QdnfError::Overlap`] (no silent merge). Ninth disjoint range →
    /// [`QdnfError::Capacity`].
    pub fn insert_range(&mut self, start: u64, end: u64) -> Result<(), QdnfError> {
        if end < start {
            return Err(QdnfError::Range);
        }
        let n = self.range_count();
        let mut i = 0;
        while i < n {
            let other = self.ranges[i];
            if end >= other.start && start <= other.end {
                return Err(QdnfError::Overlap);
            }
            i += 1;
        }
        if n >= MAX_ACK_RANGES {
            return Err(QdnfError::Capacity);
        }
        self.ranges[n] = AckRange { start, end };
        self.count = (n as u8).saturating_add(1);
        Ok(())
    }
}

/// Thirty-two slots of in-flight packet numbers.
pub struct SentTable {
    slots: [Option<u64>; SENT_HISTORY],
}

impl SentTable {
    pub const fn new() -> Self {
        Self {
            slots: [None; SENT_HISTORY],
        }
    }

    /// Record a newly sent `pn`. Duplicate occupied `pn` → [`QdnfError::Replay`].
    /// Full table → [`QdnfError::Capacity`].
    pub fn record_send(&mut self, pn: u64) -> Result<(), QdnfError> {
        let mut free: Option<usize> = None;
        let mut i = 0;
        while i < SENT_HISTORY {
            match self.slots[i] {
                Some(occupied) if occupied == pn => return Err(QdnfError::Replay),
                Some(_) => {}
                None => {
                    if free.is_none() {
                        free = Some(i);
                    }
                }
            }
            i += 1;
        }
        let idx = free.ok_or(QdnfError::Capacity)?;
        self.slots[idx] = Some(pn);
        Ok(())
    }

    /// ACK a `pn` previously recorded. Unknown `pn` → [`QdnfError::Malformed`].
    pub fn on_ack(&mut self, pn: u64) -> Result<(), QdnfError> {
        let mut i = 0;
        while i < SENT_HISTORY {
            if self.slots[i] == Some(pn) {
                self.slots[i] = None;
                return Ok(());
            }
            i += 1;
        }
        Err(QdnfError::Malformed)
    }

    pub fn in_flight_count(&self) -> usize {
        let mut n = 0;
        let mut i = 0;
        while i < SENT_HISTORY {
            if self.slots[i].is_some() {
                n += 1;
            }
            i += 1;
        }
        n
    }

    pub fn packet_at(&self, i: usize) -> Option<u64> {
        if i < SENT_HISTORY {
            self.slots[i]
        } else {
            None
        }
    }
}

/// PTO microseconds: `rtt_us * 2^pto_count` with `pto_count` capped at
/// [`PTO_BACKOFF_CAP`]. Overflow or `rtt_us == 0` → [`QdnfError::Range`].
pub fn pto_us(rtt_us: u64, pto_count: u32) -> Result<u64, QdnfError> {
    if rtt_us == 0 {
        return Err(QdnfError::Range);
    }
    let exp = pto_count.min(PTO_BACKOFF_CAP);
    let factor = 1u64.checked_shl(exp).ok_or(QdnfError::Range)?;
    rtt_us.checked_mul(factor).ok_or(QdnfError::Range)
}

pub fn reuse_packet_number_on_retransmit() -> bool {
    false
}

pub fn ack_range_cap() -> usize {
    MAX_ACK_RANGES
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eight_disjoint_ranges_ok_ninth_capacity() {
        let mut ack = AckFrame::new();
        let mut i = 0u64;
        while i < MAX_ACK_RANGES as u64 {
            let start = i * 10;
            ack.insert_range(start, start + 1)
                .expect("eight disjoint ranges fit");
            i += 1;
        }
        assert_eq!(ack.range_count(), MAX_ACK_RANGES);
        assert_eq!(ack.insert_range(800, 801), Err(QdnfError::Capacity));
        assert_eq!(ack.range_count(), MAX_ACK_RANGES);
    }

    #[test]
    fn overlapping_insert_is_overlap_count_unchanged() {
        let mut ack = AckFrame::new();
        ack.insert_range(10, 20).expect("first range");
        assert_eq!(ack.range_count(), 1);
        assert_eq!(ack.insert_range(15, 25), Err(QdnfError::Overlap));
        assert_eq!(ack.range_count(), 1);
        assert_eq!(ack.insert_range(10, 20), Err(QdnfError::Overlap));
        assert_eq!(ack.range_count(), 1);
        assert_eq!(ack.insert_range(20, 21), Err(QdnfError::Overlap));
        assert_eq!(ack.range_count(), 1);
    }

    #[test]
    fn empty_range_is_range_error() {
        let mut ack = AckFrame::new();
        assert_eq!(ack.insert_range(5, 4), Err(QdnfError::Range));
        assert_eq!(ack.range_count(), 0);
        ack.insert_range(5, 5).expect("singleton inclusive is live");
        assert_eq!(ack.range_count(), 1);
    }

    #[test]
    fn record_send_thirty_two_ok_thirty_third_capacity() {
        let mut sent = SentTable::new();
        let mut i = 0u64;
        while i < SENT_HISTORY as u64 {
            sent.record_send(i).expect("32 in-flight slots");
            i += 1;
        }
        assert_eq!(sent.in_flight_count(), SENT_HISTORY);
        assert_eq!(sent.record_send(99), Err(QdnfError::Capacity));
        assert_eq!(sent.in_flight_count(), SENT_HISTORY);
        assert_eq!(sent.record_send(0), Err(QdnfError::Replay));
    }

    #[test]
    fn on_ack_unknown_malformed_known_decreases_in_flight() {
        let mut sent = SentTable::new();
        sent.record_send(7).expect("record");
        sent.record_send(8).expect("record");
        assert_eq!(sent.in_flight_count(), 2);
        assert_eq!(sent.on_ack(99), Err(QdnfError::Malformed));
        assert_eq!(sent.in_flight_count(), 2);
        sent.on_ack(7).expect("ack recorded pn");
        assert_eq!(sent.in_flight_count(), 1);
        assert_eq!(sent.on_ack(7), Err(QdnfError::Malformed));
        assert_eq!(sent.in_flight_count(), 1);
    }

    #[test]
    fn pto_backoff_and_unknown_rtt() {
        assert_eq!(pto_us(1000, 0), Ok(1000));
        assert_eq!(pto_us(1000, 1), Ok(2000));
        assert_eq!(pto_us(0, 0), Err(QdnfError::Range));
        assert_eq!(pto_us(1000, PTO_BACKOFF_CAP), Ok(1000u64 * 256));
        assert_eq!(pto_us(1000, PTO_BACKOFF_CAP + 1), Ok(1000u64 * 256));
        assert_eq!(pto_us(u64::MAX, 1), Err(QdnfError::Range));
    }

    #[test]
    fn fresh_packet_numbers_and_ack_range_cap() {
        assert!(!reuse_packet_number_on_retransmit());
        assert_eq!(ack_range_cap(), 8);
        assert_eq!(ack_range_cap(), MAX_ACK_RANGES);
    }
}
