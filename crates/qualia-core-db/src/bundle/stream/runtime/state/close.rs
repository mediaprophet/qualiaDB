//! Consuming retained-owner close; partial progress is restored for retry.
use super::RetainedRuntime;
use crate::net::qdnf::errors::QdnfError as E;
use core::sync::atomic::Ordering;
#[must_use]
pub(crate) struct RuntimeCloseError<'parent, 'host> {
    pub error: E,
    pub state: RetainedRuntime<'parent, 'host>,
}
impl core::fmt::Debug for RuntimeCloseError<'_, '_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("RuntimeCloseError")
            .field("error", &self.error)
            .finish_non_exhaustive()
    }
}
impl<'parent, 'host> RetainedRuntime<'parent, 'host> {
    pub(crate) fn try_close(mut self) -> Result<(), RuntimeCloseError<'parent, 'host>> {
        let parents = self.parents;
        if let Err(error) = parents.association(&self) {
            return Err(RuntimeCloseError { error, state: self });
        }
        let data = self
            .data
            .as_ref()
            .expect("private retained state invariant");
        let owner = data.owner;
        let already_closed = data.closed;
        if !already_closed {
            if let Err(error) = parents.with_runtime_raw(&mut self, |runtime| runtime.close()) {
                return Err(RuntimeCloseError { error, state: self });
            }
        }
        if parents
            .active
            .compare_exchange(owner, 0, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err(RuntimeCloseError {
                error: E::Conflict,
                state: self,
            });
        }
        Ok(())
    }
}
