//! Public staged QSR lookup: exact cover/snapshot, then tokens, then handover.
//!
//! [`lookup_exact`] is cover membership (`NeedContinuation` / `Incomplete`), not
//! existence. Authenticated membership uses the snapshot. Found is membership,
//! not completeness. Four-bit digits remain indexing only.

use super::cover::{lookup_exact, CoverInterval};
use super::handover::{lookup_via_handover, Handover};
use super::outcome::QsrOutcome;
use super::tokens::{lookup_private, PrivateToken};
use super::traversal::{lookup_into, QsrSnapshot};
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::{Generation, StrongDigest};

/// Scoped private-token stage. Admission happens before the private snapshot.
#[derive(Clone, Copy, Debug)]
pub struct TokenStage<'a> {
    pub token: &'a PrivateToken,
    pub expected: &'a PrivateToken,
    pub snapshot: &'a QsrSnapshot,
    pub covers: &'a [CoverInterval],
    pub generation: Generation,
}

/// Writer-handover stage. Successor snapshot is consulted only after transfer.
#[derive(Clone, Copy, Debug)]
pub struct HandoverStage<'a> {
    pub handover: &'a Handover,
    pub snapshot: &'a QsrSnapshot,
    pub covers: &'a [CoverInterval],
    pub generation: Generation,
}

/// Public QSR lookup surface. `NativePeer::lookup_qsr` is cover-only
/// [`lookup_exact`]; `NativePeer::lookup_qsr_full` calls [`Qsr::lookup`].
pub struct Qsr;

impl Qsr {
    /// Exact cover/snapshot first, then scoped tokens, then writer handover.
    ///
    /// Wrong tokens are Unauthorized and do not fall through. Found does not
    /// imply index completeness.
    pub fn lookup(
        key: &StrongDigest,
        covers: &[CoverInterval],
        snapshot: &QsrSnapshot,
        required_generation: Generation,
        token: Option<TokenStage<'_>>,
        handover: Option<HandoverStage<'_>>,
        out: &mut [StrongDigest],
    ) -> Result<QsrOutcome, QdnfError> {
        lookup_staged(
            key,
            covers,
            snapshot,
            required_generation,
            token,
            handover,
            out,
        )
    }
}

pub(super) fn lookup_staged(
    key: &StrongDigest,
    covers: &[CoverInterval],
    snapshot: &QsrSnapshot,
    required_generation: Generation,
    token: Option<TokenStage<'_>>,
    handover: Option<HandoverStage<'_>>,
    out: &mut [StrongDigest],
) -> Result<QsrOutcome, QdnfError> {
    let cover = lookup_exact(key, covers)?;
    let exact = if cover == QsrOutcome::NeedContinuation {
        lookup_into(snapshot, key, covers, required_generation, out)?
    } else {
        cover
    };
    if terminal_membership(exact) {
        return Ok(exact);
    }

    if let Some(stage) = token {
        let tok = lookup_private(
            stage.token,
            stage.expected,
            stage.snapshot,
            key,
            stage.covers,
            stage.generation,
            out,
        )?;
        if terminal_membership(tok) {
            return Ok(tok);
        }
    }

    if let Some(stage) = handover {
        return lookup_via_handover(
            stage.handover,
            stage.snapshot,
            key,
            stage.covers,
            stage.generation,
            out,
        );
    }

    Ok(exact)
}

fn terminal_membership(outcome: QsrOutcome) -> bool {
    matches!(
        outcome,
        QsrOutcome::Found { .. }
            | QsrOutcome::Conflict
            | QsrOutcome::Stale
            | QsrOutcome::NeedContinuation
    )
}

#[cfg(test)]
mod tests {
    use super::super::completeness::membership_implies_completeness;
    use super::super::handover::{
        accept_mutation, drain_accepted, fence_old, persist_boundary, Handover,
    };
    use super::super::key::{digit, keys_equal, DIGIT_RADIX};
    use super::super::tokens::issue_token;
    use super::*;

    fn key_rest(rest: u8) -> StrongDigest {
        let mut k = StrongDigest::ZERO;
        k.0[0] = 0x10;
        let mut i = 1usize;
        while i < 48 {
            k.0[i] = rest;
            i += 1;
        }
        k
    }

    fn full_cover() -> [CoverInterval; 1] {
        [CoverInterval { start: 0, end: 15 }]
    }

