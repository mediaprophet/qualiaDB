//! Writer epoch handover and fencing (E07.7).
//!
//! Fence old admission, drain accepted writes, persist the boundary, then
//! activate the successor. Two Active writers are Ambiguous; activate_new
//! returns Conflict. Writes after fence are Unauthorized. Drain copies a
//! bounded accepted-count so no acknowledged mutation is dropped.

use super::cover::CoverInterval;
use super::outcome::QsrOutcome;
use super::traversal::{lookup_into, QsrSnapshot};
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::{Generation, StrongDigest};

/// Maximum acknowledged mutations retained across drain/transfer.
pub const ACCEPTED_CAP: u8 = 8;

/// Writer generation state. Two Active owners become Ambiguous.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WriterEpoch {
    Active = 0,
    Fenced = 1,
    Draining = 2,
    Transferred = 3,
    Ambiguous = 4,
}

/// Old/new epoch pair plus fencing state and bounded accepted count.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Handover {
    pub old: u64,
    pub new: u64,
    pub state: WriterEpoch,
    accepted: u8,
    transferred: u8,
}

impl Handover {
    /// Old writer is Active. `new` is the intended successor epoch.
    pub const fn start(old: u64, new: u64) -> Self {
        Self {
            old,
            new,
            state: WriterEpoch::Active,
            accepted: 0,
            transferred: 0,
        }
    }

    /// Competing Active writers. Ownership is unavailable.
    pub const fn ambiguous(old: u64, new: u64) -> Self {
        Self {
            old,
            new,
            state: WriterEpoch::Ambiguous,
            accepted: 0,
            transferred: 0,
        }
    }

    #[inline]
    pub const fn accepted_count(&self) -> u8 {
        self.accepted
    }

    #[inline]
    pub const fn transferred_count(&self) -> u8 {
        self.transferred
    }
}

/// Stop new write admission. Active → Fenced.
pub fn fence_old(h: &mut Handover) {
    if h.state == WriterEpoch::Active {
        h.state = WriterEpoch::Fenced;
    }
}

/// Finish admitted work. Fenced → Draining. Accepted count is retained.
pub fn drain_accepted(h: &mut Handover) {
    if h.state == WriterEpoch::Fenced {
        h.state = WriterEpoch::Draining;
    }
}

/// Commit the final accepted sequence. Draining → Transferred.
/// Copies the bounded accepted-count; no acknowledged mutation is dropped.
pub fn persist_boundary(h: &mut Handover) {
    if h.state == WriterEpoch::Draining {
        h.transferred = h.accepted;
        h.state = WriterEpoch::Transferred;
    }
}

/// Activate the successor. Transferred → Active.
///
/// Two Active writers (or Ambiguous) return Conflict. Incomplete cutover
/// (Fenced/Draining) is unavailable.
pub fn activate_new(h: &mut Handover) -> Result<(), QdnfError> {
    match h.state {
        WriterEpoch::Transferred => {
            h.state = WriterEpoch::Active;
            Ok(())
        }
        WriterEpoch::Active => {
            h.state = WriterEpoch::Ambiguous;
            Err(QdnfError::Conflict)
        }
        WriterEpoch::Ambiguous => Err(QdnfError::Conflict),
        WriterEpoch::Fenced | WriterEpoch::Draining => Err(QdnfError::Incomplete),
    }
}

/// New mutation admission. Only the single Active writer may accept writes.
pub fn admit_write(h: &Handover) -> Result<(), QdnfError> {
    match h.state {
        WriterEpoch::Active => Ok(()),
        WriterEpoch::Fenced | WriterEpoch::Draining | WriterEpoch::Transferred => {
            Err(QdnfError::Unauthorized)
        }
        WriterEpoch::Ambiguous => Err(QdnfError::Conflict),
    }
}

/// Acknowledge a mutation under the Active writer. Bounded by [`ACCEPTED_CAP`].
pub fn accept_mutation(h: &mut Handover) -> Result<u8, QdnfError> {
    admit_write(h)?;
    if h.accepted >= ACCEPTED_CAP {
        return Err(QdnfError::Capacity);
    }
    h.accepted = h.accepted + 1;
    Ok(h.accepted)
}

