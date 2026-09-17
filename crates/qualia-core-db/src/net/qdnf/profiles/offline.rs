//! Prepared offline contact packages and short-lived capabilities (E16.4).
//!
//! Queue capacity is 8. Release checks [`Generation`] equality rather than a
//! caller-asserted “authority is current” boolean. Expired packages stay
//! sealed; clock rollback does not extend expiry.
//!
//! [`ScopedCapability`] cannot be refreshed by copying: a copy keeps the
//! original deadline and [`activate_capability`] still expires after it.
//!
//! Offline permissions are never labelled “current” after freshness lapses.
//! Remote wipe and attestation cannot guarantee safety after endpoint
//! compromise or seizure (E16.7).

use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::{Generation, StrongDigest};

pub const MAX_OFFLINE: usize = 8;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OfflinePackage {
    pub recipient: StrongDigest,
    pub expiry_unix: u64,
    pub ciphertext_len: u32,
    pub digest: StrongDigest,
}

impl OfflinePackage {
    pub const EMPTY: Self = Self {
        recipient: StrongDigest::ZERO,
        expiry_unix: 0,
        ciphertext_len: 0,
        digest: StrongDigest::ZERO,
    };
}

/// Short-lived capability. Copying does not extend [`Self::expires_unix`].
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScopedCapability {
    pub expires_unix: u64,
    pub purpose: StrongDigest,
    pub recipient: StrongDigest,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct OfflineQueue {
    packages: [OfflinePackage; MAX_OFFLINE],
    occupied: [bool; MAX_OFFLINE],
    last_seen_unix: u64,
}

impl OfflineQueue {
    pub const fn new() -> Self {
        Self {
            packages: [OfflinePackage::EMPTY; MAX_OFFLINE],
            occupied: [false; MAX_OFFLINE],
            last_seen_unix: 0,
        }
    }

    #[inline]
    pub const fn last_seen_unix(&self) -> u64 {
        self.last_seen_unix
    }

    pub fn get(&self, slot: u8) -> Result<OfflinePackage, QdnfError> {
        let i = slot as usize;
        if i >= MAX_OFFLINE || !self.occupied[i] {
            return Err(QdnfError::Denied);
        }
        Ok(self.packages[i])
    }
}

impl Default for OfflineQueue {
    fn default() -> Self {
        Self::new()
    }
}

fn package_well_formed(pkg: &OfflinePackage) -> Result<(), QdnfError> {
    if pkg.recipient.is_zero() {
        return Err(QdnfError::Malformed);
    }
    if pkg.ciphertext_len == 0 && !pkg.digest.is_zero() {
        return Err(QdnfError::Malformed);
    }
    Ok(())
}

/// Enqueue a sealed package. Capacity 8. Clock rollback is Conflict and
/// does not rewrite stored expiry.
pub fn queue_offline(
    queue: &mut OfflineQueue,
    pkg: OfflinePackage,
    now: u64,
) -> Result<u8, QdnfError> {
    if now < queue.last_seen_unix {
        return Err(QdnfError::Conflict);
    }
    package_well_formed(&pkg)?;
    if pkg.ciphertext_len == 0 {
        return Err(QdnfError::Malformed);
    }
    let mut i = 0usize;
    while i < MAX_OFFLINE {
        if !queue.occupied[i] {
            queue.packages[i] = pkg;
            queue.occupied[i] = true;
            queue.last_seen_unix = now;
            return Ok(i as u8);
        }
        i += 1;
    }
    Err(QdnfError::Capacity)
}

/// Release a queued package. Authority freshness is the live generation,
/// not a boolean assertion from the caller.
pub fn release_offline(
    pkg: &OfflinePackage,
    now: u64,
    authority_generation: Generation,
    live_generation: Generation,
) -> Result<(), QdnfError> {
    package_well_formed(pkg)?;
    if pkg.ciphertext_len == 0 {
        return Err(QdnfError::Malformed);
    }
    if now >= pkg.expiry_unix {
        return Err(QdnfError::Expired);
    }
    if authority_generation != live_generation {
        return Err(QdnfError::StaleGeneration);
    }
    Ok(())
}

/// Release from the queue, applying monotonic clock checks.
pub fn release_queued(
    queue: &mut OfflineQueue,
    slot: u8,
    now: u64,
    authority_generation: Generation,
    live_generation: Generation,
) -> Result<(), QdnfError> {
    if now < queue.last_seen_unix {
        return Err(QdnfError::Conflict);
    }
    let pkg = queue.get(slot)?;
    release_offline(&pkg, now, authority_generation, live_generation)?;
    queue.last_seen_unix = now;
    queue.occupied[slot as usize] = false;
    queue.packages[slot as usize] = OfflinePackage::EMPTY;
    Ok(())
}

pub fn activate_capability(cap: &ScopedCapability, now: u64) -> Result<(), QdnfError> {
    if cap.recipient.is_zero() || cap.purpose.is_zero() {
        return Err(QdnfError::Malformed);
    }
    if now >= cap.expires_unix {
        Err(QdnfError::Expired)
    } else {
        Ok(())
    }
}

/// Copying is not a refresh: the deadline is unchanged and this call is denied.
pub fn extend_capability_by_copy(
    cap: &ScopedCapability,
    _new_expiry: u64,
) -> Result<ScopedCapability, QdnfError> {
    let _ = cap;
    Err(QdnfError::Denied)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(b: u8) -> StrongDigest {
        let mut d = StrongDigest::ZERO;
        d.0[0] = b;
        d.0[47] = 3;
        d
    }

    fn pkg(exp: u64) -> OfflinePackage {
        OfflinePackage {
            recipient: digest(1),
            expiry_unix: exp,
            ciphertext_len: 64,
            digest: digest(2),
        }
    }

    #[test]
    fn queue_and_release_matching_generation() {
        let mut q = OfflineQueue::new();
        let slot = queue_offline(&mut q, pkg(100), 10).unwrap();
        release_queued(&mut q, slot, 11, Generation(3), Generation(3)).unwrap();
        assert_eq!(q.get(slot).unwrap_err(), QdnfError::Denied);
    }

    #[test]
    fn stale_generation_is_not_a_boolean_ok() {
        let p = pkg(100);
        assert_eq!(
            release_offline(&p, 10, Generation(1), Generation(2)),
            Err(QdnfError::StaleGeneration)
        );
    }

    #[test]
    fn expired_stays_sealed() {
        let p = pkg(10);
        assert_eq!(
            release_offline(&p, 10, Generation(1), Generation(1)),
            Err(QdnfError::Expired)
        );
    }

    #[test]
    fn empty_ciphertext_with_digest_is_malformed() {
        let p = OfflinePackage {
            recipient: digest(1),
            expiry_unix: 100,
            ciphertext_len: 0,
            digest: digest(9),
        };
        let mut q = OfflineQueue::new();
        assert_eq!(
            queue_offline(&mut q, p, 1).unwrap_err(),
            QdnfError::Malformed
        );
        assert_eq!(
            release_offline(&p, 1, Generation(1), Generation(1)),
            Err(QdnfError::Malformed)
        );
    }

    #[test]
    fn clock_rollback_does_not_extend_expiry() {
        let mut q = OfflineQueue::new();
        let slot = queue_offline(&mut q, pkg(100), 50).unwrap();
        let exp = q.get(slot).unwrap().expiry_unix;
        assert_eq!(
            queue_offline(&mut q, pkg(500), 40).unwrap_err(),
            QdnfError::Conflict
        );
        assert_eq!(q.get(slot).unwrap().expiry_unix, exp);
        assert_eq!(
            release_queued(&mut q, slot, 40, Generation(1), Generation(1)).unwrap_err(),
            QdnfError::Conflict
        );
        assert_eq!(q.get(slot).unwrap().expiry_unix, exp);
    }

    #[test]
    fn capability_copy_does_not_refresh() {
        let cap = ScopedCapability {
            expires_unix: 20,
            purpose: digest(4),
            recipient: digest(5),
        };
        activate_capability(&cap, 10).unwrap();
        let copied = cap;
        assert_eq!(copied.expires_unix, cap.expires_unix);
        assert_eq!(activate_capability(&copied, 20), Err(QdnfError::Expired));
        assert_eq!(
            extend_capability_by_copy(&copied, 9999).unwrap_err(),
            QdnfError::Denied
        );
        assert_eq!(copied.expires_unix, 20);
    }

    #[test]
    fn queue_capacity_eight() {
        let mut q = OfflineQueue::new();
        let mut i = 0u8;
        while i < MAX_OFFLINE as u8 {
            queue_offline(&mut q, pkg(100), 1).unwrap();
            i += 1;
        }
        assert_eq!(
            queue_offline(&mut q, pkg(100), 2).unwrap_err(),
            QdnfError::Capacity
        );
    }
}
