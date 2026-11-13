//! Fixed candidate checkpoint checksum. Decoding never authenticates a resume record.
use super::bitmap::{geometry, validate_tail};
use super::wire::*;
use super::{
    BITMAP_BYTES, CandidateBitmapPage, IDENTITY_BYTES, ResumeHint, StorageError as E,
    StorageIdentity, decode_identity, encode_identity,
};
use crate::crypto::network::digest::sha384;
use crate::net::qdnf::types::StrongDigest;
pub const CHECKPOINT_BYTES: usize = 400;
const MAGIC: &[u8; 8] = b"QBSCP01\0";
/// The checksum is attacker-recomputable; all decoded fields remain untrusted hints.
#[derive(Clone, Copy, Debug)]
pub struct UntrustedCheckpoint {
    identity: StorageIdentity,
    page: u64,
    sequence: u64,
    payload_digest: StrongDigest,
}
impl UntrustedCheckpoint {
    pub const fn identity(&self) -> &StorageIdentity {
        &self.identity
    }
    pub const fn page_number(&self) -> u64 {
        self.page
    }
    pub const fn sequence(&self) -> u64 {
        self.sequence
    }
    pub fn bind_payload<'a>(
        &self,
        expected: &StorageIdentity,
        expected_page: u64,
        payload: &'a [u8; BITMAP_BYTES],
    ) -> Result<ResumeHint<'a>, E> {
        if self.identity != *expected {
            return Err(E::IdentityMismatch);
        }
        if self.page != expected_page {
            return Err(E::PageMismatch);
        }
        let (_, valid) = geometry(expected, expected_page)?;
        if sha384(payload) != self.payload_digest {
            return Err(E::DigestMismatch);
        }
        validate_tail(payload, valid)?;
        Ok(ResumeHint {
            identity: self.identity,
            page: self.page,
            sequence: self.sequence,
            payload,
        })
    }
}
pub fn encode_checkpoint(
    page: &CandidateBitmapPage<'_>,
    sequence: u64,
    out: &mut [u8],
) -> Result<usize, E> {
    let out = out.get_mut(..CHECKPOINT_BYTES).ok_or(E::Capacity)?;
    out.fill(0);
    out[..8].copy_from_slice(MAGIC);
    out[8..10].copy_from_slice(&1u16.to_be_bytes());
    encode_identity(page.identity(), &mut out[16..16 + IDENTITY_BYTES])?;
    out[272..280].copy_from_slice(&page.page_number().to_be_bytes());
    out[280..284].copy_from_slice(&(BITMAP_BYTES as u32).to_be_bytes());
    out[288..336].copy_from_slice(sha384(page.payload()).as_bytes());
    out[336..344].copy_from_slice(&sequence.to_be_bytes());
    let digest = sha384(&out[..352]);
    out[352..400].copy_from_slice(digest.as_bytes());
    Ok(CHECKPOINT_BYTES)
}
pub fn decode_checkpoint(bytes: &[u8]) -> Result<UntrustedCheckpoint, E> {
    if bytes.len() != CHECKPOINT_BYTES {
        return Err(E::Length);
    }
    if &bytes[..8] != MAGIC {
        return Err(E::Magic);
    }
    if u16_at(bytes, 8)? != 1 {
        return Err(E::Version);
    }
    if u16_at(bytes, 10)? != 0 {
        return Err(E::Flags);
    }
    zero(&bytes[12..16])?;
    zero(&bytes[284..288])?;
    zero(&bytes[344..352])?;
    if u32_at(bytes, 280)? != BITMAP_BYTES as u32 {
        return Err(E::Length);
    }
    if sha384(&bytes[..352]) != digest_at(bytes, 352)? {
        return Err(E::DigestMismatch);
    }
    let identity = decode_identity(&bytes[16..272])?;
    let page = u64_at(bytes, 272)?;
    geometry(&identity, page)?;
    Ok(UntrustedCheckpoint {
        identity,
        page,
        sequence: u64_at(bytes, 336)?,
        payload_digest: digest_at(bytes, 288)?,
    })
}
