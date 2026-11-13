//! Endpoint checks through the same guard used by application authority.
use super::EndpointAuthority;
use crate::net::qdnf::{authority::CurrentAuthorityGuard, errors::QdnfError as E};
impl<'a> EndpointAuthority<'a> {
    pub(in super::super) fn current_guard(&self) -> Result<CurrentAuthorityGuard<'a>, E> {
        CurrentAuthorityGuard::try_lock(self.owner)
    }
    pub(in super::super) fn check_guard(
        &self,
        current: &CurrentAuthorityGuard<'_>,
        now: u64,
    ) -> Result<(), E> {
        current.check_handle(self.owner, self.handle, self.binding, now, self.ceiling)
    }
    pub(in super::super) fn revoke_guard(
        &self,
        current: &mut CurrentAuthorityGuard<'_>,
    ) -> Result<(), E> {
        current.revoke_handle(self.owner, self.handle)
    }
}
