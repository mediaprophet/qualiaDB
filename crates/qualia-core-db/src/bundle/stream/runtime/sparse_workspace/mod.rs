//! Retained integrity-page memory charged by the existing canonical ledger.
//! There is no new root quota or forgeable numeric budget token. These bytes
//! outlive individual transfers; they are not charged again to StorageStage.
//! Parent integration must account table metadata via size_of::<TransferRuntime>,
//! refuse close while workspace_live != 0, and report30-w desktop headroom.
mod admission;
mod types;
pub(super) use types::SparseWorkspaceSlot;
pub use types::{SparsePageWorkspace, WorkspaceReleaseError, MAX_SPARSE_WORKSPACES};
#[cfg(test)]
mod tests;

pub(crate) use types::{SparseWorkspaceKey, WorkspacePageAttempt};
