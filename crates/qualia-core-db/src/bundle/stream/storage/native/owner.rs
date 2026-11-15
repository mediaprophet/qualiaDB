use super::{SelectedStoreBinding, StoreError as E, StoreLayout};
use crate::bundle::stream::runtime::{sparse_workspace::SparseWorkspaceKey, SparsePageWorkspace};
use crate::bundle::stream::storage::StorageIdentity;
use crate::storage_driver::ranges::{CleanupObservation, CleanupPending, RangeSession};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StoreState {
    Active,
    Poisoned,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LeafBarrierObservation {
    pub identity: StorageIdentity,
    pub leaf_index: u64,
    pub sequence: u64,
    pub backend_attempt: [u8; 32],
}
/// Live-session integrity candidates and actual file-barrier observations.
/// No serving permission, consumer commit or directory durability is granted.
pub struct NativeSparseStore<'driver, 'root> {
    pub(super) session: RangeSession<'driver, 'root>,
    pub(super) binding: SelectedStoreBinding,
    pub(super) layout: StoreLayout,
    pub(super) workspace_key: SparseWorkspaceKey,
    pub(super) page: u64,
    pub(super) sequence: u64,
    pub(super) state: StoreState,
    pub(super) backend_attempt: [u8; 32],
}
impl<'driver, 'root> NativeSparseStore<'driver, 'root> {
    /// A new facade resets its exact page and advances the private store
    /// incarnation; all previously constructed facade keys then refuse use.
    pub fn new(
        session: RangeSession<'driver, 'root>,
        binding: SelectedStoreBinding,
        workspace: &mut SparsePageWorkspace,
    ) -> Result<Self, E> {
        let layout = StoreLayout::for_binding(&binding)?;
        if session.limits() != layout.range_limits() {
            return Err(E::Bounds);
        }
        if workspace.identity() != binding.identity() {
            return Err(E::IdentityMismatch);
        }
        if workspace.page_number() >= layout.page_count() {
            return Err(E::PageMismatch);
        }
        let page = workspace.page_number();
        let workspace_key = workspace.reset_for_store()?;
        let backend_attempt = session.attempt_id();
        Ok(Self {
            session,
            binding,
            layout,
            workspace_key,
            page,
            sequence: 0,
            state: StoreState::Active,
            backend_attempt,
        })
    }
    pub fn identity(&self) -> &StorageIdentity {
        self.binding.identity()
    }
    pub const fn state(&self) -> StoreState {
        self.state
    }
    pub const fn page_number(&self) -> u64 {
        self.page
    }
    pub const fn layout(&self) -> StoreLayout {
        self.layout
    }
    pub(super) fn check_workspace(&self, workspace: &SparsePageWorkspace) -> Result<(), E> {
        if self.state != StoreState::Active {
            return Err(E::Poisoned);
        }
        workspace.check_binding_key(self.workspace_key)?;
        if workspace.identity() != self.identity() {
            return Err(E::IdentityMismatch);
        }
        if workspace.page_number() != self.page {
            return Err(E::PageMismatch);
        }
        Ok(())
    }
    pub(super) fn check_leaf(&self, leaf: u64) -> Result<(), E> {
        self.identity().tree_spec().logical_leaf_length(leaf)?;
        if leaf / crate::bundle::stream::storage::LEAVES_PER_PAGE != self.page {
            return Err(E::PageMismatch);
        }
        Ok(())
    }
    /// Transfers the same live backend only. Constructing another facade will
    /// clear/rebind its workspace; this is not crash reopening or durable promotion.
    pub fn into_staging(self) -> RangeSession<'driver, 'root> {
        self.session
    }
    pub fn close(self) -> Result<CleanupObservation, CleanupPending<'root>> {
        self.session.close()
    }
}
