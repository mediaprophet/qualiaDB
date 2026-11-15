//! Fixed bounded COSE profile values and caller-owned public-message storage.
use crate::crypto::network::types::{ED25519_SIG_LEN, ML_DSA_65_SIG_LEN};

pub const PROFILE: &str = "qbdl-stream-pq-1";
pub const SCHEMA_VERSION: u64 = 1;
pub const ML_DSA_65: i64 = -49;
pub const ED25519: i64 = -19;
pub const EXTERNAL_AAD: &[u8] = b"qbdl-stream-pq-1/paired-sign1/v1";
pub const MAX_BINDING_BYTES: usize = 512;
pub const MAX_PROTECTED_BYTES: usize = 64;
pub const MAX_SIG_STRUCTURE_BYTES: usize = 704;
pub const MAX_ENVELOPE_BYTES: usize = 4608;
pub const MAX_PURPOSE_BYTES: usize = 64;
pub const MAX_AUDIENCE_BYTES: usize = 128;

/// Application-domain SHA-384 references, not COSE thumbprints.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyIds {
    pub ml_dsa_65: [u8; 48],
    pub ed25519: [u8; 48],
}

/// Unverified binding. Public fields confer no authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Binding<'a> {
    pub descriptor_digest: &'a [u8; 48],
    pub key_ids: KeyIds,
    pub controller_epoch: u64,
    pub purpose: &'a [u8],
    pub audience: &'a [u8],
}

/// Caller-owned output signatures. Byte presence is not verification.
pub struct PairSignatures {
    pub ml_dsa_65: [u8; ML_DSA_65_SIG_LEN],
    pub ed25519: [u8; ED25519_SIG_LEN],
}

impl PairSignatures {
    pub const fn new() -> Self {
        Self {
            ml_dsa_65: [0; ML_DSA_65_SIG_LEN],
            ed25519: [0; ED25519_SIG_LEN],
        }
    }
}

impl Default for PairSignatures {
    fn default() -> Self {
        Self::new()
    }
}

/// Public-message scratch. Provider-internal resources are separately qualified.
pub struct Workspace {
    pub(crate) binding: [u8; MAX_BINDING_BYTES],
    pub(crate) sig_structure: [u8; MAX_SIG_STRUCTURE_BYTES],
    pub(crate) signatures: PairSignatures,
}

impl Workspace {
    pub const fn new() -> Self {
        Self {
            binding: [0; MAX_BINDING_BYTES],
            sig_structure: [0; MAX_SIG_STRUCTURE_BYTES],
            signatures: PairSignatures::new(),
        }
    }
}

impl Default for Workspace {
    fn default() -> Self {
        Self::new()
    }
}
