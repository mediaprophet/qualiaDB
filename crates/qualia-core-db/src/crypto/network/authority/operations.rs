//! Scoped asynchronous provider-operation fencing, not key delivery.
//! Existing LeaseTable owns nonwrapping operation generations.
use super::types::{Pin, ReceiptStamp};
use super::{AuthorityReceipt, PinAuthority, PinHandle};
use crate::crypto::network::errors::CryptoError;
use crate::crypto::network::key_provider::ProviderGrant;
use crate::net::peer::runtime::handles::LeaseHandle;
use crate::net::qdnf::types::{Generation, StrongDigest};

#[derive(Clone, Copy)]
pub(super) struct PendingSlot {
    lease: LeaseHandle,
    pin: PinHandle,
}

/// Trusted-host provider snapshot. Its current generation must come from the
/// existing grant owner, not an untrusted message or the local pin generation.
pub struct KeyRequest<'a> {
    pub grant: &'a ProviderGrant,
    pub current_provider_generation: u64,
    pub cell_epoch: u64,
    pub deadline_unix: u64,
}

/// Nonforgeable, non-Copy process-local operation token; no secret bytes.
pub struct KeyOperation {
    stamp: ReceiptStamp,
    lease: LeaseHandle,
    grant: ProviderGrant,
    deadline_unix: u64,
}

/// Evidence that this operation was admissible when completed. This receipt
/// cannot release/export a key or stand in for a confirmed-session capability.
pub struct KeyCompletion {
    stamp: ReceiptStamp,
    operation_generation: Generation,
}
impl KeyCompletion {
    pub const fn principal(&self) -> StrongDigest {
        self.stamp.principal
    }
    pub const fn descriptor_digest(&self) -> &[u8; 48] {
        &self.stamp.descriptor
    }
    pub const fn operation_generation(&self) -> Generation {
        self.operation_generation
    }
}

impl PinAuthority {
    pub fn begin_key_operation(
        &mut self,
        receipt: &AuthorityReceipt,
        descriptor: &[u8; 48],
        request: KeyRequest<'_>,
        now: u64,
    ) -> Result<KeyOperation, CryptoError> {
        let pin = self.check_stamp(&receipt.stamp, descriptor, now)?;
        authorize_provider(
            pin,
            request.grant,
            request.current_provider_generation,
            request.cell_epoch,
            now,
        )?;
        let deadline = request
            .deadline_unix
            .min(pin.validity.expires_unix)
            .min(request.grant.deadline_unix);
        if now >= deadline {
            return Err(CryptoError::Expired);
        }
        // Capacity zero: this is a generation fence, NOT a data-buffer reserve.
        let lease = self.operations.acquire(0, true)?.handle;
        self.pending[lease.slot as usize] = Some(PendingSlot {
            lease,
            pin: receipt.stamp.pin,
        });
        Ok(KeyOperation {
            stamp: receipt.stamp,
            lease,
            grant: *request.grant,
            deadline_unix: deadline,
        })
    }

    /// Release once; a later callback cannot consume this or a reused slot.
    pub fn cancel_key_operation(&mut self, operation: &KeyOperation) -> Result<(), CryptoError> {
        self.consume_pending(operation)
    }

    /// A completion attempt consumes its exact pending lease before rechecking
    /// authority. Rejected work must start anew; no result bytes are exposed.
    pub fn complete_key_operation(
        &mut self,
        operation: &KeyOperation,
        current_grant: &ProviderGrant,
        current_provider_generation: u64,
        cell_epoch: u64,
        now: u64,
    ) -> Result<KeyCompletion, CryptoError> {
        self.consume_pending(operation)?;
        let pin = self.check_stamp(&operation.stamp, &operation.stamp.descriptor, now)?;
        if now >= operation.deadline_unix {
            return Err(CryptoError::Expired);
        }
        authorize_provider(
            pin,
            current_grant,
            current_provider_generation,
            cell_epoch,
            now,
        )?;
        if *current_grant != operation.grant {
            return Err(CryptoError::StaleGeneration);
        }
        Ok(KeyCompletion {
            stamp: operation.stamp,
            operation_generation: operation.lease.generation,
        })
    }

    pub fn pending_key_operations(&self) -> usize {
        self.operations.occupied_count()
    }

    fn consume_pending(&mut self, operation: &KeyOperation) -> Result<(), CryptoError> {
        if operation.stamp.pin.instance != self.instance {
            return Err(CryptoError::Unauthorized);
        }
        let slot = operation.lease.slot as usize;
        let current = self
            .pending
            .get(slot)
            .and_then(Option::as_ref)
            .ok_or(CryptoError::StaleGeneration)?;
        if current.lease != operation.lease || current.pin != operation.stamp.pin {
            return Err(CryptoError::StaleGeneration);
        }
        self.operations.release(operation.lease)?;
        self.pending[slot] = None;
        Ok(())
    }

    pub(super) fn invalidate_operations(&mut self, pin: PinHandle) -> Result<(), CryptoError> {
        for slot in 0..self.pending.len() {
            if let Some(current) = self.pending[slot] {
                if current.pin == pin {
                    self.operations.release(current.lease)?;
                    self.pending[slot] = None;
                }
            }
        }
        Ok(())
    }
}

fn authorize_provider(
    pin: &Pin,
    grant: &ProviderGrant,
    generation: u64,
    cell_epoch: u64,
    now: u64,
) -> Result<(), CryptoError> {
    grant.authorize(pin.epoch.purpose, pin.epoch.controller, now, generation)?;
    if grant.epoch != pin.epoch {
        return Err(CryptoError::Unauthorized);
    }
    if grant.cell_epoch != cell_epoch {
        return Err(CryptoError::StaleGeneration);
    }
    Ok(())
}
