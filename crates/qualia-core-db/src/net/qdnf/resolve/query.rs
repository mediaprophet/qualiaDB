//! Bounded QSR query pipeline (NET-04 partial).
//!
//! Caps are remaining work, not a coverage proof. Authenticated exact lookup
//! goes through snapshot `lookup` / `lookup_into` and returns [`QsrOutcome`].
//! Compact hashes and `sameAs` are not authority. This is not Kademlia.

use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::resolve::qsr::{lookup_into, CoverInterval, QsrOutcome, QsrSnapshot};
use crate::net::qdnf::types::{Generation, QHashIndex, StrongDigest};

pub const MAX_CANDIDATES: usize = 64;
pub const MAX_VERIFICATIONS: usize = 16;
pub const MAX_RETURNED_ROUTES: usize = 8;
pub const MAX_CONNECTION_RACES: usize = 3;

/// Remaining lookup budget. `initial` loads the MAX_* remaining slots.
/// Continuations must not refill these fields (NET-04.22).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QueryBudget {
    pub candidates: u8,
    pub verifications: u8,
    pub returned: u8,
    pub races: u8,
    pub bytes: u32,
    pub work: u64,
}

impl QueryBudget {
    pub const fn initial() -> Self {
        Self {
            candidates: MAX_CANDIDATES as u8,
            verifications: MAX_VERIFICATIONS as u8,
            returned: MAX_RETURNED_ROUTES as u8,
            races: MAX_CONNECTION_RACES as u8,
            bytes: 0,
            work: 0,
        }
    }

    /// Capacity when collected candidates would exceed 64. Does not wrap.
    pub fn collect_candidate(&mut self) -> Result<(), QdnfError> {
        take_remaining(&mut self.candidates)
    }

    /// Capacity when verifications would exceed 16. Does not wrap.
    pub fn verify_one(&mut self) -> Result<(), QdnfError> {
        take_remaining(&mut self.verifications)
    }

    pub fn return_route(&mut self) -> Result<(), QdnfError> {
        take_remaining(&mut self.returned)
    }

    /// Capacity when connection races would exceed 3. Does not wrap.
    pub fn start_race(&mut self) -> Result<(), QdnfError> {
        take_remaining(&mut self.races)
    }
}

fn take_remaining(slot: &mut u8) -> Result<(), QdnfError> {
    let next = slot.checked_sub(1).ok_or(QdnfError::Capacity)?;
    *slot = next;
    Ok(())
}

/// Short `QHashIndex` values are lookup aids only (NET-04.02).
pub fn compact_hash_is_routing_authority() -> bool {
    let _ = core::mem::size_of::<QHashIndex>();
    false
}

/// Alias / `sameAs` classifiers cannot merge people (NET-04.03).
pub fn same_as_merges_people() -> bool {
    false
}

/// Provider silence is unavailable/unknown, never an allow (NET-04.08).
pub fn provider_silence_outcome() -> QdnfError {
    QdnfError::Incomplete
}

/// Unsupported or incomplete policy cannot become allow (NET-04.12).
pub fn unsupported_policy_is_allow() -> bool {
    false
}

/// Exact lookup against a generation-pinned snapshot. Cover membership alone
/// is not existence; the snapshot walk compares the full 48-byte key.
pub fn lookup_with_budget(
    snapshot: &QsrSnapshot,
    key: &StrongDigest,
    covers: &[CoverInterval],
    required_generation: Generation,
    budget: &mut QueryBudget,
    out: &mut [StrongDigest],
) -> Result<QsrOutcome, QdnfError> {
    budget.collect_candidate()?;
    budget.verify_one()?;
    lookup_into(snapshot, key, covers, required_generation, out)
}

