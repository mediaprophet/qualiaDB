//! Fixed owned integrity page. Resume hints cannot initialize current marks.
use super::{bit, geometry, CandidateBitmapPage, MarkStatus, BITMAP_BYTES};
use crate::bundle::stream::storage::{StorageError, StorageIdentity, VerifiedLeafView};

pub struct OwnedIntegrityPage {
    identity: StorageIdentity,
    page: u64,
    payload: [u8; BITMAP_BYTES],
}
impl OwnedIntegrityPage {
    pub fn new(identity: StorageIdentity, page: u64) -> Result<Self, StorageError> {
        geometry(&identity, page)?;
        Ok(Self {
            identity,
            page,
            payload: [0; BITMAP_BYTES],
        })
    }
    /// One-way invalidation removes every mark; it never imports resume hints.
    pub fn invalidate(&mut self) {
        self.payload.fill(0);
    }
    pub const fn identity(&self) -> &StorageIdentity {
        &self.identity
    }
    pub const fn page_number(&self) -> u64 {
        self.page
    }
    pub const fn payload(&self) -> &[u8; BITMAP_BYTES] {
        &self.payload
    }
    pub fn has_integrity_mark(&self, index: u64) -> Result<bool, StorageError> {
        let (byte, mask) = bit(&self.identity, self.page, index)?;
        Ok(self.payload[byte] & mask != 0)
    }
    pub fn record_integrity(
        &mut self,
        leaf: &VerifiedLeafView<'_>,
    ) -> Result<MarkStatus, StorageError> {
        self.with_candidate_page(|page| page.record_integrity(leaf))
    }
    /// Reborrows the same initialized page without clearing earlier real marks.
    /// Existing CandidateBitmapPage exposes no raw mutable payload/from-hint setter.
    pub fn with_candidate_page<T>(
        &mut self,
        effect: impl for<'a> FnOnce(&mut CandidateBitmapPage<'a>) -> T,
    ) -> T {
        let mut page = CandidateBitmapPage {
            identity: self.identity,
            page: self.page,
            payload: &mut self.payload,
        };
        effect(&mut page)
    }
}
impl core::fmt::Debug for OwnedIntegrityPage {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("OwnedIntegrityPage")
            .field("page", &self.page)
            .finish_non_exhaustive()
    }
}
