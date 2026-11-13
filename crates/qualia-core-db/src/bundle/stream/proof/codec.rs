use super::{ProofError, MAX_SIBLINGS};
use crate::net::qdnf::types::StrongDigest;

pub fn encode_proof(siblings: &[StrongDigest], out: &mut [u8]) -> Result<usize, ProofError> {
    if siblings.len() > MAX_SIBLINGS {
        return Err(ProofError::ProofLength);
    }
    let length = 1 + siblings.len() * 48;
    if out.len() < length {
        return Err(ProofError::Capacity);
    }
    out[0] = siblings.len() as u8;
    for (chunk, digest) in out[1..length].chunks_exact_mut(48).zip(siblings) {
        chunk.copy_from_slice(digest.as_bytes());
    }
    Ok(length)
}
pub fn decode_proof<'a>(
    encoded: &[u8],
    out: &'a mut [StrongDigest],
) -> Result<&'a [StrongDigest], ProofError> {
    let count = *encoded.first().ok_or(ProofError::ProofLength)? as usize;
    if count > MAX_SIBLINGS || encoded.len() != 1 + count * 48 {
        return Err(ProofError::ProofLength);
    }
    if out.len() < count {
        return Err(ProofError::Capacity);
    }
    for (slot, bytes) in out[..count].iter_mut().zip(encoded[1..].chunks_exact(48)) {
        slot.0.copy_from_slice(bytes);
    }
    Ok(&out[..count])
}
