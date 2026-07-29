//! ByteTrack-style detection score tier classification.
//!
//! High-score detections drive first association + track birth.
//! Low-score detections only recover unmatched tracks (second association).
//! Below `low` is rejected for association.

/// Detection confidence band for two-stage association.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScoreTier {
    /// `score >= high` — primary match + may spawn tracks.
    High = 0,
    /// `low <= score < high` — secondary match only (no birth).
    Low = 1,
    /// `score < low` — ignored for association.
    Reject = 2,
}

/// Classify a packed detection score into a ByteTrack tier.
///
/// When `high <= low`, the Low band collapses: scores `>= high` are High,
/// everything else Reject (deterministic fail-closed for misconfigured thresholds).
#[inline]
pub fn score_tier_of(score_u16: u16, high_score_u16: u16, low_score_u16: u16) -> ScoreTier {
    if score_u16 >= high_score_u16 {
        ScoreTier::High
    } else if high_score_u16 > low_score_u16 && score_u16 >= low_score_u16 {
        ScoreTier::Low
    } else {
        ScoreTier::Reject
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tiers_partition_scores() {
        assert_eq!(score_tier_of(50_000, 32_768, 6_554), ScoreTier::High);
        assert_eq!(score_tier_of(32_768, 32_768, 6_554), ScoreTier::High);
        assert_eq!(score_tier_of(10_000, 32_768, 6_554), ScoreTier::Low);
        assert_eq!(score_tier_of(6_554, 32_768, 6_554), ScoreTier::Low);
        assert_eq!(score_tier_of(100, 32_768, 6_554), ScoreTier::Reject);
    }

    #[test]
    fn collapsed_thresholds_reject_low_band() {
        assert_eq!(score_tier_of(40_000, 10_000, 10_000), ScoreTier::High);
        assert_eq!(score_tier_of(9_999, 10_000, 10_000), ScoreTier::Reject);
    }
}
