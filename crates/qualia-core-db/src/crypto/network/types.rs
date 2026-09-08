//! Network-facing crypto types. Full digests; compact hashes stay in QDNF types.

use crate::net::qdnf::errors::QdnfError;

pub const SHA384_LEN: usize = 48;
pub const X25519_LEN: usize = 32;
pub const AEAD_KEY_LEN: usize = 32;
pub const AEAD_NONCE_LEN: usize = 12;
pub const AEAD_TAG_LEN: usize = 16;
pub const ML_KEM_768_PK_LEN: usize = 1184;
pub const ML_KEM_768_SK_LEN: usize = 2400;
pub const ML_KEM_768_CT_LEN: usize = 1088;
pub const ML_KEM_SS_LEN: usize = 32;
pub const ML_DSA_65_PK_LEN: usize = 1952;
pub const ML_DSA_65_SK_LEN: usize = 4032;
pub const ML_DSA_65_SIG_LEN: usize = 3309;
pub const ED25519_PK_LEN: usize = 32;
pub const ED25519_SK_LEN: usize = 32;
pub const ED25519_SIG_LEN: usize = 64;

/// Purpose labels bound into every provider grant.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyPurpose {
    QLinkEphemeral = 1,
    QLinkAead = 2,
    QSessionEphemeral = 3,
    QSessionAead = 4,
    ControllerSign = 5,
    RouteUpdate = 6,
    RecordSign = 7,
    DiscoveryHmac = 8,
}

/// Typed algorithm/length pair. Reject unknown combinations.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AlgorithmSpec {
    pub algorithm: u16,
    pub length: u16,
}

impl AlgorithmSpec {
    pub const SHA384: Self = Self {
        algorithm: 1,
        length: SHA384_LEN as u16,
    };
    pub const X25519: Self = Self {
        algorithm: 4,
        length: X25519_LEN as u16,
    };
    pub const ML_KEM_768: Self = Self {
        algorithm: 5,
        length: ML_KEM_768_PK_LEN as u16,
    };

    pub fn check(self, actual_len: usize) -> Result<(), QdnfError> {
        if actual_len != self.length as usize {
            return Err(QdnfError::Range);
        }
        Ok(())
    }
}

/// Key epoch bound to a controller and purpose.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyEpoch {
    pub purpose: KeyPurpose,
    pub epoch: u64,
    pub controller: u64,
}
