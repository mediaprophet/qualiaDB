//! Bounded-work quanta for network graph reads (CORE-02.03, 02.04, 02.10 partial).
//!
//! Output page size is not a work limit. Continuations bind to an exact
//! source digest, query generation, profile and scope. A recent-fact window
//! never certifies global completeness. Logical Quin count — not a small
//! physical root — decides whether a full load is admitted.

use crate::net::qdnf::errors::QdnfError;

/// Remaining scan-block, work and wall-time quanta.
///
/// Callers size output pages independently. Filling or emptying a page does
/// not consume this budget; [`ScanBudget::consume`] is the only deduction.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ScanBudget {
    pub blocks: u32,
    pub work: u64,
    pub micros: u64,
}

impl ScanBudget {
    pub const ZERO: Self = Self {
        blocks: 0,
        work: 0,
        micros: 0,
    };

    /// Deduct one quantum. Fails [`QdnfError::BudgetExhausted`] without wrapping
    /// or mutating the budget when any counter would underflow.
    pub fn consume(&mut self, blocks: u32, work: u64, micros: u64) -> Result<(), QdnfError> {
        let blocks = self
            .blocks
            .checked_sub(blocks)
            .ok_or(QdnfError::BudgetExhausted)?;
        let work = self
            .work
            .checked_sub(work)
            .ok_or(QdnfError::BudgetExhausted)?;
        let micros = self
            .micros
            .checked_sub(micros)
            .ok_or(QdnfError::BudgetExhausted)?;
        self.blocks = blocks;
        self.work = work;
        self.micros = micros;
        Ok(())
    }
}

/// Resumable network-graph continuation.
///
/// Bound to one immutable source, query generation, profile and scope. Offset
/// is progress within that bind; it is not an authorization field.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NetworkCursor {
    pub source_digest: [u8; 32],
    pub query_generation: u64,
    pub profile: u16,
    pub scope: u64,
    pub offset: u64,
}

impl NetworkCursor {
    /// Issue a continuation at offset 0 for this exact query/source/profile/scope.
    pub const fn bind(
        source_digest: [u8; 32],
        query_generation: u64,
        profile: u16,
        scope: u64,
    ) -> Self {
        Self {
            source_digest,
            query_generation,
            profile,
            scope,
            offset: 0,
        }
    }

    /// Advance `delta` only when the live query/source/profile/scope match this
    /// cursor. Stale generation is [`QdnfError::StaleGeneration`]; forged source,
    /// profile or cross-scope reuse is [`QdnfError::Unauthorized`]. Offset does
    /// not wrap.
    pub fn advance(
        &mut self,
        source_digest: &[u8; 32],
        query_generation: u64,
        profile: u16,
        scope: u64,
        delta: u64,
    ) -> Result<u64, QdnfError> {
        if self.query_generation != query_generation {
            return Err(QdnfError::StaleGeneration);
        }
        if self.source_digest != *source_digest {
            return Err(QdnfError::Unauthorized);
        }
        if self.profile != profile {
            return Err(QdnfError::Unauthorized);
        }
        if self.scope != scope {
            return Err(QdnfError::Unauthorized);
        }
        self.offset = self.offset.checked_add(delta).ok_or(QdnfError::Range)?;
        Ok(self.offset)
    }
}

/// Honesty of a bounded scan relative to the logical universe.
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Completeness {
    Complete = 0,
    IncompleteFrontier = 1,
    Unsupported = 2,
}

impl Completeness {
    /// Classify a recent-fact window. Never [`Completeness::Complete`]: a window
    /// is a frontier, not a census. `universe_unknown` yields IncompleteFrontier.
    pub fn from_window(window_size: u64, universe_unknown: bool) -> Self {
        // A finite (or empty) window is not a universe census, even when the
        // caller believes the universe is known.
        let _ = (window_size, universe_unknown);
        Self::IncompleteFrontier
    }
}

