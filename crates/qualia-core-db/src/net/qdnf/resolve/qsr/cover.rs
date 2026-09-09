//! Parsed cover structure and coverage checks.
//!
//! Intervals are inclusive digit ranges in `0..=15` at one depth. Validation
//! rejects empty covers, inverted ranges, out-of-radix endpoints, overlaps,
//! and interior gaps. A valid cover is not an existence proof.

use super::key::{digit, DIGIT_RADIX};
use super::outcome::QsrOutcome;
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::StrongDigest;

/// Inclusive digit range `0..=15` at one trie depth.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CoverInterval {
    pub start: u8,
    pub end: u8,
}

pub fn validate_cover(children: &[CoverInterval]) -> Result<(), QdnfError> {
    if children.is_empty() {
        return Err(QdnfError::Incomplete);
    }
    let mut prev_end: Option<u8> = None;
    let mut i = 0usize;
    while i < children.len() {
        let child = children[i];
        if child.end < child.start {
            return Err(QdnfError::Malformed);
        }
        if child.end >= DIGIT_RADIX {
            return Err(QdnfError::Range);
        }
        if let Some(end) = prev_end {
            if child.start <= end {
                return Err(QdnfError::Overlap);
            }
            if child.start > end + 1 {
                return Err(QdnfError::Incomplete);
            }
        }
        prev_end = Some(child.end);
        i += 1;
    }
    Ok(())
}

/// True when `d` (0..16) lies in some validated interval. Does not validate.
pub fn covers_digit(covers: &[CoverInterval], d: u8) -> bool {
    let mut i = 0usize;
    while i < covers.len() {
        let c = covers[i];
        if d >= c.start && d <= c.end {
            return true;
        }
        i += 1;
    }
    false
}

/// Cover membership for the depth-0 digit. Not authenticated existence.
///
/// A covered digit means traversal may continue for that prefix. It does not
/// mean the key is present.
pub fn lookup_exact(key: &StrongDigest, covers: &[CoverInterval]) -> Result<QsrOutcome, QdnfError> {
    validate_cover(covers)?;
    let d = digit(key, 0)?;
    if covers_digit(covers, d) {
        Ok(QsrOutcome::NeedContinuation)
    } else {
        Ok(QsrOutcome::Incomplete)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlapping_cover_is_rejected() {
        let children = [
            CoverInterval { start: 0, end: 10 },
            CoverInterval { start: 8, end: 12 },
        ];
        assert_eq!(validate_cover(&children), Err(QdnfError::Overlap));
    }

    #[test]
    fn interior_gap_is_incomplete() {
        let children = [
            CoverInterval { start: 0, end: 4 },
            CoverInterval { start: 6, end: 15 },
        ];
        assert_eq!(validate_cover(&children), Err(QdnfError::Incomplete));
    }

    #[test]
    fn empty_cover_is_incomplete() {
        assert_eq!(validate_cover(&[]), Err(QdnfError::Incomplete));
    }

    #[test]
    fn inverted_interval_is_malformed() {
        let children = [CoverInterval { start: 9, end: 3 }];
        assert_eq!(validate_cover(&children), Err(QdnfError::Malformed));
    }

    #[test]
    fn end_sixteen_is_range() {
        let children = [CoverInterval { start: 0, end: 16 }];
        assert_eq!(validate_cover(&children), Err(QdnfError::Range));
    }

    #[test]
    fn adjacent_full_radix_is_ok() {
        let children = [
            CoverInterval { start: 0, end: 7 },
            CoverInterval { start: 8, end: 15 },
        ];
        assert_eq!(validate_cover(&children), Ok(()));
    }

    #[test]
    fn lookup_exact_is_cover_membership_not_existence() {
        let covers = [CoverInterval { start: 0, end: 10 }];
        assert_eq!(
            lookup_exact(&StrongDigest::ZERO, &covers),
            Ok(QsrOutcome::NeedContinuation)
        );
        let uncovered = [CoverInterval { start: 1, end: 15 }];
        assert_eq!(
            lookup_exact(&StrongDigest::ZERO, &uncovered),
            Ok(QsrOutcome::Incomplete)
        );
    }
}
