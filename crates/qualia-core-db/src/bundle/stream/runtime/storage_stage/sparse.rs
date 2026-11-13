//! Scoped retained-page use under the same runtime/ticket and existing charges.
use super::{ProofStorageBuffers, StorageStage, StorageStageControl};
use crate::bundle::stream::{
    runtime::sparse_workspace::SparsePageWorkspace, storage::CandidateBitmapPage,
};
use crate::net::qdnf::{
    errors::QdnfError,
    types::{Generation, StrongDigest},
};

impl<'owners> StorageStage<'_, 'owners> {
    /// Validates the exact retained owner before reborrowing its real page.
    /// Typed siblings and ordinary byte scratch use the original transfer's
    /// storage component. The persistent page remains charged to its separate
    /// actual canonical reservation; it is never charged twice here.
    ///
    /// The native adapter must also match the selected store identity/page and
    /// keep WorkspacePageAttempt armed through outer policy/final runtime
    /// acceptance. No page, input slice, control or new authority escapes.
    pub fn with_sparse_page_buffers<T>(
        &mut self,
        now: u64,
        policy: Generation,
        workspace: &mut SparsePageWorkspace,
        siblings: &mut [StrongDigest],
        effect: impl for<'b, 'p> FnOnce(
            &mut StorageStageControl<'b, 'owners>,
            ProofStorageBuffers<'b>,
            &mut CandidateBitmapPage<'p>,
        ) -> T,
    ) -> Result<T, QdnfError> {
        self.runtime.validate_sparse_workspace(workspace)?;
        self.with_proof_buffers(now, policy, siblings, |control, buffers| {
            workspace.with_candidate_page(|page| effect(control, buffers, page))
        })
    }
}
