//! Preflight -> MOVE -> existing engine -> unconditional restore -> unlock.
use super::super::TransferRuntime;
use super::{restore::Restore, RetainedRuntime, RuntimeAccess, RuntimeParents};
use crate::net::qdnf::errors::QdnfError as E;
impl<'host> RuntimeParents<'host> {
    /// One bounded nonblocking poll; all parent locks are released on return.
    /// No guard/borrow may escape. Never wait for peer or synchronous disk flush.
    pub(crate) fn try_with_runtime<T>(
        &self,
        state: &mut RetainedRuntime<'_, '_>,
        effect: impl for<'b, 'p> FnOnce(RuntimeAccess<'b, 'p>) -> Result<T, E>,
    ) -> Result<T, E> {
        self.with_runtime_raw(state, |runtime| {
            effect(RuntimeAccess::new(
                runtime,
                self.ledger,
                self.identities.ledger,
            ))
        })
    }
    pub(super) fn with_runtime_raw<T>(
        &self,
        state: &mut RetainedRuntime<'_, '_>,
        effect: impl for<'p> FnOnce(&mut TransferRuntime<'p>) -> Result<T, E>,
    ) -> Result<T, E> {
        self.association(state)?;
        if state.data.as_ref().ok_or(E::Incomplete)?.closed {
            return Err(E::Closed);
        }
        let mut loans = self.loans()?;
        state
            .data
            .as_ref()
            .ok_or(E::Incomplete)?
            .preflight(&loans)?;
        // ALL fallible affinity/handle checks precede taking the single owner.
        // Catch INSIDE guard lifetime: restore runs on unwind; actual mutex
        // guards then drop normally before we resume the original panic payload.
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let data = state.data.take().ok_or(E::Incomplete)?;
            let runtime = data.attach(
                &mut loans.ledger,
                &mut loans.leases,
                &mut loans.operations,
                &mut loans.scheduler,
                self.governor,
            );
            let mut restore = Restore::new(&mut state.data, runtime);
            RuntimeAccess::new(restore.runtime(), self.ledger, self.identities.ledger)
                .observe_cancellation()?;
            effect(restore.runtime())
        }));
        drop(loans);
        match result {
            Ok(result) => result,
            Err(payload) => std::panic::resume_unwind(payload),
        }
    }
}
impl RetainedRuntime<'_, '_> {
    pub(crate) fn live_transfers(&self) -> Result<usize, E> {
        Ok(self.data.as_ref().ok_or(E::Incomplete)?.live)
    }
    pub(crate) fn pending_schedule_records(&self) -> Result<usize, E> {
        Ok(self.data.as_ref().ok_or(E::Incomplete)?.pending_records)
    }
    pub(crate) fn charged_window_bytes(&self) -> Result<u64, E> {
        Ok(self.data.as_ref().ok_or(E::Incomplete)?.used_bytes)
    }
    /// Logical moved state footprint, not an assertion of generated memcpy count.
    pub(crate) fn move_bytes_per_poll() -> usize {
        2 * core::mem::size_of::<super::RuntimeData>()
    }
}