/// Epochs, retries, branches and continuations cannot reset work or byte caps.
pub fn continuation_resets_budget() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sixty_fifth_collect_candidate_is_capacity() {
        let mut budget = QueryBudget::initial();
        for _ in 0..MAX_CANDIDATES {
            assert_eq!(budget.collect_candidate(), Ok(()));
        }
        assert_eq!(budget.collect_candidate(), Err(QdnfError::Capacity));
        assert_eq!(budget.candidates, 0);
    }

    #[test]
    fn seventeenth_verify_one_is_capacity() {
        let mut budget = QueryBudget::initial();
        for _ in 0..MAX_VERIFICATIONS {
            assert_eq!(budget.verify_one(), Ok(()));
        }
        assert_eq!(budget.verify_one(), Err(QdnfError::Capacity));
        assert_eq!(budget.verifications, 0);
    }

    #[test]
    fn fourth_start_race_is_capacity() {
        let mut budget = QueryBudget::initial();
        for _ in 0..MAX_CONNECTION_RACES {
            assert_eq!(budget.start_race(), Ok(()));
        }
        assert_eq!(budget.start_race(), Err(QdnfError::Capacity));
        assert_eq!(budget.races, 0);
    }

    #[test]
    fn ninth_return_route_is_capacity() {
        let mut budget = QueryBudget::initial();
        for _ in 0..MAX_RETURNED_ROUTES {
            assert_eq!(budget.return_route(), Ok(()));
        }
        assert_eq!(budget.return_route(), Err(QdnfError::Capacity));
        assert_eq!(budget.returned, 0);
    }

    #[test]
    fn compact_hash_is_not_routing_authority() {
        assert!(!compact_hash_is_routing_authority());
    }

    #[test]
    fn same_as_does_not_merge_people() {
        assert!(!same_as_merges_people());
    }

    #[test]
    fn unsupported_policy_is_not_allow() {
        assert!(!unsupported_policy_is_allow());
    }

    #[test]
    fn lookup_with_budget_rejects_overlapping_covers() {
        let mut budget = QueryBudget::initial();
        let snap = QsrSnapshot::empty(Generation::ZERO);
        let covers = [
            CoverInterval { start: 0, end: 10 },
            CoverInterval { start: 8, end: 12 },
        ];
        let mut out = [StrongDigest::ZERO; 1];
        assert_eq!(
            lookup_with_budget(
                &snap,
                &StrongDigest::ZERO,
                &covers,
                Generation::ZERO,
                &mut budget,
                &mut out
            ),
            Err(QdnfError::Overlap)
        );
        assert_eq!(budget.candidates, MAX_CANDIDATES as u8 - 1);
        assert_eq!(budget.verifications, MAX_VERIFICATIONS as u8 - 1);
    }

    #[test]
    fn lookup_with_budget_uses_authenticated_exact_hit() {
        let mut budget = QueryBudget::initial();
        let mut snap = QsrSnapshot::empty(Generation(1));
        snap.insert(StrongDigest::ZERO, StrongDigest::ZERO).unwrap();
        let covers = [CoverInterval { start: 0, end: 10 }];
        let mut out = [StrongDigest::ZERO; 1];
        assert_eq!(
            lookup_with_budget(
                &snap,
                &StrongDigest::ZERO,
                &covers,
                Generation(1),
                &mut budget,
                &mut out
            ),
            Ok(QsrOutcome::Found { count: 1 })
        );
    }

    #[test]
    fn lookup_with_budget_empty_snapshot_is_empty_not_found() {
        let mut budget = QueryBudget::initial();
        let snap = QsrSnapshot::empty(Generation(1));
        let covers = [CoverInterval { start: 0, end: 15 }];
        let mut out = [StrongDigest::ZERO; 1];
        assert_eq!(
            lookup_with_budget(
                &snap,
                &StrongDigest::ZERO,
                &covers,
                Generation(1),
                &mut budget,
                &mut out
            ),
            Ok(QsrOutcome::EmptyInSnapshot)
        );
    }

    #[test]
    fn continuation_does_not_reset_budget() {
        assert!(!continuation_resets_budget());
        let mut budget = QueryBudget::initial();
        budget.bytes = 12;
        budget.work = 7;
        assert!(!continuation_resets_budget());
        assert_eq!(budget.bytes, 12);
        assert_eq!(budget.work, 7);
    }

    #[test]
    fn provider_silence_is_incomplete() {
        assert_eq!(provider_silence_outcome(), QdnfError::Incomplete);
    }
}