    fn uncovered_first_digit() -> [CoverInterval; 1] {
        // Depth-0 digit of `key_rest` is 0x1 (high nibble of 0x10). 2..=15 misses it.
        [CoverInterval { start: 2, end: 15 }]
    }

    #[test]
    fn exact_miss_token_hit() {
        let key = key_rest(0x21);
        assert_eq!(digit(&key, 0).unwrap(), 0x01);
        assert_eq!(DIGIT_RADIX, 16);
        let miss_covers = uncovered_first_digit();
        assert_eq!(lookup_exact(&key, &miss_covers), Ok(QsrOutcome::Incomplete));

        let public = QsrSnapshot::empty(Generation(1));
        let mut private = QsrSnapshot::empty(Generation(1));
        let mut target = StrongDigest::ZERO;
        target.0[47] = 0x5a;
        private.insert(key, target).unwrap();
        let token = issue_token(&key, 4, 1);
        let stage = TokenStage {
            token: &token,
            expected: &token,
            snapshot: &private,
            covers: &full_cover(),
            generation: Generation(1),
        };
        let mut out = [StrongDigest::ZERO; 1];
        let outcome = Qsr::lookup(
            &key,
            &miss_covers,
            &public,
            Generation(1),
            Some(stage),
            None,
            &mut out,
        )
        .unwrap();
        assert_eq!(outcome, QsrOutcome::Found { count: 1 });
        assert!(keys_equal(&out[0], &target));
        assert!(!membership_implies_completeness());
        assert_ne!(outcome, QsrOutcome::EmptyInSnapshot);
        assert_ne!(outcome, QsrOutcome::Incomplete);
    }

    #[test]
    fn exact_miss_handover_hit() {
        let key = key_rest(0x22);
        let miss_covers = uncovered_first_digit();
        assert_eq!(lookup_exact(&key, &miss_covers), Ok(QsrOutcome::Incomplete));

        let public = QsrSnapshot::empty(Generation(2));
        let mut successor = QsrSnapshot::empty(Generation(2));
        let mut target = StrongDigest::ZERO;
        target.0[0] = 0x44;
        successor.insert(key, target).unwrap();

        let mut h = Handover::start(1, 2);
        accept_mutation(&mut h).unwrap();
        fence_old(&mut h);
        drain_accepted(&mut h);
        persist_boundary(&mut h);

        let stage = HandoverStage {
            handover: &h,
            snapshot: &successor,
            covers: &full_cover(),
            generation: Generation(2),
        };
        let mut out = [StrongDigest::ZERO; 1];
        let outcome = super::super::lookup_qsr_full(
            &key,
            &miss_covers,
            &public,
            Generation(2),
            None,
            Some(stage),
            &mut out,
        )
        .unwrap();
        assert_eq!(outcome, QsrOutcome::Found { count: 1 });
        assert!(keys_equal(&out[0], &target));
        assert!(!membership_implies_completeness());
    }

    #[test]
    fn wrong_token_does_not_fall_through_to_handover() {
        let key = key_rest(0x23);
        let miss_covers = uncovered_first_digit();
        let public = QsrSnapshot::empty(Generation(1));
        let mut private = QsrSnapshot::empty(Generation(1));
        private.insert(key, StrongDigest::ZERO).unwrap();
        let expected = issue_token(&key, 1, 1);
        let wrong = issue_token(&key, 1, 2);
        let mut h = Handover::start(1, 2);
        accept_mutation(&mut h).unwrap();
        fence_old(&mut h);
        drain_accepted(&mut h);
        persist_boundary(&mut h);
        let mut successor = QsrSnapshot::empty(Generation(1));
        successor.insert(key, StrongDigest::ZERO).unwrap();
        let mut out = [StrongDigest::ZERO; 1];
        let result = Qsr::lookup(
            &key,
            &miss_covers,
            &public,
            Generation(1),
            Some(TokenStage {
                token: &wrong,
                expected: &expected,
                snapshot: &private,
                covers: &full_cover(),
                generation: Generation(1),
            }),
            Some(HandoverStage {
                handover: &h,
                snapshot: &successor,
                covers: &full_cover(),
                generation: Generation(1),
            }),
            &mut out,
        );
        assert_eq!(result, Err(QdnfError::Unauthorized));
        assert_ne!(result, Ok(QsrOutcome::Found { count: 1 }));
    }
}
