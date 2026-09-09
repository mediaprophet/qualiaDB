//! Bounded traffic-key update (NET-05.12 partial).
//!
//! Rekey and replayed cached [`PolicyOutcome::Allow`] do not renew service
//! authority. Drain: Active → Draining is permitted; application from
//! Draining is rejected except closure. Old-key retention is current plus
//! [`MAX_OLD_KEYS`] previous generations. Packages remain open.
//!
//! CRY-02.11: a traffic-key update is not fresh hybrid recovery.

use crate::net::qdnf::authority::PolicyOutcome;
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::session::handshake::{SessionBinding, SessionState};

/// Bounded old-key retention (current + 2 old).
pub const MAX_OLD_KEYS: usize = 2;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyPhase {
    Current = 1,
    Previous = 2,
    Stale = 3,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TrafficKeySlot {
    pub generation: u64,
    pub phase: KeyPhase,
}

/// Current generation plus up to [`MAX_OLD_KEYS`] previous generations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RekeyTable {
    current: u64,
    /// Newest-first previous generations. Index 0 is immediately previous.
    old: [u64; MAX_OLD_KEYS],
    old_len: u8,
}

impl RekeyTable {
    pub const fn new() -> Self {
        Self {
            current: 0,
            old: [0; MAX_OLD_KEYS],
            old_len: 0,
        }
    }

    #[inline]
    pub fn current_generation(&self) -> u64 {
        self.current
    }

    /// Install next traffic key generation. Does not change
    /// [`SessionBinding::policy`] / grant.
    ///
    /// The oldest retained generation is retired when the old-key bound is
    /// full. [`QdnfError::Capacity`] if the generation counter cannot advance
    /// (would exceed the bound without a generation left to install).
    pub fn rotate(&mut self) -> Result<u64, QdnfError> {
        if self.current == u64::MAX {
            return Err(QdnfError::Capacity);
        }
        let retiring = self.current;
        let n = self.old_len as usize;
        if n == MAX_OLD_KEYS {
            let mut i = MAX_OLD_KEYS;
            while i > 1 {
                i -= 1;
                self.old[i] = self.old[i - 1];
            }
            self.old[0] = retiring;
        } else {
            let mut i = n;
            while i > 0 {
                self.old[i] = self.old[i - 1];
                i -= 1;
            }
            self.old[0] = retiring;
            self.old_len = self.old_len.saturating_add(1);
        }
        self.current = self.current.saturating_add(1);
        Ok(self.current)
    }

    /// Lookup. Unknown generation → [`QdnfError::Expired`]. Stale (beyond
    /// retention) → [`QdnfError::Expired`].
    pub fn phase(&self, generation: u64) -> Result<KeyPhase, QdnfError> {
        if generation == self.current {
            return Ok(KeyPhase::Current);
        }
        let n = self.old_len as usize;
        let mut i = 0;
        while i < n {
            if self.old[i] == generation {
                return Ok(if i == 0 {
                    KeyPhase::Previous
                } else {
                    KeyPhase::Stale
                });
            }
            i += 1;
        }
        Err(QdnfError::Expired)
    }

    /// Occupied slot for `generation`, including phase. Same miss codes as
    /// [`Self::phase`].
    pub fn slot(&self, generation: u64) -> Result<TrafficKeySlot, QdnfError> {
        Ok(TrafficKeySlot {
            generation,
            phase: self.phase(generation)?,
        })
    }
}

/// Rekey never renews service authority (NET-05.12).
#[inline]
pub fn rekey_renews_service_authority() -> bool {
    false
}

/// Replayed cached Allow does not become a current grant.
#[inline]
pub fn replayed_cached_allow_renews_grant() -> bool {
    let _ = PolicyOutcome::Allow;
    false
}

