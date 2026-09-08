//! CRY-02.08 **PARTIAL** — bind negotiation to cached authorized peer/realm minima.
//!
//! Only `qpr-pq-1` ([`PROFILE_QPR_PQ_1`]) is admitted as the initial profile.
//! Stripped/missing offers, classical offers, unknown profiles, revoked cache
//! entries, and any offered profile below the cached minimum fail closed.
//! [`automatic_classical_retry`] is false; there is no classical fallback.
//!
//! Domain labels for the admitted profile live in
//! [`crate::crypto::network::pq_handshake::{qlink_domain, qsession_domain}`].
//! Classical-only authority is never enough
//! ([`crate::crypto::network::pq_handshake::classical_only_authority_is_enough`]).
//!
//! Draft until FND-03 freeze. Alternate proof pairs, cookie admission chunks,
//! and independent vectors remain open. No security proof is claimed.

use crate::crypto::network::errors::CryptoError;

/// Occupied minima slots. An 9th distinct `(peer, realm)` is [`CryptoError::Capacity`].
pub const MAX_CACHED_MINIMA: usize = 8;

/// Native PQ initial profile `qpr-pq-1`. The only admitted selected result.
pub const PROFILE_QPR_PQ_1: u16 = 1;

/// Classical-only profile. Forbidden as a selected result (maps to Downgrade).
pub const PROFILE_CLASSICAL: u16 = 0;

/// Cached authorized peer/realm minimum. `min_profile` must be
/// [`PROFILE_QPR_PQ_1`] for [`select_profile`] to admit.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PeerMinima {
    /// Cached authorized peer id (opaque).
    pub peer: u64,
    /// Realm id.
    pub realm: u64,
    /// Minimum admitted profile. Must be [`PROFILE_QPR_PQ_1`] to admit.
    pub min_profile: u16,
    /// Revoked cache entry. [`select_profile`] returns [`CryptoError::Revoked`].
    pub revoked: bool,
}

/// Fixed 8-slot cache of authorized peer/realm minima. No heap.
pub struct MinimaCache {
    slots: [Option<PeerMinima>; MAX_CACHED_MINIMA],
}

impl MinimaCache {
    pub const fn new() -> Self {
        Self {
            slots: [None; MAX_CACHED_MINIMA],
        }
    }

    /// Insert or replace by `(peer, realm)`. A 9th distinct pair is Capacity.
    pub fn insert(&mut self, m: PeerMinima) -> Result<(), CryptoError> {
        let mut i = 0usize;
        while i < MAX_CACHED_MINIMA {
            if let Some(existing) = &mut self.slots[i] {
                if existing.peer == m.peer && existing.realm == m.realm {
                    *existing = m;
                    return Ok(());
                }
            }
            i += 1;
        }
        let mut i = 0usize;
        while i < MAX_CACHED_MINIMA {
            if self.slots[i].is_none() {
                self.slots[i] = Some(m);
                return Ok(());
            }
            i += 1;
        }
        Err(CryptoError::Capacity)
    }

    /// Lookup by authorized `(peer, realm)`. Missing → Unauthorized.
    pub fn lookup(&self, peer: u64, realm: u64) -> Result<PeerMinima, CryptoError> {
        let mut i = 0usize;
        while i < MAX_CACHED_MINIMA {
            if let Some(m) = self.slots[i] {
                if m.peer == peer && m.realm == realm {
                    return Ok(m);
                }
            }
            i += 1;
        }
        Err(CryptoError::Unauthorized)
    }
}

