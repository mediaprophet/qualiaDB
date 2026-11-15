use crate::bundle::stream::storage::{CandidateBitmapPage, OwnedIntegrityPage, StorageIdentity};
use crate::net::peer::runtime::ReservationHandle;
use crate::net::qdnf::{errors::QdnfError, types::Generation};

pub const MAX_SPARSE_WORKSPACES: usize = 8;

/// Actual fixed page memory coupled to a private canonical runtime incarnation.
/// Drop never releases its reservation or removes the runtime's live record.
#[must_use = "release explicitly through the original TransferRuntime"]
#[derive(Debug)]
pub struct SparsePageWorkspace {
    pub(super) owner: usize,
    pub(super) slot: u8,
    pub(super) generation: Generation,
    pub(super) store_incarnation: Generation,
    pub(super) page: OwnedIntegrityPage,
}
impl SparsePageWorkspace {
    pub fn identity(&self) -> &StorageIdentity {
        self.page.identity()
    }
    pub fn page_number(&self) -> u64 {
        self.page.page_number()
    }
    pub fn has_integrity_mark(
        &self,
        leaf: u64,
    ) -> Result<bool, crate::bundle::stream::storage::StorageError> {
        self.page.has_integrity_mark(leaf)
    }
    pub const fn charged_bytes() -> usize {
        core::mem::size_of::<Self>()
    }
    pub(in super::super) fn with_candidate_page<T>(
        &mut self,
        effect: impl for<'a> FnOnce(&mut CandidateBitmapPage<'a>) -> T,
    ) -> T {
        self.page.with_candidate_page(effect)
    }
}

/// Every failure returns the actual non-Copy workspace, allowing correct-owner retry.
#[must_use = "retain the workspace and retry release through its original owner"]
#[derive(Debug)]
pub struct WorkspaceReleaseError {
    pub error: QdnfError,
    pub workspace: SparsePageWorkspace,
}

pub(in super::super) struct SparseWorkspaceSlot {
    pub(in super::super) generation: Generation,
    pub(in super::super) charge: Option<ReservationHandle>,
    pub(in super::super) identity: Option<StorageIdentity>,
    pub(in super::super) page: u64,
}
impl SparseWorkspaceSlot {
    pub(in super::super) const fn empty() -> Self {
        Self {
            generation: Generation::ZERO,
            charge: None,
            identity: None,
            page: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SparseWorkspaceKey {
    owner: usize,
    slot: u8,
    generation: Generation,
    store_incarnation: Generation,
}
impl SparsePageWorkspace {
    pub(crate) fn binding_key(&self) -> SparseWorkspaceKey {
        SparseWorkspaceKey {
            owner: self.owner,
            slot: self.slot,
            generation: self.generation,
            store_incarnation: self.store_incarnation,
        }
    }
    /// A new facade cannot inherit marks from another backend/store incarnation.
    /// This only removes marks and cannot manufacture a verified leaf.
    pub(crate) fn reset_for_store(&mut self) -> Result<SparseWorkspaceKey, QdnfError> {
        let next = self.store_incarnation.next()?;
        self.page.invalidate();
        self.store_incarnation = next;
        Ok(self.binding_key())
    }
    pub(crate) fn check_binding_key(&self, key: SparseWorkspaceKey) -> Result<(), QdnfError> {
        if self.binding_key() != key {
            return Err(QdnfError::StaleGeneration);
        }
        Ok(())
    }
    /// Arm before possible effects; commit only after the outer policy wrapper
    /// AND final runtime check succeed. Failure/unwind mechanically clears marks.
    pub(crate) fn begin_attempt(&mut self) -> WorkspacePageAttempt<'_> {
        WorkspacePageAttempt {
            workspace: self,
            committed: false,
        }
    }
}
#[must_use = "dropping an uncommitted attempt invalidates the page"]
pub(crate) struct WorkspacePageAttempt<'a> {
    workspace: &'a mut SparsePageWorkspace,
    committed: bool,
}
impl WorkspacePageAttempt<'_> {
    pub(crate) fn workspace(&mut self) -> &mut SparsePageWorkspace {
        self.workspace
    }
    pub(crate) fn commit(mut self) {
        self.committed = true;
    }
}
impl Drop for WorkspacePageAttempt<'_> {
    fn drop(&mut self) {
        if !self.committed {
            self.workspace.page.invalidate();
        }
    }
}
