use super::{ProofStorageBuffers, StorageStage, StorageStageControl};
use crate::bundle::stream::{
    runtime::{recovery_source::RetainedRecoverySource, SparsePageWorkspace},
    storage::CandidateBitmapPage,
};
use crate::{
    net::qdnf::{
        errors::QdnfError,
        types::{Generation, StrongDigest},
    },
    storage_driver::ranges::recovery::RangeRecoveryRead,
};
impl<'owners> StorageStage<'_, 'owners> {
    /// Validate same actual source/page/runtime owners, then borrow all existing
    /// buffers and retained handles without another reservation or escaped view.
    pub fn with_recovery_buffers<'root, T>(
        &mut self,
        now: u64,
        policy: Generation,
        workspace: &mut SparsePageWorkspace,
        source: &mut RetainedRecoverySource<'root>,
        siblings: &mut [StrongDigest],
        effect: impl for<'b, 'p, 's> FnOnce(
            &mut StorageStageControl<'b, 'owners>,
            ProofStorageBuffers<'b>,
            &mut CandidateBitmapPage<'p>,
            RangeRecoveryRead<'s, 'root>,
        ) -> T,
    ) -> Result<T, QdnfError> {
        self.runtime.validate_recovery_source(source)?;
        self.with_sparse_page_buffers(
            now,
            policy,
            workspace,
            siblings,
            |control, buffers, page| {
                source.with_read_view(|read| effect(control, buffers, page, read))
            },
        )
    }
}
