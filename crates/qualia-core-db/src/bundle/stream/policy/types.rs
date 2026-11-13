use crate::crypto::network::errors::CryptoError;
use crate::net::qdnf::{errors::QdnfError, types::StrongDigest};
use crate::semantic_instruments::resolve::bounded_closure::ClosureError;
pub const CLOSURE_PURPOSE: &[u8] = b"qbdl-stream/closure-issuer/v1";
pub const PUBLICATION_PURPOSE: &[u8] = b"qbdl-stream/publisher/v1";
pub const MAX_MANIFEST_BYTES: usize = 2048;
pub(super) const MAGIC: &[u8] = b"QBDL-CLOSURE-V1\0";
/// Untrusted input fields. Issuer is recognized only by an installed paired pin.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ManifestFields {
    pub source: StrongDigest,
    pub source_length: u64,
    pub semantic_root: StrongDigest,
    pub policy: StrongDigest,
    pub consumer: StrongDigest,
    pub issuer: StrongDigest,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PolicyError {
    Capacity,
    Encoding,
    Limit,
    Reference,
    Graph,
    Descriptor,
    Spent,
    Crypto(CryptoError),
    Authority(QdnfError),
    Closure(ClosureError),
}
impl From<CryptoError> for PolicyError {
    fn from(e: CryptoError) -> Self {
        Self::Crypto(e)
    }
}

/// Untrusted proof inputs. Selected publisher pin is locally installed and role scoped.
pub struct PublicationProof<'a> {
    pub pin: crate::crypto::network::authority::PinHandle,
    pub key_ids: crate::crypto::network::cose::KeyIds,
    pub epoch: u64,
    pub audience: &'a [u8],
    pub envelope: &'a [u8],
}
