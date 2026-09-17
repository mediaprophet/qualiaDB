//! Membership versus index completeness for QSR (E07.4).
//!
//! A Found / EmptyInSnapshot answer is membership in a named snapshot under a
//! cover. It is not completeness. Completeness is recorded as publisher-asserted
//! or independently reconstructed from admitted covers plus matching roots.
//! Forged, missing, or partial covers cannot prove absence.

use super::cover::{covers_digit, CoverInterval};
use super::key::digit;
use super::outcome::QsrOutcome;
use super::roots::{bind_roots, IndexRoots};
use super::traversal::{lookup_into, QsrSnapshot};
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::{Generation, StrongDigest};

/// How completeness was obtained. Publisher signatures are not reconstruction.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompletenessKind {
    /// Unknown / not claimed.
    Unknown = 0,
    /// Publisher signed a completeness assertion. Not independently reconstructed.
    PublisherAsserted = 1,
    /// Local reconstruction from admitted covers + snapshot root matched compiler digest.
    IndependentlyReconstructed = 2,
}

/// Bound roots plus whether the queried prefix is independently complete.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompletenessEvidence {
    pub kind: CompletenessKind,
    pub roots: IndexRoots,
    /// True only when IndependentlyReconstructed and the cover is endpoint-complete
    /// for the queried prefix. Publisher assertion never sets this by itself.
    pub complete_for_prefix: bool,
}

/// Membership answer plus completeness. Completeness is never inferred from Found/Empty.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QsrAnswer {
    pub outcome: QsrOutcome,
    pub completeness: CompletenessEvidence,
}

/// Membership of a key in a snapshot does not imply the index is complete.
pub fn membership_implies_completeness() -> bool {
    false
}

/// Authoritative absence: EmptyInSnapshot under independently reconstructed prefix completeness.
pub fn absence_is_authoritative(answer: &QsrAnswer) -> bool {
    matches!(answer.outcome, QsrOutcome::EmptyInSnapshot)
        && answer.completeness.complete_for_prefix
        && answer.completeness.kind == CompletenessKind::IndependentlyReconstructed
}

