//! Fixed-width big-endian helpers; caller lengths are checked, no heap scratch.
use super::StorageError as E;
use crate::net::qdnf::types::StrongDigest;
pub(super) fn array<const N: usize>(bytes: &[u8], start: usize) -> Result<[u8; N], E> {
    let end = start.checked_add(N).ok_or(E::Overflow)?;
    bytes
        .get(start..end)
        .ok_or(E::Length)?
        .try_into()
        .map_err(|_| E::Length)
}
pub(super) fn u16_at(bytes: &[u8], start: usize) -> Result<u16, E> {
    Ok(u16::from_be_bytes(array(bytes, start)?))
}
pub(super) fn u32_at(bytes: &[u8], start: usize) -> Result<u32, E> {
    Ok(u32::from_be_bytes(array(bytes, start)?))
}
pub(super) fn u64_at(bytes: &[u8], start: usize) -> Result<u64, E> {
    Ok(u64::from_be_bytes(array(bytes, start)?))
}
pub(super) fn digest_at(bytes: &[u8], start: usize) -> Result<StrongDigest, E> {
    Ok(StrongDigest(array(bytes, start)?))
}
pub(super) fn zero(bytes: &[u8]) -> Result<(), E> {
    if bytes.iter().any(|b| *b != 0) {
        Err(E::Reserved)
    } else {
        Ok(())
    }
}
