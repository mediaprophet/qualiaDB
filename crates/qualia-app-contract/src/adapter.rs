//! Canonical AppAdapter trait defining the standardized engine boundary.

use crate::descriptor::Availability;
use crate::error::ContractError;
use crate::gesture::{GestureCancel, GestureCommit, GestureSession, GestureUpdate};
use crate::id::{CommandId, DecimalU64};
use crate::request::{AppSessionDescriptor, CommandReceipt, CommandRequest, JobDescriptor};
use crate::target::CommandTarget;

/// Standardized adapter interface implemented by all application engines.
///
/// Ensures uniform hosting across browser Web Workers, native worker processes,
/// and embedded shell stages.
pub trait AppAdapter: Send + Sync {
    /// Return the session and engine metadata descriptor.
    fn describe(&self) -> Result<AppSessionDescriptor, ContractError>;

    /// Open or create a document session in the specified mode ("read_only" or "read_write").
    fn open(&mut self, _document_id: &str, _open_mode: &str) -> Result<DecimalU64, ContractError> {
        Ok(DecimalU64(0))
    }

    /// Check command availability given the active target and selection.
    fn availability(&self, _target: &CommandTarget, _command_id: &CommandId) -> Availability {
        Availability::Available
    }

    /// Read-only inspection of the specified target object.
    fn inspect(&self, _target: &CommandTarget) -> Result<serde_json::Value, ContractError> {
        Ok(serde_json::Value::Null)
    }

    /// Execute an authorized command request and return the resulting receipt.
    fn execute(&mut self, request: &CommandRequest) -> Result<CommandReceipt, ContractError>;

    /// Begin an interactive gesture stream.
    fn begin_gesture(&mut self, _session: &GestureSession) -> Result<(), ContractError> {
        Err(ContractError::GesturesUnsupported("gestures unsupported".into()))
    }

    /// Stream incremental pointer updates to the active gesture.
    fn update_gesture(&mut self, _update: &GestureUpdate) -> Result<(), ContractError> {
        Err(ContractError::GesturesUnsupported("gestures unsupported".into()))
    }

    /// Commit an active gesture into an authoritative document revision.
    fn commit_gesture(&mut self, _commit: &GestureCommit) -> Result<CommandReceipt, ContractError> {
        Err(ContractError::GesturesUnsupported("gestures unsupported".into()))
    }

    /// Cancel an active gesture, discarding in-flight changes and restoring baseline revision.
    fn cancel_gesture(&mut self, _cancel: &GestureCancel) -> Result<(), ContractError> {
        Err(ContractError::GesturesUnsupported("gestures unsupported".into()))
    }

    /// Produce an opaque serialized snapshot of current document state.
    fn snapshot(&self) -> Result<Vec<u8>, ContractError> {
        Ok(Vec::new())
    }

    /// Restore document state from a serialized snapshot.
    fn restore(&mut self, _snapshot_bytes: &[u8]) -> Result<(), ContractError> {
        Ok(())
    }

    /// Poll status of a background or asynchronous job.
    fn poll_job(&self, job_id: &str) -> Result<JobDescriptor, ContractError> {
        Err(ContractError::InvalidIdentifier(format!("job {job_id} not found")))
    }

    /// Cancel a running background job.
    fn cancel_job(&mut self, _job_id: &str) -> Result<(), ContractError> {
        Ok(())
    }

    /// Close the active document session, releasing held memory and resources.
    fn close(&mut self) -> Result<(), ContractError> {
        Ok(())
    }
}
