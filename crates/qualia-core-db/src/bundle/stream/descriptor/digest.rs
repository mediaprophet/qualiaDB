//! Domain framing only; the canonical crypto library owns SHA384.
use super::{DescriptorError, MAX_ENCODED};
use crate::crypto::network::digest::sha384;
use crate::net::qdnf::types::StrongDigest;
pub const HASH_SCRATCH_BYTES: usize = MAX_ENCODED + 64;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DigestDomain {
    Context,
    ContextPage,
    EntryPage,
    Body,
}
pub fn domain_digest(
    domain: DigestDomain,
    encoded: &[u8],
    scratch: &mut [u8],
) -> Result<StrongDigest, DescriptorError> {
    if encoded.len() > MAX_ENCODED {
        return Err(DescriptorError::Limit);
    }
    let prefix: &[u8] = match domain {
        DigestDomain::Context => b"QBDL-QASD-CONTEXT-V1\0",
        DigestDomain::ContextPage => b"QBDL-QASD-CONTEXT-PAGE-V1\0",
        DigestDomain::EntryPage => b"QBDL-QASD-ENTRY-PAGE-V1\0",
        DigestDomain::Body => b"QBDL-QASD-BODY-V1\0",
    };
    let len = prefix
        .len()
        .checked_add(encoded.len())
        .ok_or(DescriptorError::Overflow)?;
    let out = scratch.get_mut(..len).ok_or(DescriptorError::Capacity)?;
    out[..prefix.len()].copy_from_slice(prefix);
    out[prefix.len()..].copy_from_slice(encoded);
    Ok(sha384(out))
}