/// Successor snapshot lookup after a completed transfer.
///
/// Transferred, or Active with a persisted boundary, may answer. Ambiguous
/// ownership is Conflict. An incomplete cutover is Incomplete, not Found.
pub fn lookup_via_handover(
    h: &Handover,
    snapshot: &QsrSnapshot,
    key: &StrongDigest,
    covers: &[CoverInterval],
    required_generation: Generation,
    out: &mut [StrongDigest],
) -> Result<QsrOutcome, QdnfError> {
    match h.state {
        WriterEpoch::Transferred => {}
        WriterEpoch::Active if h.transferred > 0 => {}
        WriterEpoch::Ambiguous => return Ok(QsrOutcome::Conflict),
        WriterEpoch::Fenced | WriterEpoch::Draining | WriterEpoch::Active => {
            return Ok(QsrOutcome::Incomplete);
        }
    }
    lookup_into(snapshot, key, covers, required_generation, out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fence_rejects_writes() {
        let mut h = Handover::start(1, 2);
        assert_eq!(accept_mutation(&mut h), Ok(1));
        fence_old(&mut h);
        assert_eq!(h.state, WriterEpoch::Fenced);
        assert_eq!(admit_write(&h), Err(QdnfError::Unauthorized));
        assert_eq!(accept_mutation(&mut h), Err(QdnfError::Unauthorized));
        assert_eq!(h.accepted_count(), 1);
    }

    #[test]
    fn drain_retains_accepted_and_transfer_copies_count() {
        let mut h = Handover::start(3, 4);
        accept_mutation(&mut h).unwrap();
        accept_mutation(&mut h).unwrap();
        fence_old(&mut h);
        drain_accepted(&mut h);
        assert_eq!(h.state, WriterEpoch::Draining);
        assert_eq!(h.accepted_count(), 2);
        persist_boundary(&mut h);
        assert_eq!(h.state, WriterEpoch::Transferred);
        assert_eq!(h.transferred_count(), 2);
        assert_eq!(h.transferred_count(), h.accepted_count());
        assert_eq!(admit_write(&h), Err(QdnfError::Unauthorized));
    }

    #[test]
    fn activate_after_transfer() {
        let mut h = Handover::start(5, 6);
        fence_old(&mut h);
        drain_accepted(&mut h);
        persist_boundary(&mut h);
        assert_eq!(activate_new(&mut h), Ok(()));
        assert_eq!(h.state, WriterEpoch::Active);
        assert_eq!(accept_mutation(&mut h), Ok(1));
    }

    #[test]
    fn ambiguous_two_active_writers_is_conflict() {
        let mut h = Handover::start(7, 8);
        assert_eq!(activate_new(&mut h), Err(QdnfError::Conflict));
        assert_eq!(h.state, WriterEpoch::Ambiguous);
        assert_eq!(activate_new(&mut h), Err(QdnfError::Conflict));
        assert_eq!(admit_write(&h), Err(QdnfError::Conflict));
    }

    #[test]
    fn constructed_ambiguous_is_conflict() {
        let mut h = Handover::ambiguous(1, 2);
        assert_eq!(h.state, WriterEpoch::Ambiguous);
        assert_eq!(activate_new(&mut h), Err(QdnfError::Conflict));
    }

    #[test]
    fn activate_before_transfer_is_incomplete() {
        let mut h = Handover::start(1, 2);
        fence_old(&mut h);
        assert_eq!(activate_new(&mut h), Err(QdnfError::Incomplete));
        drain_accepted(&mut h);
        assert_eq!(activate_new(&mut h), Err(QdnfError::Incomplete));
    }

    #[test]
    fn accepted_cap_is_bounded() {
        let mut h = Handover::start(1, 2);
        let mut i = 0u8;
        while i < ACCEPTED_CAP {
            accept_mutation(&mut h).unwrap();
            i = i + 1;
        }
        assert_eq!(accept_mutation(&mut h), Err(QdnfError::Capacity));
    }

    #[test]
    fn lookup_before_transfer_is_incomplete_not_found() {
        let mut snap = QsrSnapshot::empty(Generation(1));
        snap.insert(StrongDigest::ZERO, StrongDigest::ZERO).unwrap();
        let covers = [CoverInterval { start: 0, end: 15 }];
        let mut out = [StrongDigest::ZERO; 1];
        let mut h = Handover::start(1, 2);
        assert_eq!(
            lookup_via_handover(
                &h,
                &snap,
                &StrongDigest::ZERO,
                &covers,
                Generation(1),
                &mut out
            ),
            Ok(QsrOutcome::Incomplete)
        );
        fence_old(&mut h);
        drain_accepted(&mut h);
        persist_boundary(&mut h);
        assert_eq!(
            lookup_via_handover(
                &h,
                &snap,
                &StrongDigest::ZERO,
                &covers,
                Generation(1),
                &mut out
            ),
            Ok(QsrOutcome::Found { count: 1 })
        );
    }
}
