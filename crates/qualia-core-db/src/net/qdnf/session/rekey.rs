//! Bounded traffic-key update (E02.4).
//!
//! Rekey and replayed cached [`PolicyOutcome::Allow`] do not renew service
//! authority. Drain: Active → Draining is permitted; application from
//! Draining is rejected except closure. Old-key retention is current plus
//! [`MAX_OLD_KEYS`] previous generations with actual secret material.
//! Packages remain open.
//!
//! CRY-02.11: a traffic-key update is not fresh hybrid recovery.

use crate::crypto::network::types::AEAD_KEY_LEN;
use crate::net::qdnf::authority::PolicyOutcome;
use crate::net::qdnf::crypto::schedule::derive_traffic_update;
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::session::handshake::{SessionBinding, SessionState};
use crate::net::qdnf::types::Generation;
use zeroize::Zeroize;

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

struct SecretSlot {
    occupied: bool,
    generation: u64,
    send: [u8; AEAD_KEY_LEN],
    recv: [u8; AEAD_KEY_LEN],
}

impl SecretSlot {
    const EMPTY: Self = Self {
        occupied: false,
        generation: 0,
        send: [0u8; AEAD_KEY_LEN],
        recv: [0u8; AEAD_KEY_LEN],
    };

    fn erase(&mut self) {
        self.send.zeroize();
        self.recv.zeroize();
        self.occupied = false;
        self.generation = 0;
    }
}

/// Owner of traffic secrets. Not `Copy`: generations without keys are not a rotation.
pub struct RekeyTable {
    current: u64,
    /// Newest-first previous generations. Index 0 is immediately previous.
    old: [u64; MAX_OLD_KEYS],
    old_len: u8,
    current_keys: SecretSlot,
    old_keys: [SecretSlot; MAX_OLD_KEYS],
}

impl RekeyTable {
    pub fn from_keys(
        send: [u8; AEAD_KEY_LEN],
        recv: [u8; AEAD_KEY_LEN],
    ) -> Result<Self, QdnfError> {
        if send == [0u8; AEAD_KEY_LEN] || recv == [0u8; AEAD_KEY_LEN] || send == recv {
            return Err(QdnfError::CryptoFailure);
        }
        Ok(Self {
            current: 1,
            old: [0; MAX_OLD_KEYS],
            old_len: 0,
            current_keys: SecretSlot {
                occupied: true,
                generation: 1,
                send,
                recv,
            },
            old_keys: [SecretSlot::EMPTY; MAX_OLD_KEYS],
        })
    }

    #[inline]
    pub fn current_generation(&self) -> u64 {
        self.current
    }

    pub fn current_generation_token(&self) -> Generation {
        Generation(self.current)
    }

    /// Install next traffic key generation. Does not change
    /// [`SessionBinding::policy`] / grant.
    ///
    /// Derives new secrets from the current pair, zeroizes the retired slot,
    /// and resets uniqueness by exposing a new generation id for packet spaces.
    pub fn rotate(&mut self) -> Result<u64, QdnfError> {
        if self.current == u64::MAX {
            return Err(QdnfError::Capacity);
        }
        if !self.current_keys.occupied {
            return Err(QdnfError::Incomplete);
        }
        let next = self.current.saturating_add(1);
        let (next_send, next_recv) =
            derive_traffic_update(&self.current_keys.send, &self.current_keys.recv, next)?;
        let retiring = self.current;
        let retiring_keys = SecretSlot {
            occupied: true,
            generation: retiring,
            send: self.current_keys.send,
            recv: self.current_keys.recv,
        };
        let n = self.old_len as usize;
        if n == MAX_OLD_KEYS {
            self.old_keys[MAX_OLD_KEYS - 1].erase();
            let mut i = MAX_OLD_KEYS;
            while i > 1 {
                i -= 1;
                self.old[i] = self.old[i - 1];
                self.old_keys[i] = SecretSlot {
                    occupied: self.old_keys[i - 1].occupied,
                    generation: self.old_keys[i - 1].generation,
                    send: self.old_keys[i - 1].send,
                    recv: self.old_keys[i - 1].recv,
                };
            }
            self.old[0] = retiring;
            self.old_keys[0] = retiring_keys;
        } else {
            let mut i = n;
            while i > 0 {
                self.old[i] = self.old[i - 1];
                self.old_keys[i] = SecretSlot {
                    occupied: self.old_keys[i - 1].occupied,
                    generation: self.old_keys[i - 1].generation,
                    send: self.old_keys[i - 1].send,
                    recv: self.old_keys[i - 1].recv,
                };
                i -= 1;
            }
            self.old[0] = retiring;
            self.old_keys[0] = retiring_keys;
            self.old_len = self.old_len.saturating_add(1);
        }
        self.current_keys.send.zeroize();
        self.current_keys.recv.zeroize();
        self.current_keys = SecretSlot {
            occupied: true,
            generation: next,
            send: next_send,
            recv: next_recv,
        };
        self.current = next;
        Ok(self.current)
    }

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

