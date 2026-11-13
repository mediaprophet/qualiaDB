//! Bounded resource/lifecycle adapter; not source authority or verified consumption.
pub mod sparse_workspace;
pub use sparse_workspace::{SparsePageWorkspace, WorkspaceReleaseError, MAX_SPARSE_WORKSPACES};
mod dispatch;
mod owner;
mod state;
pub(crate) use state::{RetainedRuntime, RuntimeAccess, RuntimeCloseError, RuntimeParents};
#[cfg(not(target_arch = "wasm32"))]
pub mod recovery_source;
#[cfg(not(target_arch = "wasm32"))]
pub use recovery_source::{
    RetainedRecoverySource, SourceError, SourceReleaseError, MAX_RETAINED_SOURCES,
};
mod plan;
mod storage_stage;
mod transfer;
pub use storage_stage::{ProofStorageBuffers, StorageBuffers, StorageStage, StorageStageControl};
mod types;
pub use owner::TransferRuntime;
pub use plan::{ColdPlan, TransferRequirements, TransferSpec, MAX_PLAN_ROWS};
pub use types::{
    CancelReason, CompletionKey, FinishOutcome, RequestCredit, RuntimePoll, RuntimeProfile,
    TransferState, TransferTicket,
};
#[cfg(test)]
mod tests;
