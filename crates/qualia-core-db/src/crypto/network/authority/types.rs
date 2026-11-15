use crate::crypto::network::cose::{self, KeyIds, PairedPublicKeys};
use crate::crypto::network::errors::CryptoError;
use crate::crypto::network::types::{KeyEpoch, ML_DSA_65_PK_LEN};
use crate::net::qdnf::authority::TemporalGrant;
use crate::net::qdnf::types::{Generation, StrongDigest};

/// Explicit host-local bootstrap capability. Keep outside untrusted handlers.
/// This library does not authenticate the human/operator who receives it.
pub struct LocalBootstrap {
    pub(super) instance: [u8; 16],
}

/// Process-local pin selection; private fields, no wire decoder.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PinHandle {
    pub(super) instance: [u8; 16],
    pub(super) slot: u8,
    pub(super) generation: Generation,
}
impl PinHandle {
    pub const fn generation(&self) -> Generation {
        self.generation
    }
}

/// Full public pair and exact scope adopted by a trusted local administrator.
pub struct PinConfig<'a> {
    pub principal: StrongDigest,
    pub provider_epoch: KeyEpoch,
    pub keys: PairedPublicKeys<'a>,
    pub purpose: &'a [u8],
    pub audience: &'a [u8],
    pub not_before_unix: u64,
    pub expires_unix: u64,
}

/// Only LocalPin is supported in this candidate; no opportunistic key import.
pub enum AuthoritySource<'a> {
    LocalPin(PinHandle),
    SignedControllerChain(&'a [u8]),
    ClassicalDid(&'a [u8]),
}

#[derive(Clone, Copy)]
pub(super) struct ReceiptStamp {
    pub pin: PinHandle,
    pub principal: StrongDigest,
    pub descriptor: [u8; 48],
}

/// Both signatures and current explicit pinned authority were checked.
/// Not a semantic policy permit, consent grant or transferable network token.
pub struct AuthorityReceipt {
    pub(super) stamp: ReceiptStamp,
}
impl AuthorityReceipt {
    pub const fn principal(&self) -> StrongDigest {
        self.stamp.principal
    }
    pub const fn descriptor_digest(&self) -> &[u8; 48] {
        &self.stamp.descriptor
    }
    pub const fn pin_generation(&self) -> Generation {
        self.stamp.pin.generation
    }
}

pub(super) struct Scope {
    purpose: [u8; cose::MAX_PURPOSE_BYTES],
    audience: [u8; cose::MAX_AUDIENCE_BYTES],
    purpose_len: u8,
    audience_len: u8,
}
impl Scope {
    pub fn new(purpose: &[u8], audience: &[u8]) -> Result<Self, CryptoError> {
        if purpose.is_empty() || audience.is_empty() {
            return Err(CryptoError::Malformed);
        }
        if purpose.len() > cose::MAX_PURPOSE_BYTES || audience.len() > cose::MAX_AUDIENCE_BYTES {
            return Err(CryptoError::Capacity);
        }
        let mut out = Self {
            purpose: [0; cose::MAX_PURPOSE_BYTES],
            audience: [0; cose::MAX_AUDIENCE_BYTES],
            purpose_len: purpose.len() as u8,
            audience_len: audience.len() as u8,
        };
        out.purpose[..purpose.len()].copy_from_slice(purpose);
        out.audience[..audience.len()].copy_from_slice(audience);
        Ok(out)
    }
    pub fn purpose(&self) -> &[u8] {
        &self.purpose[..self.purpose_len as usize]
    }
    pub fn audience(&self) -> &[u8] {
        &self.audience[..self.audience_len as usize]
    }
    pub fn matches(&self, purpose: &[u8], audience: &[u8]) -> bool {
        self.purpose() == purpose && self.audience() == audience
    }
}

pub(super) struct Pin {
    pub principal: StrongDigest,
    pub epoch: KeyEpoch,
    pub ml_pk: [u8; ML_DSA_65_PK_LEN],
    pub ed_pk: [u8; 32],
    pub ids: KeyIds,
    pub scope: Scope,
    pub validity: TemporalGrant,
    pub generation: Generation,
    pub revoked: bool,
}
impl Pin {
    pub fn keys(&self) -> PairedPublicKeys<'_> {
        PairedPublicKeys {
            ml_dsa_65: &self.ml_pk,
            ed25519: &self.ed_pk,
        }
    }
    pub fn current_at(&self, now: u64) -> Result<(), CryptoError> {
        if self.revoked {
            return Err(CryptoError::Revoked);
        }
        self.validity.current_at(now)
    }
}