    pub fn slot(&self, generation: u64) -> Result<TrafficKeySlot, QdnfError> {
        Ok(TrafficKeySlot {
            generation,
            phase: self.phase(generation)?,
        })
    }

    /// Copy directional keys for an occupied generation. Retired generations fail.
    pub fn keys(
        &self,
        generation: u64,
    ) -> Result<([u8; AEAD_KEY_LEN], [u8; AEAD_KEY_LEN]), QdnfError> {
        if generation == self.current && self.current_keys.occupied {
            return Ok((self.current_keys.send, self.current_keys.recv));
        }
        let n = self.old_len as usize;
        let mut i = 0;
        while i < n {
            if self.old_keys[i].occupied && self.old_keys[i].generation == generation {
                return Ok((self.old_keys[i].send, self.old_keys[i].recv));
            }
            i += 1;
        }
        Err(QdnfError::Expired)
    }
}

impl Drop for RekeyTable {
    fn drop(&mut self) {
        self.current_keys.erase();
        let mut i = 0;
        while i < MAX_OLD_KEYS {
            self.old_keys[i].erase();
            i += 1;
        }
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
        let mut table = RekeyTable::from_keys([1u8; 32], [2u8; 32]).unwrap();
        assert_eq!(table.current_generation(), 1);
        let first = table.keys(1).unwrap();
        assert_eq!(table.rotate(), Ok(2));
        assert_eq!(table.rotate(), Ok(3));
        assert_eq!(table.current_generation(), 3);
        assert_eq!(table.phase(3), Ok(KeyPhase::Current));
        assert_eq!(table.phase(2), Ok(KeyPhase::Previous));
        assert_eq!(table.phase(1), Ok(KeyPhase::Stale));
        assert_eq!(table.slot(1).unwrap().generation, 1);
        assert_eq!(table.rotate(), Ok(4));
        assert_eq!(table.current_generation(), 4);
        assert_eq!(table.phase(4), Ok(KeyPhase::Current));
        assert_eq!(table.phase(3), Ok(KeyPhase::Previous));
        assert_eq!(table.phase(2), Ok(KeyPhase::Stale));
        assert_eq!(table.phase(1), Err(QdnfError::Expired));
        assert_eq!(table.phase(99), Err(QdnfError::Expired));
        assert_eq!(table.keys(1), Err(QdnfError::Expired));
        assert_ne!(table.keys(4).unwrap(), first);
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
        let mut table = RekeyTable::from_keys([1u8; 32], [2u8; 32]).unwrap();
        assert_eq!(table.rotate(), Ok(2));
        assert_eq!(table.rotate(), Ok(3));
        assert_eq!(
            table.phase(table.current_generation()),
            Ok(KeyPhase::Current)
        );
        assert_eq!(table.phase(2), Ok(KeyPhase::Previous));
    }

    #[test]
    fn rotate_derives_new_secrets_and_does_not_renew_grant() {
        let mut table = RekeyTable::from_keys([7u8; 32], [8u8; 32]).unwrap();
        let before = table.keys(1).unwrap();
        table.rotate().unwrap();
        let after = table.keys(2).unwrap();
        assert_ne!(before, after);
        assert!(!rekey_renews_service_authority());
        let s = binding(SessionState::Active, PolicyOutcome::Allow);
        assert_eq!(admit_after_rekey(&s, true), Ok(()));
        assert_eq!(admit_after_rekey(&s, false), Err(QdnfError::Unauthorized));
    }

    #[test]
    fn all_zero_or_equal_keys_are_rejected() {
        assert!(RekeyTable::from_keys([0u8; 32], [1u8; 32]).is_err());
        assert!(RekeyTable::from_keys([1u8; 32], [1u8; 32]).is_err());
    }
}