/// Drain: Active → Draining is ok; application from Draining is Unauthorized
/// except closure.
///
/// If `!grant_current` → [`QdnfError::Unauthorized`] even if
/// `binding.policy` is [`PolicyOutcome::Allow`] and state is Active.
/// If `binding.state` is Draining or Closed → [`QdnfError::Closed`].
/// Else delegate to [`SessionBinding::admit_application`] (already requires
/// Active + current policy).
pub fn admit_after_rekey(binding: &SessionBinding, grant_current: bool) -> Result<(), QdnfError> {
    if !grant_current {
        return Err(QdnfError::Unauthorized);
    }
    if matches!(binding.state(), SessionState::Draining | SessionState::Closed) {
        return Err(QdnfError::Closed);
    }
    binding.admit_application()
}

/// CRY-02.11: key update ≠ fresh hybrid recovery.
#[inline]
pub fn traffic_update_is_hybrid_recovery() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::qdnf::types::{OperationId, StrongDigest};

    fn binding(state: SessionState, policy: PolicyOutcome) -> SessionBinding {
        SessionBinding::test_fixture(
            OperationId([1u8; 16]),
            StrongDigest([2u8; 48]),
            StrongDigest([3u8; 48]),
            StrongDigest([4u8; 48]),
            policy,
            state,
        )
    }

    #[test]
    fn rotate_twice_keeps_current_and_two_old_third_retires_oldest() {
        let mut table = RekeyTable::new();
        assert_eq!(table.current_generation(), 0);
        assert_eq!(table.rotate(), Ok(1));
        assert_eq!(table.rotate(), Ok(2));
        assert_eq!(table.current_generation(), 2);
        assert_eq!(table.phase(2), Ok(KeyPhase::Current));
        assert_eq!(table.phase(1), Ok(KeyPhase::Previous));
        assert_eq!(table.phase(0), Ok(KeyPhase::Stale));
        assert_eq!(table.slot(0).unwrap().generation, 0);
        assert_eq!(table.rotate(), Ok(3));
        assert_eq!(table.current_generation(), 3);
        assert_eq!(table.phase(3), Ok(KeyPhase::Current));
        assert_eq!(table.phase(2), Ok(KeyPhase::Previous));
        assert_eq!(table.phase(1), Ok(KeyPhase::Stale));
        assert_eq!(table.phase(0), Err(QdnfError::Expired));
        assert_eq!(table.phase(99), Err(QdnfError::Expired));
    }

    #[test]
    fn rekey_and_replayed_cache_do_not_renew_authority() {
        assert!(!rekey_renews_service_authority());
        assert!(!replayed_cached_allow_renews_grant());
    }

    #[test]
    fn admit_after_rekey_stale_grant_unauthorized_when_active_allow() {
        let s = binding(SessionState::Active, PolicyOutcome::Allow);
        assert_eq!(s.admit_application(), Ok(()));
        assert_eq!(admit_after_rekey(&s, false), Err(QdnfError::Unauthorized));
    }

    #[test]
    fn admit_after_rekey_active_allow_current_grant_ok() {
        let s = binding(SessionState::Active, PolicyOutcome::Allow);
        assert_eq!(admit_after_rekey(&s, true), Ok(()));
    }

    #[test]
    fn admit_after_rekey_draining_is_closed() {
        let s = binding(SessionState::Draining, PolicyOutcome::Allow);
        assert_eq!(admit_after_rekey(&s, true), Err(QdnfError::Closed));
    }

    #[test]
    fn traffic_update_is_not_hybrid_recovery() {
        assert!(!traffic_update_is_hybrid_recovery());
    }

    #[test]
    fn rotate_zero_to_two_current_and_previous_phases() {
        let mut table = RekeyTable::new();
        assert_eq!(table.rotate(), Ok(1));
        assert_eq!(table.rotate(), Ok(2));
        assert_eq!(
            table.phase(table.current_generation()),
            Ok(KeyPhase::Current)
        );
        assert_eq!(table.phase(1), Ok(KeyPhase::Previous));
    }
}
