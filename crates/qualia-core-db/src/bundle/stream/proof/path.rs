//! Shared bounded tree-shape path for canonical inclusion proofs.
use super::{ProofError, LEAF_BYTES, MAX_LOGICAL_BYTES, MAX_SIBLINGS};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SiblingSide {
    Left,
    Right,
}

/// Canonical sibling subtree for one leaf, in the bottom-up order consumed by
/// `verify_leaf` and `encode_proof`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ProofSiblingRange {
    pub start_leaf: u64,
    pub leaf_count: u64,
    pub side: SiblingSide,
}

impl ProofSiblingRange {
    pub(crate) const EMPTY: Self = Self {
        start_leaf: 0,
        leaf_count: 0,
        side: SiblingSide::Right,
    };
}

/// Write the canonical sibling ranges for `leaf_index` into caller storage.
/// Ranges are bottom-up, matching the existing proof encoding. This helper
/// shares the tree split rule with the verifier; it computes shape only and
/// does not authenticate any digest or leaf bytes.
pub(crate) fn sibling_ranges(
    leaf_count: u64,
    leaf_index: u64,
    out: &mut [ProofSiblingRange],
) -> Result<usize, ProofError> {
    if leaf_count == 0 || leaf_index >= leaf_count {
        return Err(ProofError::Index);
    }
    let max_leaf_count =
        MAX_LOGICAL_BYTES / LEAF_BYTES + (MAX_LOGICAL_BYTES % LEAF_BYTES != 0) as u64;
    if leaf_count > max_leaf_count {
        return Err(ProofError::Limit);
    }

    let mut pending = [ProofSiblingRange::EMPTY; MAX_SIBLINGS];
    let mut depth = 0usize;
    let mut base = 0u64;
    let mut local = leaf_index;
    let mut remaining = leaf_count;

    while remaining > 1 {
        if depth == MAX_SIBLINGS {
            return Err(ProofError::Limit);
        }
        let split = split_point(remaining);
        if local < split {
            let start = base.checked_add(split).ok_or(ProofError::Index)?;
            let right_count = remaining - split;
            if right_count == 0 {
                return Err(ProofError::Index);
            }
            pending[depth] = ProofSiblingRange {
                start_leaf: start,
                leaf_count: right_count,
                side: SiblingSide::Right,
            };
            remaining = split;
        } else {
            pending[depth] = ProofSiblingRange {
                start_leaf: base,
                leaf_count: split,
                side: SiblingSide::Left,
            };
            base = base.checked_add(split).ok_or(ProofError::Index)?;
            local = local.checked_sub(split).ok_or(ProofError::Index)?;
            remaining = remaining.checked_sub(split).ok_or(ProofError::Index)?;
        }
        depth += 1;
    }

    if out.len() < depth {
        return Err(ProofError::Capacity);
    }
    for (dst, src) in out[..depth].iter_mut().zip(pending[..depth].iter().rev()) {
        *dst = *src;
    }
    Ok(depth)
}

/// Existing right-balanced tree rule: split at the largest power of two
/// strictly less than the current subtree size.
fn split_point(count: u64) -> u64 {
    debug_assert!(count > 1);
    1u64 << (63 - (count - 1).leading_zeros())
}

#[cfg(test)]
#[path = "path_tests.rs"]
mod tests;
