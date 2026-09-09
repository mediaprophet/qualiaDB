//! Protected local vaults (E16.2).
//!
//! Keys are stored as [`KeyRef`] (slot + generation), never as exported raw
//! key bytes on this path. A digest in a slot is a handle, not a secret dump.
//!
//! # What this module cannot enforce (E16.7)
//!
//! Remote wipe and attestation cannot guarantee safety after endpoint
//! compromise or seizure. An unlocked or coerced device can disclose
//! accessible plaintext. Crash dumps must omit key material
//! ([`crash_dump_redacted`] is true when `crash_dump_allowed` is false).
//! Hardware-backed references are reported only when a real token exists;
//! this Linux CI path does not fake a TPM.

use crate::crypto::network::aead::{decrypt_in_place, encrypt_in_place};
use crate::crypto::network::types::{AEAD_KEY_LEN, AEAD_NONCE_LEN, AEAD_TAG_LEN};
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::{Generation, StrongDigest};

use super::catalog::{catalog_entry, ProtectionProfile};

/// AEAD AAD for sealed catalog-profile blobs.
const SEALED_PROFILE_AAD: &[u8] = b"qdnf-profile-vault-v1";
/// `profile_u8` + `control_count` + catalog digest.
pub const SEALED_PROFILE_PLAIN_LEN: usize = 1 + 1 + 48;

/// Maximum notification preview size. Not a full payload.
pub const NOTIFICATION_PREVIEW_MAX_BYTES: usize = 32;
pub const VAULT_SLOTS: usize = 8;

/// Slot + generation handle. Not raw key bytes.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyRef {
    pub slot: u8,
    pub generation: Generation,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VaultSlot {
    pub occupied: bool,
    pub expiry_unix: u64,
    pub locked: bool,
    pub crash_dump_allowed: bool,
}

impl VaultSlot {
    pub const EMPTY: Self = Self {
        occupied: false,
        expiry_unix: 0,
        locked: false,
        crash_dump_allowed: false,
    };
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct LocalVault {
    slots: [VaultSlot; VAULT_SLOTS],
    refs: [StrongDigest; VAULT_SLOTS],
    generations: [Generation; VAULT_SLOTS],
    last_seen_unix: u64,
    locked: bool,
}

impl LocalVault {
    pub const fn new() -> Self {
        Self {
            slots: [VaultSlot::EMPTY; VAULT_SLOTS],
            refs: [StrongDigest::ZERO; VAULT_SLOTS],
            generations: [Generation::ZERO; VAULT_SLOTS],
            last_seen_unix: 0,
            locked: false,
        }
    }

    #[inline]
    pub const fn is_locked(&self) -> bool {
        self.locked
    }

    #[inline]
    pub fn slot(&self, index: u8) -> Result<&VaultSlot, QdnfError> {
        let i = index as usize;
        if i >= VAULT_SLOTS {
            return Err(QdnfError::Range);
        }
        Ok(&self.slots[i])
    }

