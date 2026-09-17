//! Governed service identifiers. Replaces libp2p `StreamProtocol` strings.

use crate::crypto::network::digest::sha384;
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::StrongDigest;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ServiceId {
    digest: StrongDigest,
}

impl ServiceId {
    pub fn from_iri(iri: &[u8]) -> Result<Self, QdnfError> {
        if iri.is_empty() {
            return Err(QdnfError::Range);
        }
        Ok(Self {
            digest: sha384(iri),
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
    fn unknown_iri_is_still_a_typed_digest() {
        let a = ServiceId::from_iri(b"q42:QSync/1").unwrap();
        let b = ServiceId::from_iri(b"q42:QSync/2").unwrap();
        assert_ne!(a, b);
    }
}
