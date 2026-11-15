//! Caller-owned candidate marks and untrusted restart hints; neither establishes presence.
#[path = "bitmap/owned.rs"]
mod owned;
pub use owned::OwnedIntegrityPage;

use super::{StorageError as E, StorageIdentity, VerifiedLeafView};
pub const BITMAP_BYTES: usize = 4096;
pub const LEAVES_PER_PAGE: u64 = (BITMAP_BYTES as u64) * 8;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MarkStatus {
    New,
    AlreadyMarked,
}
pub(super) fn geometry(identity: &StorageIdentity, page: u64) -> Result<(u64, usize), E> {
    let first = page.checked_mul(LEAVES_PER_PAGE).ok_or(E::Overflow)?;
    let count = identity.leaf_count();
    if first >= count {
        return Err(E::Bounds);
    }
    Ok((first, (count - first).min(LEAVES_PER_PAGE) as usize))
}
fn bit(identity: &StorageIdentity, page: u64, index: u64) -> Result<(usize, u8), E> {
    let (first, valid) = geometry(identity, page)?;
    if index >= identity.leaf_count() {
        return Err(E::Bounds);
    }
    if index < first {
        return Err(E::PageMismatch);
    }
    let local = index - first;
    if local >= valid as u64 {
        return Err(E::PageMismatch);
    }
    let local = usize::try_from(local).map_err(|_| E::Overflow)?;
    Ok((local / 8, 1u8 << (local % 8)))
}
pub(super) fn validate_tail(payload: &[u8; BITMAP_BYTES], valid: usize) -> Result<(), E> {
    let mut first = valid / 8;
    let bits = valid % 8;
    if bits != 0 {
        let mask = ((1u16 << bits) - 1) as u8;
        if payload[first] & !mask != 0 {
            return Err(E::Bounds);
        }
        first += 1;
    }
    if payload[first..].iter().any(|b| *b != 0) {
        return Err(E::Bounds);
    }
    Ok(())
}
/// Fresh marks require actual VerifiedLeafView; never constructed from resume bits.
pub struct CandidateBitmapPage<'a> {
    identity: StorageIdentity,
    page: u64,
    payload: &'a mut [u8; BITMAP_BYTES],
}
impl<'a> CandidateBitmapPage<'a> {
    pub fn new(
        identity: StorageIdentity,
        page: u64,
        payload: &'a mut [u8; BITMAP_BYTES],
    ) -> Result<Self, E> {
        geometry(&identity, page)?;
        payload.fill(0);
        Ok(Self {
            identity,
            page,
            payload,
        })
    }
    pub const fn identity(&self) -> &StorageIdentity {
        &self.identity
    }
    pub const fn page_number(&self) -> u64 {
        self.page
    }
    pub fn payload(&self) -> &[u8; BITMAP_BYTES] {
        self.payload
    }
    pub fn record_integrity(&mut self, leaf: &VerifiedLeafView<'_>) -> Result<MarkStatus, E> {
        if *leaf.identity() != self.identity {
            return Err(E::IdentityMismatch);
        }
        let (byte, mask) = bit(&self.identity, self.page, leaf.leaf_index())?;
        let previous = self.payload[byte] & mask != 0;
        self.payload[byte] |= mask;
        Ok(if previous {
            MarkStatus::AlreadyMarked
        } else {
            MarkStatus::New
        })
    }
    pub fn has_integrity_mark(&self, index: u64) -> Result<bool, E> {
        let (byte, mask) = bit(&self.identity, self.page, index)?;
        Ok(self.payload[byte] & mask != 0)
    }
}
/// Borrowed untrusted bitmap claims. No method promotes a claim to verified leaf bytes.
pub struct ResumeHint<'a> {
    pub(super) identity: StorageIdentity,
    pub(super) page: u64,
    pub(super) sequence: u64,
    pub(super) payload: &'a [u8; BITMAP_BYTES],
}
impl ResumeHint<'_> {
    pub const fn identity(&self) -> &StorageIdentity {
        &self.identity
    }
    pub const fn page_number(&self) -> u64 {
        self.page
    }
    pub const fn sequence(&self) -> u64 {
        self.sequence
    }
    pub fn claims_leaf(&self, index: u64) -> Result<bool, E> {
        let (byte, mask) = bit(&self.identity, self.page, index)?;
        Ok(self.payload[byte] & mask != 0)
    }
}

impl core::fmt::Debug for ResumeHint<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ResumeHint")
            .field("page", &self.page)
            .field("sequence", &self.sequence)
            .finish_non_exhaustive()
    }
}
impl core::fmt::Debug for CandidateBitmapPage<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CandidateBitmapPage")
            .field("page", &self.page)
            .finish_non_exhaustive()
    }
}