/// Exact lookup with explicit completeness evidence. Calls [`lookup_into`] for membership.
///
/// Root disagreement is [`QsrOutcome::Conflict`] and is never treated as absence.
/// Publisher-asserted completeness is recorded; `complete_for_prefix` stays false
/// unless this node independently reconstructed matching roots and an endpoint-complete
/// cover for the queried first digit. Narrower covers are complete only for digits
/// they include; absence outside the cover is Incomplete, not EmptyInSnapshot.
pub fn lookup_with_completeness(
    snapshot: &QsrSnapshot,
    key: &StrongDigest,
    covers: &[CoverInterval],
    required_generation: Generation,
    claimed: CompletenessEvidence,
    expected_roots: &IndexRoots,
    out: &mut [StrongDigest],
) -> Result<QsrAnswer, QdnfError> {
    let bound = bind_roots(
        snapshot,
        expected_roots.compiler_digest,
        expected_roots.index_root,
    )?;
    if claimed.roots != *expected_roots || bound != *expected_roots {
        return Ok(QsrAnswer {
            outcome: QsrOutcome::Conflict,
            completeness: CompletenessEvidence {
                kind: claimed.kind,
                roots: *expected_roots,
                complete_for_prefix: false,
            },
        });
    }

    let outcome = lookup_into(snapshot, key, covers, required_generation, out)?;
    let prefix_covered = match digit(key, 0) {
        Ok(d) => covers_digit(covers, d),
        Err(_) => false,
    };
    let reconstructed = claimed.kind == CompletenessKind::IndependentlyReconstructed
        && bound == *expected_roots
        && prefix_covered
        && !matches!(outcome, QsrOutcome::Stale | QsrOutcome::Incomplete);

    let complete_for_prefix = match claimed.kind {
        CompletenessKind::IndependentlyReconstructed => reconstructed,
        CompletenessKind::PublisherAsserted | CompletenessKind::Unknown => false,
    };

    Ok(QsrAnswer {
        outcome,
        completeness: CompletenessEvidence {
            kind: claimed.kind,
            roots: bound,
            complete_for_prefix,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::super::key::KEY_DEPTH;
    use super::super::roots::bind_roots;
    use super::*;
    use crate::crypto::network::digest::sha384;

    fn full_cover() -> [CoverInterval; 1] {
        [CoverInterval { start: 0, end: 15 }]
    }

    fn compiler_index() -> (StrongDigest, StrongDigest) {
        (
            sha384(b"qdnf:qsr:compiler:test"),
            sha384(b"qdnf:qsr:index:test"),
        )
    }

    fn evidence(
        kind: CompletenessKind,
        roots: IndexRoots,
        complete_for_prefix: bool,
    ) -> CompletenessEvidence {
        CompletenessEvidence {
            kind,
            roots,
            complete_for_prefix,
        }
    }

    fn key_same_first_byte(rest: u8) -> StrongDigest {
        let mut k = StrongDigest::ZERO;
        k.0[0] = 0x10;
        let mut i = 1usize;
        while i < 48 {
            k.0[i] = rest;
            i += 1;
        }
        k
    }

    #[test]
    fn found_publisher_asserted_is_not_prefix_complete() {
        let mut key = StrongDigest::ZERO;
        key.0[0] = 0x20;
        let mut target = StrongDigest::ZERO;
        target.0[47] = 0x77;
        let mut snap = QsrSnapshot::empty(Generation(3));
        snap.insert(key, target).unwrap();
        let (compiler, index) = compiler_index();
        let roots = bind_roots(&snap, compiler, index).unwrap();
        let claimed = evidence(CompletenessKind::PublisherAsserted, roots, true);
        let mut out = [StrongDigest::ZERO; 1];
        let answer = lookup_with_completeness(
            &snap,
            &key,
            &full_cover(),
            Generation(3),
            claimed,
            &roots,
            &mut out,
        )
        .unwrap();
        assert_eq!(answer.outcome, QsrOutcome::Found { count: 1 });
        assert_eq!(
            answer.completeness.kind,
            CompletenessKind::PublisherAsserted
        );
        assert!(!answer.completeness.complete_for_prefix);
        assert!(!absence_is_authoritative(&answer));
        assert!(crate::net::qdnf::resolve::qsr::key::keys_equal(
            &out[0], &target
        ));
    }

    #[test]
    fn missing_key_partial_cover_is_incomplete_not_authoritative() {
        let snap = QsrSnapshot::empty(Generation(1));
        let (compiler, index) = compiler_index();
        let roots = bind_roots(&snap, compiler, index).unwrap();
        let claimed = evidence(CompletenessKind::IndependentlyReconstructed, roots, true);
        let covers = [CoverInterval { start: 1, end: 15 }];
        let mut out = [StrongDigest::ZERO; 1];
        let answer = lookup_with_completeness(
            &snap,
            &StrongDigest::ZERO,
            &covers,
            Generation(1),
            claimed,
            &roots,
            &mut out,
        )
        .unwrap();
        assert_eq!(answer.outcome, QsrOutcome::Incomplete);
        assert!(!answer.completeness.complete_for_prefix);
        assert!(!absence_is_authoritative(&answer));
    }

    #[test]
    fn missing_key_full_cover_reconstructed_is_authoritative_empty() {
        let snap = QsrSnapshot::empty(Generation(1));
        let (compiler, index) = compiler_index();
        let roots = bind_roots(&snap, compiler, index).unwrap();
        let claimed = evidence(CompletenessKind::IndependentlyReconstructed, roots, false);
        let mut out = [StrongDigest::ZERO; 1];
        let answer = lookup_with_completeness(
            &snap,
            &StrongDigest::ZERO,
            &full_cover(),
            Generation(1),
            claimed,
            &roots,
            &mut out,
        )
        .unwrap();
        assert_eq!(answer.outcome, QsrOutcome::EmptyInSnapshot);
        assert_eq!(
            answer.completeness.kind,
            CompletenessKind::IndependentlyReconstructed
        );
        assert!(answer.completeness.complete_for_prefix);
        assert!(absence_is_authoritative(&answer));
    }

    #[test]
    fn publisher_asserted_mismatched_snapshot_root_is_conflict() {
        let snap = QsrSnapshot::empty(Generation(1));
        let (compiler, index) = compiler_index();
        let roots = bind_roots(&snap, compiler, index).unwrap();
        let mut forged = roots;
        forged.snapshot_root.0[0] ^= 0xff;
        assert_ne!(forged.snapshot_root, roots.snapshot_root);
        let claimed = evidence(CompletenessKind::PublisherAsserted, forged, true);
        let mut out = [StrongDigest::ZERO; 1];
        let answer = lookup_with_completeness(
            &snap,
            &StrongDigest::ZERO,
            &full_cover(),
            Generation(1),
            claimed,
            &roots,
            &mut out,
        )
        .unwrap();
        assert_eq!(answer.outcome, QsrOutcome::Conflict);
        assert!(!answer.completeness.complete_for_prefix);
        assert!(!absence_is_authoritative(&answer));
    }

    #[test]
    fn forged_index_root_is_conflict() {
        let snap = QsrSnapshot::empty(Generation(1));
        let (compiler, index) = compiler_index();
        let roots = bind_roots(&snap, compiler, index).unwrap();
        let mut forged = roots;
        forged.index_root.0[47] ^= 0x01;
        assert_ne!(forged.index_root, roots.index_root);
        let claimed = evidence(CompletenessKind::IndependentlyReconstructed, forged, true);
        let mut out = [StrongDigest::ZERO; 1];
        let answer = lookup_with_completeness(
            &snap,
            &StrongDigest::ZERO,
            &full_cover(),
            Generation(1),
            claimed,
            &roots,
            &mut out,
        )
        .unwrap();
        assert_eq!(answer.outcome, QsrOutcome::Conflict);
        assert!(!absence_is_authoritative(&answer));
    }

    #[test]
    fn same_first_byte_different_key_found_only_for_exact_key() {
        let a = key_same_first_byte(0x01);
        let b = key_same_first_byte(0x02);
        assert_eq!(a.0[0], b.0[0]);
        assert_eq!(a.0[0] % KEY_DEPTH as u8, b.0[0] % KEY_DEPTH as u8);

        let mut snap = QsrSnapshot::empty(Generation(1));
        snap.insert(a, StrongDigest::ZERO).unwrap();
        let (compiler, index) = compiler_index();
        let roots = bind_roots(&snap, compiler, index).unwrap();
        let claimed = evidence(CompletenessKind::IndependentlyReconstructed, roots, true);
        let covers = full_cover();
        let mut out = [StrongDigest::ZERO; 1];

        let found =
            lookup_with_completeness(&snap, &a, &covers, Generation(1), claimed, &roots, &mut out)
                .unwrap();
        assert_eq!(found.outcome, QsrOutcome::Found { count: 1 });
        assert!(found.completeness.complete_for_prefix);
        assert!(!absence_is_authoritative(&found));

        let missing =
            lookup_with_completeness(&snap, &b, &covers, Generation(1), claimed, &roots, &mut out)
                .unwrap();
        assert_eq!(missing.outcome, QsrOutcome::EmptyInSnapshot);
        assert_ne!(missing.outcome, QsrOutcome::Found { count: 1 });
        assert!(absence_is_authoritative(&missing));
        assert_eq!(found.completeness.kind, missing.completeness.kind);
        assert_eq!(found.completeness.roots, missing.completeness.roots);
    }

    #[test]
    fn membership_does_not_imply_completeness() {
        assert!(!membership_implies_completeness());
    }

    #[test]
    fn publisher_asserted_empty_is_membership_not_authoritative_absence() {
        let snap = QsrSnapshot::empty(Generation(1));
        let (compiler, index) = compiler_index();
        let roots = bind_roots(&snap, compiler, index).unwrap();
        let claimed = evidence(CompletenessKind::PublisherAsserted, roots, true);
        let mut out = [StrongDigest::ZERO; 1];
        let answer = lookup_with_completeness(
            &snap,
            &StrongDigest::ZERO,
            &full_cover(),
            Generation(1),
            claimed,
            &roots,
            &mut out,
        )
        .unwrap();
        assert_eq!(answer.outcome, QsrOutcome::EmptyInSnapshot);
        assert!(!answer.completeness.complete_for_prefix);
        assert!(!absence_is_authoritative(&answer));
    }
}