/// Admit a resident/full load from logical Quin count, not physical root size.
///
/// A tiny root (manifest pointer) must not authorize loading a logical graph
/// that exceeds `cap_quins`.
pub fn admit_logical_size(
    root_bytes: u64,
    logical_quin_count: u64,
    cap_quins: u64,
) -> Result<(), QdnfError> {
    let _ = root_bytes;
    if logical_quin_count > cap_quins {
        Err(QdnfError::Capacity)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn consume_deducts_quanta_not_page_size() {
        let mut budget = ScanBudget {
            blocks: 64,
            work: 10_000,
            micros: 5_000,
        };
        let page_slots: u32 = 8;
        budget.consume(50, 400, 200).unwrap();
        assert_eq!(budget.blocks, 14);
        assert_eq!(budget.work, 9_600);
        assert_eq!(budget.micros, 4_800);
        assert!(budget.work > page_slots as u64);
        assert!(budget.blocks > page_slots);
    }

    #[test]
    fn consume_fails_exhausted_without_wrapping_or_partial_update() {
        let mut budget = ScanBudget {
            blocks: 1,
            work: 2,
            micros: 3,
        };
        assert_eq!(budget.consume(2, 0, 0), Err(QdnfError::BudgetExhausted));
        assert_eq!(budget.blocks, 1);
        assert_eq!(budget.work, 2);
        assert_eq!(budget.micros, 3);

        assert_eq!(budget.consume(0, 3, 0), Err(QdnfError::BudgetExhausted));
        assert_eq!(budget.work, 2);

        assert_eq!(budget.consume(0, 0, 4), Err(QdnfError::BudgetExhausted));
        assert_eq!(budget.micros, 3);

        let mut saturated = ScanBudget {
            blocks: 0,
            work: 0,
            micros: 0,
        };
        assert_eq!(saturated.consume(1, 0, 0), Err(QdnfError::BudgetExhausted));
        assert_eq!(saturated, ScanBudget::ZERO);
        saturated.consume(0, 0, 0).unwrap();
    }

    #[test]
    fn advance_binds_query_source_profile_generation() {
        let digest = [0x11u8; 32];
        let mut cursor = NetworkCursor::bind(digest, 7, 0x0101, 0xA11CE);
        assert_eq!(cursor.advance(&digest, 7, 0x0101, 0xA11CE, 4).unwrap(), 4);
        assert_eq!(cursor.offset, 4);
        assert_eq!(cursor.advance(&digest, 7, 0x0101, 0xA11CE, 1).unwrap(), 5);
    }

    #[test]
    fn advance_rejects_stale_forged_and_cross_scope() {
        let digest = [0x22u8; 32];
        let mut cursor = NetworkCursor::bind(digest, 3, 9, 100);

        assert_eq!(
            cursor.advance(&digest, 2, 9, 100, 1),
            Err(QdnfError::StaleGeneration)
        );
        assert_eq!(
            cursor.advance(&digest, 4, 9, 100, 1),
            Err(QdnfError::StaleGeneration)
        );
        assert_eq!(cursor.offset, 0);

        let other_source = [0x33u8; 32];
        assert_eq!(
            cursor.advance(&other_source, 3, 9, 100, 1),
            Err(QdnfError::Unauthorized)
        );

        assert_eq!(
            cursor.advance(&digest, 3, 8, 100, 1),
            Err(QdnfError::Unauthorized)
        );

        assert_eq!(
            cursor.advance(&digest, 3, 9, 101, 1),
            Err(QdnfError::Unauthorized)
        );
        assert_eq!(cursor.offset, 0);
    }

    #[test]
    fn advance_offset_does_not_wrap() {
        let digest = [0u8; 32];
        let mut cursor = NetworkCursor::bind(digest, 1, 1, 1);
        cursor.offset = u64::MAX;
        assert_eq!(cursor.advance(&digest, 1, 1, 1, 1), Err(QdnfError::Range));
        assert_eq!(cursor.offset, u64::MAX);
    }

    #[test]
    fn recent_fact_window_never_certifies_complete() {
        assert_eq!(
            Completeness::from_window(8, true),
            Completeness::IncompleteFrontier
        );
        assert_eq!(
            Completeness::from_window(0, true),
            Completeness::IncompleteFrontier
        );
        assert_ne!(Completeness::from_window(8, false), Completeness::Complete);
        assert_ne!(
            Completeness::from_window(u64::MAX, false),
            Completeness::Complete
        );
        assert_eq!(
            Completeness::from_window(1, false),
            Completeness::IncompleteFrontier
        );
        assert_ne!(Completeness::Unsupported, Completeness::Complete);
    }

    #[test]
    fn small_root_does_not_admit_oversize_logical_graph() {
        assert_eq!(
            admit_logical_size(256, 1_000_001, 1_000_000),
            Err(QdnfError::Capacity)
        );
        admit_logical_size(256, 1_000_000, 1_000_000).unwrap();
        admit_logical_size(u64::MAX, 0, 0).unwrap();
        assert_eq!(admit_logical_size(1, 1, 0), Err(QdnfError::Capacity));
    }
}
