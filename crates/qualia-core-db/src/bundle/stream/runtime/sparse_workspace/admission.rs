use super::{SparsePageWorkspace, WorkspaceReleaseError};
use crate::bundle::stream::runtime::TransferRuntime;
use crate::bundle::stream::storage::{LEAVES_PER_PAGE, OwnedIntegrityPage, StorageIdentity};
use crate::net::peer::runtime::{ReservationHandle, ResourceBudget, WorkClass};
use crate::net::qdnf::errors::QdnfError;

impl TransferRuntime<'_> {
    /// Reserve the actual whole fixed owner before constructing page memory.
    /// Pages start zero; untrusted checkpoint bytes cannot initialize marks.
    pub fn acquire_sparse_page(
        &mut self,
        identity: StorageIdentity,
        page: u64,
    ) -> Result<SparsePageWorkspace, QdnfError> {
        self.require_open()?;
        let first = page.checked_mul(LEAVES_PER_PAGE).ok_or(QdnfError::Range)?;
        if first >= identity.leaf_count() {
            return Err(QdnfError::Range);
        }
        if self
            .sparse_workspaces
            .iter()
            .any(|s| s.charge.is_some() && s.identity == Some(identity) && s.page == page)
        {
            return Err(QdnfError::Conflict);
        }
        let index = self
            .sparse_workspaces
            .iter()
            .position(|s| s.charge.is_none() && s.generation.0 != u64::MAX)
            .ok_or(QdnfError::Capacity)?;
        let generation = self.sparse_workspaces[index].generation.next()?;
        let bytes =
            u64::try_from(SparsePageWorkspace::charged_bytes()).map_err(|_| QdnfError::Range)?;
        let next_window = self.used_bytes.checked_add(bytes).ok_or(QdnfError::Range)?;
        if next_window > self.profile.window_bytes() {
            return Err(QdnfError::BudgetExhausted);
        }
        // Each workspace takes one of the same32 parent reservation handles.
        let next_count = self.workspace_live.checked_add(1).ok_or(QdnfError::Range)?;
        let handle_ceiling = 30usize
            .saturating_sub(next_count)
            .saturating_sub(self.source_live);
        if self.live > self.profile.inflight_limit().min(handle_ceiling) {
            return Err(QdnfError::Capacity);
        }
        let charge = self.governor.admit_and_reserve(
            self.ledger,
            ResourceBudget {
                bytes,
                work: 0,
                io: 0,
            },
            WorkClass::Background,
            true,
        )?;
        let owned = match OwnedIntegrityPage::new(identity, page) {
            Ok(p) => p,
            Err(_) => {
                self.ledger.release(charge)?;
                return Err(QdnfError::Range);
            }
        };
        self.sparse_workspaces[index].generation = generation;
        self.sparse_workspaces[index].identity = Some(identity);
        self.sparse_workspaces[index].page = page;
        self.sparse_workspaces[index].charge = Some(charge);
        self.workspace_live = next_count;
        self.used_bytes = next_window;
        Ok(SparsePageWorkspace {
            owner: self.owner,
            slot: index as u8,
            generation,
            store_incarnation: crate::net::qdnf::types::Generation::ZERO,
            page: owned,
        })
    }

    pub const fn retained_sparse_pages(&self) -> usize {
        self.workspace_live
    }

    /// Handle-limited ceiling; other existing host use and byte caps may narrow it.
    pub fn sparse_inflight_ceiling(&self) -> usize {
        self.profile.inflight_limit().min(
            30usize
                .saturating_sub(self.workspace_live)
                .saturating_sub(self.source_live),
        )
    }

    pub(in super::super) fn validate_sparse_workspace(
        &self,
        workspace: &SparsePageWorkspace,
    ) -> Result<usize, QdnfError> {
        self.require_open()?;
        if workspace.owner != self.owner {
            return Err(QdnfError::StaleGeneration);
        }
        let index = usize::from(workspace.slot);
        let slot = self
            .sparse_workspaces
            .get(index)
            .ok_or(QdnfError::StaleGeneration)?;
        if slot.charge.is_none()
            || slot.generation != workspace.generation
            || slot.identity.as_ref() != Some(workspace.identity())
            || slot.page != workspace.page_number()
        {
            return Err(QdnfError::StaleGeneration);
        }
        Ok(index)
    }

    /// Actual page borrow cannot coexist with this consuming release. On every
    /// refusal the original owner is returned intact, including wrong-runtime use.
    pub fn release_sparse_page(
        &mut self,
        workspace: SparsePageWorkspace,
    ) -> Result<(), WorkspaceReleaseError> {
        match self.release_sparse_page_inner(&workspace) {
            Ok(()) => Ok(()),
            Err(error) => Err(WorkspaceReleaseError { error, workspace }),
        }
    }
    fn release_sparse_page_inner(
        &mut self,
        workspace: &SparsePageWorkspace,
    ) -> Result<(), QdnfError> {
        let index = self.validate_sparse_workspace(workspace)?;
        let bytes =
            u64::try_from(SparsePageWorkspace::charged_bytes()).map_err(|_| QdnfError::Range)?;
        let next_window = self
            .used_bytes
            .checked_sub(bytes)
            .ok_or(QdnfError::Conflict)?;
        let next_count = self
            .workspace_live
            .checked_sub(1)
            .ok_or(QdnfError::Conflict)?;
        let charge = self.sparse_workspaces[index]
            .charge
            .take()
            .ok_or(QdnfError::Incomplete)?;
        let original_ref = charge.as_ref();
        // Parent release consumes even a refused token. Restore only the exact
        // original owner-issued reference on its pre-mutation failure; no
        // caller numeric caps/IDs manufacture a reservation.
        if let Err(error) = self.ledger.release_checked_bytes(charge, bytes) {
            self.sparse_workspaces[index].charge = Some(ReservationHandle::from_ref(original_ref));
            return Err(error);
        }
        self.sparse_workspaces[index].identity = None;
        self.sparse_workspaces[index].page = 0;
        self.used_bytes = next_window;
        self.workspace_live = next_count;
        Ok(())
    }
}
