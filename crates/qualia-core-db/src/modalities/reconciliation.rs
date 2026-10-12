//! Corpus reconciliation and vintage staleness engine (§GAP-11, reconciliation.md).
//!
//! Models the temporal distance between the internal view (held state stamped with vintage)
//! and the external view (authoritative upstream or live feed). Replaces silent overwrites
//! with governed comparison (`Converged`, `Diverged`, `Deferred`, `Contested`), queryable staleness,
//! and `reCheckable` cascades on derived downstream claims. Zero-heap hot path.

use crate::{q_hash, NQuin};

/// Bit flag in Quin metadata (bit 15) indicating a downstream claim is re-checkable due to upstream vintage advance.
pub const RECHECKABLE_FLAG: u64 = 1 << 15;

/// Outcome status of an internal-external view reconciliation.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReconciliationStatus {
    /// Internal view matches external view.
    #[default]
    Converged = 0,
    /// Upstream vintage or content has advanced beyond held state.
    Diverged = 1,
    /// Upstream source update received but reconciliation intentionally deferred (e.g. pending review).
    Deferred = 2,
    /// Ground-truth dispute or conflicting source data recorded.
    Contested = 3,
}

/// A reconciliation event record measuring vintage distance and state agreement.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReconciliationRecord {
    pub entity_id: u64,
    pub internal_vintage: u32,
    pub external_vintage: u32,
    pub reconciled_at: u32,
    pub expected_lag_s: u32,
    pub status: ReconciliationStatus,
    pub divergence_score: f32,
}

impl ReconciliationRecord {
    /// Whether the held state is considered stale at `now` relative to `expected_lag_s`.
    #[inline]
    pub fn is_stale(&self, now: u32) -> bool {
        let elapsed = now.saturating_sub(self.reconciled_at);
        elapsed > self.expected_lag_s || self.external_vintage > self.internal_vintage
    }
}

/// Compare internal view against external view to produce a reconciliation record.
/// Pure function, zero-heap.
pub fn compare_views(
    entity_id: u64,
    internal_fingerprint: u64,
    external_fingerprint: u64,
    internal_vintage: u32,
    external_vintage: u32,
    now: u32,
    expected_lag_s: u32,
) -> ReconciliationRecord {
    let (status, divergence_score) = if internal_fingerprint == external_fingerprint {
        (ReconciliationStatus::Converged, 0.0)
    } else if external_vintage > internal_vintage {
        let vintage_delta = (external_vintage - internal_vintage) as f32;
        (ReconciliationStatus::Diverged, vintage_delta.min(1.0))
    } else {
        // Same vintage but differing content indicates a contested assertion or alternate edition
        (ReconciliationStatus::Contested, 0.5)
    };

    ReconciliationRecord {
        entity_id,
        internal_vintage,
        external_vintage,
        reconciled_at: now,
        expected_lag_s,
        status,
        divergence_score,
    }
}

/// Cascades re-check flags onto downstream derived claims when an upstream reconciliation diverged.
/// Sets `RECHECKABLE_FLAG` (bit 15) in the metadata field of each matching quin.
/// Returns the number of updated quins. Zero-heap caller-buffered.
pub fn cascade_recheck_flags(
    record: &ReconciliationRecord,
    downstream_claims: &mut [NQuin],
) -> usize {
    if record.status == ReconciliationStatus::Converged {
        return 0;
    }

    let mut updated = 0usize;
    for q in downstream_claims.iter_mut() {
        if q.context == record.entity_id || q.subject == record.entity_id {
            if (q.metadata & RECHECKABLE_FLAG) == 0 {
                q.metadata |= RECHECKABLE_FLAG;
                q.parity = q.subject ^ q.predicate ^ q.object ^ q.context;
                updated += 1;
            }
        }
    }
    updated
}

/// Predicate URI for reconciliation event (`qualia:reconciliation:Event`).
#[inline]
pub fn reconciliation_event_predicate() -> u64 {
    q_hash("qualia:reconciliation:Event")
}

/// Predicate URI for staleness status (`qualia:reconciliation:isStale`).
#[inline]
pub fn is_stale_predicate() -> u64 {
    q_hash("qualia:reconciliation:isStale")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compare_views_converged() {
        let rec = compare_views(0x123, 0xABC, 0xABC, 2026, 2026, 1000, 3600);
        assert_eq!(rec.status, ReconciliationStatus::Converged);
        assert_eq!(rec.divergence_score, 0.0);
        assert!(!rec.is_stale(1500));
        assert!(rec.is_stale(5000)); // exceeded lag
    }

    #[test]
    fn test_compare_views_diverged() {
        let rec = compare_views(0x123, 0xABC, 0xDEF, 2024, 2026, 1000, 3600);
        assert_eq!(rec.status, ReconciliationStatus::Diverged);
        assert!(rec.is_stale(1050)); // upstream vintage is newer
    }

    #[test]
    fn test_cascade_recheck_flags() {
        let rec = compare_views(0x999, 0x111, 0x222, 1, 2, 100, 300);
        let mut q1 = NQuin {
            subject: 0x999,
            predicate: q_hash("pred:metric"),
            object: 100,
            context: 0,
            metadata: 0,
            parity: 0,
        };
        let mut q2 = NQuin {
            subject: 0x888,
            predicate: q_hash("pred:unrelated"),
            object: 200,
            context: 0,
            metadata: 0,
            parity: 0,
        };

        let mut claims = [q1, q2];
        let count = cascade_recheck_flags(&rec, &mut claims);
        assert_eq!(count, 1);
        assert_ne!(claims[0].metadata & RECHECKABLE_FLAG, 0);
        assert_eq!(claims[1].metadata & RECHECKABLE_FLAG, 0);
    }
}