/// Select a profile given cached minima and an offered profile.
///
/// Stripped / missing offer → [`CryptoError::Malformed`].
/// Offered classical ([`PROFILE_CLASSICAL`]) → [`CryptoError::Downgrade`]
/// (no automatic classical retry).
/// Cached revoked → [`CryptoError::Revoked`].
/// Offered profile &lt; cached `min_profile` → [`CryptoError::Downgrade`].
/// Unknown profile (not 0 or 1) → [`CryptoError::UnknownProfile`].
/// Success only if offered == [`PROFILE_QPR_PQ_1`] and
/// `minima.min_profile` == [`PROFILE_QPR_PQ_1`] and `!revoked`.
pub fn select_profile(cached: &PeerMinima, offered: Option<u16>) -> Result<u16, CryptoError> {
    let offered = match offered {
        Some(profile) => profile,
        None => return Err(CryptoError::Malformed),
    };
    if offered != PROFILE_CLASSICAL && offered != PROFILE_QPR_PQ_1 {
        return Err(CryptoError::UnknownProfile);
    }
    if cached.revoked {
        return Err(CryptoError::Revoked);
    }
    if offered == PROFILE_CLASSICAL {
        return Err(CryptoError::Downgrade);
    }
    if offered < cached.min_profile {
        return Err(CryptoError::Downgrade);
    }
    if offered != PROFILE_QPR_PQ_1 || cached.min_profile != PROFILE_QPR_PQ_1 {
        return Err(CryptoError::Downgrade);
    }
    Ok(PROFILE_QPR_PQ_1)
}

/// Negotiation never retries a classical-only profile automatically.
pub fn automatic_classical_retry() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::network::pq_handshake::classical_only_authority_is_enough;

    fn pq_minima(peer: u64, realm: u64) -> PeerMinima {
        PeerMinima {
            peer,
            realm,
            min_profile: PROFILE_QPR_PQ_1,
            revoked: false,
        }
    }

    #[test]
    fn insert_eight_ok_ninth_capacity() {
        let mut cache = MinimaCache::new();
        let mut i = 0u64;
        while i < MAX_CACHED_MINIMA as u64 {
            assert_eq!(cache.insert(pq_minima(i + 1, 1)), Ok(()));
            i += 1;
        }
        assert_eq!(cache.insert(pq_minima(99, 1)), Err(CryptoError::Capacity));
        assert_eq!(cache.lookup(1, 1).unwrap().peer, 1);
        assert_eq!(
            cache.insert(PeerMinima {
                peer: 1,
                realm: 1,
                min_profile: PROFILE_QPR_PQ_1,
                revoked: true,
            }),
            Ok(())
        );
        assert!(cache.lookup(1, 1).unwrap().revoked);
    }

    #[test]
    fn lookup_missing_is_unauthorized() {
        let cache = MinimaCache::new();
        assert_eq!(cache.lookup(1, 1), Err(CryptoError::Unauthorized));
        let mut cache = MinimaCache::new();
        assert_eq!(cache.insert(pq_minima(2, 3)), Ok(()));
        assert_eq!(cache.lookup(2, 4), Err(CryptoError::Unauthorized));
        assert_eq!(cache.lookup(1, 3), Err(CryptoError::Unauthorized));
    }

    #[test]
    fn select_profile_none_is_malformed() {
        let cached = pq_minima(1, 1);
        assert_eq!(select_profile(&cached, None), Err(CryptoError::Malformed));
    }

    #[test]
    fn select_profile_classical_is_downgrade() {
        let cached = pq_minima(1, 1);
        assert_eq!(
            select_profile(&cached, Some(PROFILE_CLASSICAL)),
            Err(CryptoError::Downgrade)
        );
    }

    #[test]
    fn select_profile_unknown_is_unknown_profile() {
        let cached = pq_minima(1, 1);
        assert_eq!(
            select_profile(&cached, Some(99)),
            Err(CryptoError::UnknownProfile)
        );
    }

    #[test]
    fn select_profile_revoked_even_if_offered_pq() {
        let cached = PeerMinima {
            peer: 7,
            realm: 8,
            min_profile: PROFILE_QPR_PQ_1,
            revoked: true,
        };
        assert_eq!(
            select_profile(&cached, Some(PROFILE_QPR_PQ_1)),
            Err(CryptoError::Revoked)
        );
    }

    #[test]
    fn no_automatic_classical_retry_and_classical_authority_never_enough() {
        assert!(!automatic_classical_retry());
        assert!(!classical_only_authority_is_enough());
    }

    #[test]
    fn select_profile_pq_against_min_pq_is_ok() {
        let cached = pq_minima(4, 5);
        assert_eq!(
            select_profile(&cached, Some(PROFILE_QPR_PQ_1)),
            Ok(PROFILE_QPR_PQ_1)
        );
    }
}