    #[inline]
    pub fn issued_ref(&self, slot: u8) -> Result<KeyRef, QdnfError> {
        let i = slot as usize;
        if i >= VAULT_SLOTS || !self.slots[i].occupied {
            return Err(QdnfError::Denied);
        }
        Ok(KeyRef {
            slot,
            generation: self.generations[i],
        })
    }
}

impl Default for LocalVault {
    fn default() -> Self {
        Self::new()
    }
}

fn note_time(vault: &LocalVault, now: u64) -> Result<(), QdnfError> {
    if now < vault.last_seen_unix {
        // Clock rollback: do not extend expiry; fail closed.
        return Err(QdnfError::Conflict);
    }
    Ok(())
}

/// Store a key *reference* digest. Raw secret bytes are not accepted here.
pub fn store_ref(
    vault: &mut LocalVault,
    digest: StrongDigest,
    now: u64,
    expiry: u64,
) -> Result<u8, QdnfError> {
    if vault.locked {
        return Err(QdnfError::Denied);
    }
    note_time(vault, now)?;
    if digest.is_zero() {
        return Err(QdnfError::Malformed);
    }
    if expiry <= now {
        return Err(QdnfError::Expired);
    }
    let mut free = None;
    let mut i = 0usize;
    while i < VAULT_SLOTS {
        if !vault.slots[i].occupied {
            free = Some(i);
            break;
        }
        i += 1;
    }
    let idx = free.ok_or(QdnfError::Capacity)?;
    let next_gen = vault.generations[idx].next()?;
    vault.slots[idx] = VaultSlot {
        occupied: true,
        expiry_unix: expiry,
        locked: false,
        crash_dump_allowed: false,
    };
    vault.refs[idx] = digest;
    vault.generations[idx] = next_gen;
    vault.last_seen_unix = now;
    Ok(idx as u8)
}

/// Load the stored digest. Denied if the vault/slot is locked; Expired if
/// `now` is at or past the slot deadline. Generation mismatch is stale.
pub fn load_ref(vault: &mut LocalVault, slot: u8, now: u64) -> Result<StrongDigest, QdnfError> {
    load_key_ref(
        vault,
        KeyRef {
            slot,
            generation: {
                let i = slot as usize;
                if i >= VAULT_SLOTS {
                    return Err(QdnfError::Range);
                }
                vault.generations[i]
            },
        },
        now,
    )
}

pub fn load_key_ref(
    vault: &mut LocalVault,
    key_ref: KeyRef,
    now: u64,
) -> Result<StrongDigest, QdnfError> {
    note_time(vault, now)?;
    let i = key_ref.slot as usize;
    if i >= VAULT_SLOTS {
        return Err(QdnfError::Range);
    }
    let slot = vault.slots[i];
    if vault.locked || slot.locked {
        return Err(QdnfError::Denied);
    }
    if !slot.occupied {
        return Err(QdnfError::Denied);
    }
    if vault.generations[i] != key_ref.generation {
        return Err(QdnfError::StaleGeneration);
    }
    if now >= slot.expiry_unix {
        return Err(QdnfError::Expired);
    }
    vault.last_seen_unix = now;
    Ok(vault.refs[i])
}

/// Irreversible local lock. There is no unlock / backdoor API.
pub fn lock(vault: &mut LocalVault) {
    vault.locked = true;
    let mut i = 0usize;
    while i < VAULT_SLOTS {
        vault.slots[i].locked = true;
        i += 1;
    }
}

/// True when dumps must not include key material (`crash_dump_allowed` is false).
pub fn crash_dump_redacted(vault: &LocalVault) -> bool {
    let mut i = 0usize;
    while i < VAULT_SLOTS {
        if vault.slots[i].occupied && vault.slots[i].crash_dump_allowed {
            return false;
        }
        i += 1;
    }
    true
}

pub fn notification_preview_max_bytes() -> usize {
    NOTIFICATION_PREVIEW_MAX_BYTES
}

/// Hardware-backed [`KeyRef`] support. Presence of `/dev/tpm0` is not enough:
/// this crate has no TPM token binding and must not fake one.
pub fn hardware_key_ref_supported() -> bool {
    false
}

/// P4-with-hardware-token requirement. Unsupported hardware is unavailable,
/// never a silent software downgrade.
pub fn require_hardware_backed_p4() -> Result<(), QdnfError> {
    if hardware_key_ref_supported() {
        Ok(())
    } else {
        Err(QdnfError::UnknownProfile)
    }
}

/// Catalog profile sealed under a wrap key. Ciphertext is not a key dump.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SealedProfileBlob {
    pub ct: [u8; SEALED_PROFILE_PLAIN_LEN],
    pub tag: [u8; AEAD_TAG_LEN],
    pub nonce: [u8; AEAD_NONCE_LEN],
}

fn encode_profile_plain(profile: ProtectionProfile) -> [u8; SEALED_PROFILE_PLAIN_LEN] {
    let set = catalog_entry(profile);
    let mut plain = [0u8; SEALED_PROFILE_PLAIN_LEN];
    plain[0] = profile.to_u8();
    plain[1] = set.control_count;
    plain[2..].copy_from_slice(set.digest.as_bytes());
    plain
}

/// Seal a catalog profile. Locked vaults fail closed. Zero keys are Malformed.
pub fn seal_profile_blob(
    vault: &LocalVault,
    profile: ProtectionProfile,
    key: &[u8; AEAD_KEY_LEN],
    nonce: &[u8; AEAD_NONCE_LEN],
) -> Result<SealedProfileBlob, QdnfError> {
    if vault.locked {
        return Err(QdnfError::Denied);
    }
    if key == &[0u8; AEAD_KEY_LEN] {
        return Err(QdnfError::Malformed);
    }
    let mut ct = encode_profile_plain(profile);
    let mut tag = [0u8; AEAD_TAG_LEN];
    encrypt_in_place(key, nonce, SEALED_PROFILE_AAD, &mut ct, &mut tag)?;
    Ok(SealedProfileBlob {
        ct,
        tag,
        nonce: *nonce,
    })
}

