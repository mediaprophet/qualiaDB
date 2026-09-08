//! Persistent controller identity. Replaces libp2p `PeerId`.
//!
//! A transport/link key is not this value. `LinkId` remains epoch-scoped and
//! separate from the controller digest.

use crate::crypto::network::digest::sha384;
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::StrongDigest;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ControllerIdentity {
    digest: StrongDigest,
}

impl ControllerIdentity {
    pub fn from_controller(bytes: &[u8]) -> Result<Self, QdnfError> {
        if bytes.is_empty() {
            return Err(QdnfError::Range);
        }
        Ok(Self {
            digest: sha384(bytes),
        })
    }

    #[inline]
    pub const fn digest(self) -> StrongDigest {
        self.digest
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn controller_digest_is_not_raw_key_bytes() {
        let id = ControllerIdentity::from_controller(b"did:q42:example").unwrap();
        assert_ne!(id.digest(), StrongDigest::ZERO);
        assert_ne!(&id.digest().0[..5], b"did:q");
    }
}
