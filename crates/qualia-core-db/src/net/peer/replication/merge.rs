//! Dataset merge/conflict: retain concurrent alternatives (SVC-01.10 partial).
//!
//! Membership and revocation are independent of LWW and wall-clock. A Lamport
//! field on an alternative is a hint only and never selects a winner. Two
//! non-revoked concurrent alternatives remain both retained; [`decide`] reports
//! [`QdnfError::Conflict`] so the caller keeps them. Packages remain open.

use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::StrongDigest;

pub const MAX_ALTERNATIVES: usize = 4;

/// Accepted merge/conflict profile for one logical key.
///
/// [`MergeProfile::LwwHint`] must not drop a non-revoked concurrent alternative.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MergeProfile {
    ConcurrentRetain = 1,
    LwwHint = 2,
}

/// One concurrent alternative for a logical key. `lamport` is never authority.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Alternate {
    pub op_id: StrongDigest,
    pub lamport: u64,
}

#[derive(Clone, Copy)]
struct Slot {
    occupied: bool,
    alt: Alternate,
}

/// Up to [`MAX_ALTERNATIVES`] concurrent alternatives for one logical key.
pub struct MergeSet {
    slots: [Slot; MAX_ALTERNATIVES],
}

impl MergeSet {
    pub const fn new() -> Self {
        Self {
            slots: [Slot {
                occupied: false,
                alt: Alternate {
                    op_id: StrongDigest::ZERO,
                    lamport: 0,
                },
            }; MAX_ALTERNATIVES],
        }
    }

    /// Insert a non-revoked alternative. Revocation cannot be overridden by LWW.
    ///
    /// [`QdnfError::Revoked`] if `revoked` is true (set is unchanged).
    /// Duplicate `op_id` is idempotent [`Ok`].
    /// A fifth distinct id is [`QdnfError::Capacity`].
    pub fn insert(&mut self, alt: Alternate, revoked: bool) -> Result<(), QdnfError> {
        if revoked {
            return Err(QdnfError::Revoked);
        }
        if self.find_id(&alt.op_id).is_some() {
            return Ok(());
        }
        let idx = self.find_free().ok_or(QdnfError::Capacity)?;
        self.slots[idx] = Slot {
            occupied: true,
            alt,
        };
        Ok(())
    }

