//! Shared existing managed-operation fence, extracted from managed_signing.
//! managed_signing retains SigningControl as a compatibility re-export.
use crate::crypto::network::errors::CryptoError;
use std::sync::{Mutex, MutexGuard};
pub(super) struct ControlState {
    pub(super) generation: u64,
    pub(super) cancelled: bool,
}
/// Local host fence, not authentication or execution authority. Cancel is terminal.
pub struct ManagedOperationControl {
    state: Mutex<ControlState>,
}
impl ManagedOperationControl {
    pub fn new(authority_generation: u64) -> Result<Self, CryptoError> {
        if authority_generation == 0 {
            return Err(CryptoError::StaleGeneration);
        }
        Ok(Self {
            state: Mutex::new(ControlState {
                generation: authority_generation,
                cancelled: false,
            }),
        })
    }
    pub(super) fn lock(&self) -> Result<MutexGuard<'_, ControlState>, CryptoError> {
        self.state.lock().map_err(|_| CryptoError::Unauthorized)
    }
    pub fn cancel(&self) -> Result<(), CryptoError> {
        self.lock()?.cancelled = true;
        Ok(())
    }
    pub fn advance_generation(&self, next: u64) -> Result<(), CryptoError> {
        let mut state = self.lock()?;
        if next <= state.generation {
            return Err(CryptoError::StaleGeneration);
        }
        state.generation = next;
        Ok(())
    }
}
