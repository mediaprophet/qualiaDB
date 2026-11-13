use super::{SelectedStoreBinding, StoreError as E};
use crate::bundle::stream::{
    proof::{Representation, LEAF_BYTES, MAX_SIBLINGS},
    storage::{StorageIdentity, BITMAP_BYTES, CHECKPOINT_BYTES, LEAVES_PER_PAGE},
};
use crate::storage_driver::ranges::RangeLimits;
pub const PROOF_SLOT_BYTES: usize = 1 + 48 * MAX_SIBLINGS;
pub const CHECKPOINT_BANK_BYTES: u64 = (CHECKPOINT_BYTES + BITMAP_BYTES) as u64;
pub const CHECKPOINT_PAGE_BYTES: u64 = 2 * CHECKPOINT_BANK_BYTES;
pub const WRITE_SCRATCH_BYTES: usize = PROOF_SLOT_BYTES + CHECKPOINT_BYTES;
pub const READ_SCRATCH_BYTES: usize = WRITE_SCRATCH_BYTES + LEAF_BYTES as usize + BITMAP_BYTES;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StoreLayout {
    identity: StorageIdentity,
    pages: u64,
    limits: RangeLimits,
}
impl StoreLayout {
    pub fn for_binding(binding: &SelectedStoreBinding) -> Result<Self, E> {
        Self::for_identity(binding.identity())
    }
    pub(super) fn for_identity(identity: &StorageIdentity) -> Result<Self, E> {
        let f = identity.fields();
        if f.representation != Representation::Plaintext {
            return Err(E::Unsupported);
        }
        let leaves = identity.leaf_count();
        if leaves == 0 {
            return Err(E::Empty);
        }
        let pages = leaves.checked_add(LEAVES_PER_PAGE - 1).ok_or(E::Overflow)? / LEAVES_PER_PAGE;
        let limits = RangeLimits {
            payload_bytes: f.logical_length,
            proof_bytes: leaves
                .checked_mul(PROOF_SLOT_BYTES as u64)
                .ok_or(E::Overflow)?,
            checkpoint_bytes: pages
                .checked_mul(CHECKPOINT_PAGE_BYTES)
                .ok_or(E::Overflow)?,
        };
        limits.total()?;
        Ok(Self {
            identity: *identity,
            pages,
            limits,
        })
    }
    pub const fn range_limits(self) -> RangeLimits {
        self.limits
    }
    pub fn aggregate_allowance(self) -> Result<u64, E> {
        Ok(self.limits.total()?)
    }
    pub const fn page_count(self) -> u64 {
        self.pages
    }
    pub fn leaf_offset(self, leaf: u64) -> Result<u64, E> {
        self.identity.tree_spec().logical_leaf_length(leaf)?;
        leaf.checked_mul(LEAF_BYTES).ok_or(E::Overflow)
    }
    pub fn proof_offset(self, leaf: u64) -> Result<u64, E> {
        self.identity.tree_spec().logical_leaf_length(leaf)?;
        leaf.checked_mul(PROOF_SLOT_BYTES as u64).ok_or(E::Overflow)
    }
    pub fn checkpoint_offset(self, page: u64, bank: u8) -> Result<u64, E> {
        if page >= self.pages || bank > 1 {
            return Err(E::Bounds);
        }
        page.checked_mul(2)
            .and_then(|n| n.checked_add(u64::from(bank)))
            .and_then(|n| n.checked_mul(CHECKPOINT_BANK_BYTES))
            .ok_or(E::Overflow)
    }
}
