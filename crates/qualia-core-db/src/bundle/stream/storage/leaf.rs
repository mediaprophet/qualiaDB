//! Mathematical inclusion over actual borrowed bytes; no disk-presence or rights claim.
use super::{StorageError, StorageIdentity};
use crate::bundle::stream::proof::verify_leaf;
use crate::net::qdnf::types::StrongDigest;
pub struct VerifiedLeafView<'a> {
    identity: StorageIdentity,
    index: u64,
    logical_len: usize,
    bytes: &'a [u8],
}
impl<'a> VerifiedLeafView<'a> {
    pub const fn identity(&self) -> &StorageIdentity {
        &self.identity
    }
    pub const fn leaf_index(&self) -> u64 {
        self.index
    }
    pub const fn logical_length(&self) -> usize {
        self.logical_len
    }
    pub const fn bytes(&self) -> &'a [u8] {
        self.bytes
    }
}
/// A ciphertext view proves inclusion only, not AEAD validity or authenticated plaintext.
pub fn verify_leaf_view<'a>(
    identity: &StorageIdentity,
    index: u64,
    bytes: &'a [u8],
    siblings: &[StrongDigest],
) -> Result<VerifiedLeafView<'a>, StorageError> {
    let spec = identity.tree_spec();
    let logical_len = spec
        .logical_leaf_length(index)
        .map_err(StorageError::Proof)?;
    verify_leaf(&spec, identity.fields().root, index, bytes, siblings)
        .map_err(StorageError::Proof)?;
    Ok(VerifiedLeafView {
        identity: *identity,
        index,
        logical_len,
        bytes,
    })
}

impl core::fmt::Debug for VerifiedLeafView<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("VerifiedLeafView")
            .field("index", &self.index)
            .field("logical_length", &self.logical_len)
            .field("encoded_length", &self.bytes.len())
            .finish_non_exhaustive()
    }
}