/// Open a sealed catalog profile. Wrong key is CryptoFailure (fail closed).
pub fn open_profile_blob(
    vault: &LocalVault,
    blob: &SealedProfileBlob,
    key: &[u8; AEAD_KEY_LEN],
) -> Result<ProtectionProfile, QdnfError> {
    if vault.locked {
        return Err(QdnfError::Denied);
    }
    let mut plain = blob.ct;
    match decrypt_in_place(key, &blob.nonce, SEALED_PROFILE_AAD, &mut plain, &blob.tag) {
        Ok(()) => {}
        Err(e) => {
            let mut i = 0usize;
            while i < plain.len() {
                plain[i] = 0;
                i += 1;
            }
            return Err(e);
        }
    }
    let profile = ProtectionProfile::from_u8(plain[0])?;
    let expected = encode_profile_plain(profile);
    let mut i = 0usize;
    while i < SEALED_PROFILE_PLAIN_LEN {
        if plain[i] != expected[i] {
            return Err(QdnfError::Malformed);
        }
        i += 1;
    }
    Ok(profile)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(b: u8) -> StrongDigest {
        let mut d = StrongDigest::ZERO;
        d.0[0] = b;
        d.0[47] = 7;
        d
    }

    #[test]
    fn store_and_load_ref_round_trip() {
        let mut v = LocalVault::new();
        let slot = store_ref(&mut v, digest(1), 10, 100).unwrap();
        assert_eq!(load_ref(&mut v, slot, 11).unwrap(), digest(1));
        let kr = v.issued_ref(slot).unwrap();
        assert_eq!(kr.slot, slot);
        assert_ne!(kr.generation, Generation::ZERO);
    }

    #[test]
    fn load_expired_is_expired() {
        let mut v = LocalVault::new();
        let slot = store_ref(&mut v, digest(2), 10, 20).unwrap();
        assert_eq!(load_ref(&mut v, slot, 20).unwrap_err(), QdnfError::Expired);
    }

    #[test]
    fn lock_denies_load_and_store() {
        let mut v = LocalVault::new();
        let slot = store_ref(&mut v, digest(3), 1, 50).unwrap();
        lock(&mut v);
        assert_eq!(load_ref(&mut v, slot, 2).unwrap_err(), QdnfError::Denied);
        assert_eq!(
            store_ref(&mut v, digest(4), 3, 50).unwrap_err(),
            QdnfError::Denied
        );
        assert!(v.is_locked());
    }

    #[test]
    fn crash_dump_redacted_when_not_allowed() {
        let mut v = LocalVault::new();
        let _ = store_ref(&mut v, digest(5), 1, 50).unwrap();
        assert!(!v.slot(0).unwrap().crash_dump_allowed);
        assert!(crash_dump_redacted(&v));
    }

    #[test]
    fn notification_preview_is_small() {
        assert_eq!(notification_preview_max_bytes(), 32);
        assert!(notification_preview_max_bytes() < 256);
    }

    #[test]
    fn hardware_key_ref_not_faked() {
        assert!(!hardware_key_ref_supported());
        assert_eq!(require_hardware_backed_p4(), Err(QdnfError::UnknownProfile));
    }

    #[test]
    fn clock_rollback_does_not_extend_expiry() {
        let mut v = LocalVault::new();
        let slot = store_ref(&mut v, digest(6), 50, 200).unwrap();
        let expiry = v.slot(slot).unwrap().expiry_unix;
        assert_eq!(load_ref(&mut v, slot, 40).unwrap_err(), QdnfError::Conflict);
        assert_eq!(v.slot(slot).unwrap().expiry_unix, expiry);
    }

    #[test]
    fn stale_generation_rejected() {
        let mut v = LocalVault::new();
        let slot = store_ref(&mut v, digest(8), 1, 50).unwrap();
        let mut kr = v.issued_ref(slot).unwrap();
        kr.generation = Generation::ZERO;
        assert_eq!(
            load_key_ref(&mut v, kr, 2).unwrap_err(),
            QdnfError::StaleGeneration
        );
    }

    #[test]
    fn capacity_eight() {
        let mut v = LocalVault::new();
        let mut i = 0u8;
        while i < VAULT_SLOTS as u8 {
            store_ref(&mut v, digest(10 + i), 1, 100).unwrap();
            i += 1;
        }
        assert_eq!(
            store_ref(&mut v, digest(99), 2, 100).unwrap_err(),
            QdnfError::Capacity
        );
    }

    #[test]
    fn sealed_profile_blob_round_trip() {
        let v = LocalVault::new();
        let key = [7u8; AEAD_KEY_LEN];
        let nonce = [3u8; AEAD_NONCE_LEN];
        let blob = seal_profile_blob(&v, ProtectionProfile::P2, &key, &nonce).unwrap();
        assert_eq!(
            open_profile_blob(&v, &blob, &key).unwrap(),
            ProtectionProfile::P2
        );
    }

    #[test]
    fn sealed_profile_wrong_key_fails_closed() {
        let v = LocalVault::new();
        let key = [7u8; AEAD_KEY_LEN];
        let nonce = [3u8; AEAD_NONCE_LEN];
        let blob = seal_profile_blob(&v, ProtectionProfile::P3, &key, &nonce).unwrap();
        let wrong = [8u8; AEAD_KEY_LEN];
        assert_eq!(
            open_profile_blob(&v, &blob, &wrong).unwrap_err(),
            QdnfError::CryptoFailure
        );
        let mut locked = LocalVault::new();
        lock(&mut locked);
        assert_eq!(
            seal_profile_blob(&locked, ProtectionProfile::P1, &key, &nonce).unwrap_err(),
            QdnfError::Denied
        );
    }
}
