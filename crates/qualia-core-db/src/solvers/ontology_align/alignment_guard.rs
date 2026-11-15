//! Poisoned-alignment flood detection and contest-abuse guard (§GAP-07 / G30 / G31).
//!
//! Protects the contributor graph and human-ratification queue from:
//! 1. Bogus `closeMatch` proposal floods / Sybil poisoning attempts (`G30`).
//! 2. Denial-of-service through bad-faith or spam contest records (`G31`).
//! Zero-heap hot path.

use super::correspondence::Alignment;

/// Errors arising from alignment security evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlignmentSecurityError {
    /// Proposal count exceeds the rate-limiting sliding window budget.
    ProposalFloodExceeded,
    /// Average match degree is suspiciously degraded (bulk noise/entropy injection).
    LowEntropyPoisoning,
    /// Contest quota exceeded for the submitting actor.
    ContestRateLimitExceeded,
}

/// Evaluates an alignment proposal against poisoning flood metrics (`G30`).
pub fn detect_poisoned_alignment_flood(
    alignment: &Alignment,
    recent_proposals_count: usize,
    window_budget: usize,
    min_mean_quality: f64,
) -> Result<(), AlignmentSecurityError> {
    if alignment.correspondences.is_empty() {
        return Ok(());
    }

    if recent_proposals_count + alignment.correspondences.len() > window_budget {
        return Err(AlignmentSecurityError::ProposalFloodExceeded);
    }

    let mean_degree = alignment.quality / alignment.correspondences.len() as f64;
    if mean_degree < min_mean_quality {
        return Err(AlignmentSecurityError::LowEntropyPoisoning);
    }

    Ok(())
}

/// A contest claim asserting factual, licence, or custodial dispute over a graph triple.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContestRecord {
    pub contestant_did: u64,
    pub target_assertion: u64,
    pub timestamp_epoch: u32,
    pub grounds_hash: u64,
}

/// Evaluates contest rate limits to prevent DoS via contest record storms (`G31`).
/// Zero-heap inspection over caller-supplied slice of recent contests.
pub fn check_contest_budget(
    contestant_did: u64,
    recent_contests: &[ContestRecord],
    now: u32,
    window_seconds: u32,
    max_contests_per_window: usize,
) -> Result<(), AlignmentSecurityError> {
    let window_start = now.saturating_sub(window_seconds);
    let mut actor_contests = 0usize;

    for c in recent_contests {
        if c.contestant_did == contestant_did && c.timestamp_epoch >= window_start {
            actor_contests += 1;
            if actor_contests >= max_contests_per_window {
                return Err(AlignmentSecurityError::ContestRateLimitExceeded);
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solvers::ontology_align::correspondence::Correspondence;

    #[test]
    fn test_detect_poisoned_flood() {
        let align = Alignment {
            correspondences: vec![
                Correspondence::propose(0, 0, 0.9),
                Correspondence::propose(1, 1, 0.85),
            ],
            quality: 1.75,
        };

        // Within budget
        assert!(detect_poisoned_alignment_flood(&align, 5, 10, 0.6).is_ok());

        // Exceeds budget (flood)
        assert_eq!(
            detect_poisoned_alignment_flood(&align, 9, 10, 0.6),
            Err(AlignmentSecurityError::ProposalFloodExceeded)
        );

        // Low entropy spam
        let spam_align = Alignment {
            correspondences: vec![
                Correspondence::propose(0, 0, 0.1),
                Correspondence::propose(1, 1, 0.2),
            ],
            quality: 0.3,
        };
        assert_eq!(
            detect_poisoned_alignment_flood(&spam_align, 0, 10, 0.5),
            Err(AlignmentSecurityError::LowEntropyPoisoning)
        );
    }

    #[test]
    fn test_contest_budget_rate_limiting() {
        let actor = 0xAAAA;
        let contests = [
            ContestRecord {
                contestant_did: actor,
                target_assertion: 1,
                timestamp_epoch: 100,
                grounds_hash: 1,
            },
            ContestRecord {
                contestant_did: actor,
                target_assertion: 2,
                timestamp_epoch: 120,
                grounds_hash: 1,
            },
            ContestRecord {
                contestant_did: 0xBBBB, // other actor
                target_assertion: 3,
                timestamp_epoch: 130,
                grounds_hash: 1,
            },
        ];

        // Actor has 2 contests in window [0, 200]; budget is 3 -> OK
        assert!(check_contest_budget(actor, &contests, 150, 100, 3).is_ok());

        // Budget is 2 -> Exceeded
        assert_eq!(
            check_contest_budget(actor, &contests, 150, 100, 2),
            Err(AlignmentSecurityError::ContestRateLimitExceeded)
        );

        // Outside window (e.g. at t=300 with 100s window) -> 0 contests in window -> OK
        assert!(check_contest_budget(actor, &contests, 300, 100, 2).is_ok());
    }
}