    pub fn len(&self) -> usize {
        let mut n = 0usize;
        let mut i = 0usize;
        while i < MAX_ALTERNATIVES {
            if self.slots[i].occupied {
                n += 1;
            }
            i += 1;
        }
        n
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn find_id(&self, id: &StrongDigest) -> Option<usize> {
        let mut i = 0usize;
        while i < MAX_ALTERNATIVES {
            if self.slots[i].occupied && self.slots[i].alt.op_id == *id {
                return Some(i);
            }
            i += 1;
        }
        None
    }

    fn find_free(&self) -> Option<usize> {
        let mut i = 0usize;
        while i < MAX_ALTERNATIVES {
            if !self.slots[i].occupied {
                return Some(i);
            }
            i += 1;
        }
        None
    }

    fn first_occupied(&self) -> Option<StrongDigest> {
        let mut i = 0usize;
        while i < MAX_ALTERNATIVES {
            if self.slots[i].occupied {
                return Some(self.slots[i].alt.op_id);
            }
            i += 1;
        }
        None
    }
}

impl Default for MergeSet {
    fn default() -> Self {
        Self::new()
    }
}

/// Last-writer-wins cannot override membership or revocation authority.
pub fn lww_overrides_revocation() -> bool {
    false
}

/// Wall-clock ordering is not membership authority.
pub fn wall_clock_is_membership_authority() -> bool {
    false
}

/// Apply `profile` to `set`. LWW/wall-clock never drop a non-revoked concurrent alt.
///
/// Empty set → [`QdnfError::Unauthorized`].
/// `len > 1` → [`QdnfError::Conflict`] (retain alternatives; do not pick a winner).
/// `len == 1` → `Ok` of that operation id, for either profile.
pub fn decide(set: &MergeSet, profile: MergeProfile) -> Result<StrongDigest, QdnfError> {
    match set.len() {
        0 => Err(QdnfError::Unauthorized),
        1 => set.first_occupied().ok_or(QdnfError::Unauthorized),
        _ => match profile {
            MergeProfile::ConcurrentRetain | MergeProfile::LwwHint => Err(QdnfError::Conflict),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn alt(tag: u8, lamport: u64) -> Alternate {
        let mut id = StrongDigest::ZERO;
        id.0[0] = tag;
        Alternate { op_id: id, lamport }
    }

    #[test]
    fn lww_and_wall_clock_are_not_authority() {
        assert!(!lww_overrides_revocation());
        assert!(!wall_clock_is_membership_authority());
    }

    #[test]
    fn insert_four_ok_fifth_is_capacity() {
        let mut set = MergeSet::new();
        let mut i = 1u8;
        while i <= MAX_ALTERNATIVES as u8 {
            set.insert(alt(i, u64::from(i) * 10), false).unwrap();
            i += 1;
        }
        assert_eq!(set.len(), MAX_ALTERNATIVES);
        assert_eq!(set.insert(alt(99, 1), false), Err(QdnfError::Capacity));
        assert_eq!(set.len(), MAX_ALTERNATIVES);
    }

    #[test]
    fn insert_revoked_is_revoked_len_unchanged() {
        let mut set = MergeSet::new();
        set.insert(alt(1, 5), false).unwrap();
        assert_eq!(set.len(), 1);
        assert_eq!(set.insert(alt(2, 99), true), Err(QdnfError::Revoked));
        assert_eq!(set.len(), 1);
        assert_eq!(
            decide(&set, MergeProfile::ConcurrentRetain).unwrap(),
            alt(1, 5).op_id
        );
    }

    #[test]
    fn concurrent_retain_two_alts_is_conflict() {
        let mut set = MergeSet::new();
        set.insert(alt(1, 1), false).unwrap();
        set.insert(alt(2, 9), false).unwrap();
        assert_eq!(set.len(), 2);
        assert_eq!(
            decide(&set, MergeProfile::ConcurrentRetain),
            Err(QdnfError::Conflict)
        );
    }

    #[test]
    fn single_alt_concurrent_retain_is_ok() {
        let mut set = MergeSet::new();
        assert_eq!(
            decide(&set, MergeProfile::ConcurrentRetain),
            Err(QdnfError::Unauthorized)
        );
        set.insert(alt(7, 3), false).unwrap();
        assert_eq!(
            decide(&set, MergeProfile::ConcurrentRetain).unwrap(),
            alt(7, 3).op_id
        );
    }

    #[test]
    fn lww_hint_two_alts_is_still_conflict() {
        let mut set = MergeSet::new();
        set.insert(alt(1, 1), false).unwrap();
        set.insert(alt(2, 100), false).unwrap();
        assert_eq!(
            decide(&set, MergeProfile::LwwHint),
            Err(QdnfError::Conflict)
        );
        let mut one = MergeSet::new();
        one.insert(alt(3, 0), false).unwrap();
        assert_eq!(
            decide(&one, MergeProfile::LwwHint).unwrap(),
            alt(3, 0).op_id
        );
    }

    #[test]
    fn duplicate_insert_is_idempotent() {
        let mut set = MergeSet::new();
        let a = alt(4, 1);
        set.insert(a, false).unwrap();
        set.insert(
            Alternate {
                op_id: a.op_id,
                lamport: 50,
            },
            false,
        )
        .unwrap();
        assert_eq!(set.len(), 1);
        assert_eq!(
            decide(&set, MergeProfile::ConcurrentRetain).unwrap(),
            a.op_id
        );
    }
}
