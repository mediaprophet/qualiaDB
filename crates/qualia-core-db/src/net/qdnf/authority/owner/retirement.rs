//! Explicit terminal reclamation; global admission generations never recycle.
use super::{AuthorityOwner, MutationHandle, QdnfError, Record};
impl AuthorityOwner {
    /// Reclaim only this exact already-revoked record after dependent uses drain.
    /// The host must end all uses first. This does not drain sessions or async IO.
    /// A copied ExecutionPermit remains a historical snapshot: consumers MUST
    /// obtain current issuance from the concrete owner before any later effect.
    pub fn retire_revoked(&mut self, handle: MutationHandle) -> Result<(), QdnfError> {
        let record = self.record_mut(handle)?;
        if !record.revoked {
            return Err(QdnfError::Denied);
        }
        *record = Record::EMPTY;
        // next_generation belongs to the owner, not the vacant slot. Preserve it.
        Ok(())
    }
}
#[cfg(test)]
#[path = "retirement_tests.rs"]
mod tests;
