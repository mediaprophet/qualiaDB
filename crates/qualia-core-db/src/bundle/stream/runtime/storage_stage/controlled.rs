//! Recheck lifecycle while proof and payload borrows remain alive.
use super::{StorageStage, TransferRuntime, TransferTicket};
use crate::net::qdnf::{errors::QdnfError, types::Generation};

/// Disjoint view of already charged caller storage. These bytes convey no
/// integrity, persistence or disclosure permission on their own.
pub struct StorageBuffers<'a> {
    pub payload: &'a [u8],
    pub proof: &'a [u8],
    pub scratch: &'a mut [u8],
}
/// Same live runtime and ticket; never an additional reservation or permit.
/// Available only within a bounded synchronous StorageStage callback.
pub struct StorageStageControl<'a, 'owners> {
    runtime: &'a mut TransferRuntime<'owners>,
    ticket: &'a TransferTicket,
}
impl StorageStageControl<'_, '_> {
    pub fn recheck(&mut self, now: u64, policy: Generation) -> Result<(), QdnfError> {
        self.runtime.check_storage_stage(self.ticket, now, policy)
    }
    pub fn cancel(&mut self) -> Result<(), QdnfError> {
        self.runtime.cancel(self.ticket)
    }
}
impl<'owners> StorageStage<'_, 'owners> {
    /// Borrow the existing charged buffers and lifecycle control disjointly.
    /// The callback can retain a proof view while checking fresh host time and
    /// policy immediately before each bounded storage effect. No borrow may
    /// escape the callback. Resource completion remains excluded until the
    /// entire stage ends; returning or unwinding never pretends I/O completed.
    /// This supplies lifecycle checks only: source authority stays with its
    /// canonical policy owner and the host must sample those inputs freshly.
    pub fn with_buffers<T>(
        &mut self,
        now: u64,
        policy: Generation,
        effect: impl for<'b> FnOnce(&mut StorageStageControl<'b, 'owners>, StorageBuffers<'b>) -> T,
    ) -> Result<T, QdnfError> {
        self.recheck(now, policy)?;
        let mut control = StorageStageControl {
            runtime: self.runtime,
            ticket: self.ticket,
        };
        let buffers = StorageBuffers {
            payload: self.payload,
            proof: self.proof,
            scratch: self.scratch,
        };
        Ok(effect(&mut control, buffers))
    }
}

/// Typed proof scratch is borrowed and charged alongside ordinary storage
/// scratch; no alignment cast or hidden sibling allocation is required.
pub struct ProofStorageBuffers<'a> {
    pub bytes: StorageBuffers<'a>,
    pub siblings: &'a mut [crate::net::qdnf::types::StrongDigest],
}
impl<'owners> StorageStage<'_, 'owners> {
    pub fn with_proof_buffers<T>(
        &mut self,
        now: u64,
        policy: Generation,
        siblings: &mut [crate::net::qdnf::types::StrongDigest],
        effect: impl for<'b> FnOnce(&mut StorageStageControl<'b, 'owners>, ProofStorageBuffers<'b>) -> T,
    ) -> Result<T, QdnfError> {
        self.recheck(now, policy)?;
        if siblings.len() > crate::bundle::stream::proof::MAX_SIBLINGS {
            return Err(QdnfError::Capacity);
        }
        let bytes = self
            .scratch
            .len()
            .checked_add(core::mem::size_of_val(siblings))
            .ok_or(QdnfError::Capacity)?;
        let slot = &self.runtime.slots[self.runtime.validate(self.ticket)?];
        if bytes as u64 > slot.storage_bytes {
            return Err(QdnfError::Capacity);
        }
        let mut control = StorageStageControl {
            runtime: self.runtime,
            ticket: self.ticket,
        };
        let buffers = ProofStorageBuffers {
            bytes: StorageBuffers {
                payload: self.payload,
                proof: self.proof,
                scratch: self.scratch,
            },
            siblings,
        };
        Ok(effect(&mut control, buffers))
    }
}
