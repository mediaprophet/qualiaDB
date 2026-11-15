//! Same concrete authority owner, short locks, explicit local adoption only.
use super::{Intent, Role, E};
use crate::net::qdnf::authority::{
    AuthorityBinding, AuthorityOwner, ContactState, ExecutionPermit, MutationHandle,
};
use std::sync::Mutex;
mod current;
pub struct EndpointAuthority<'a> {
    owner: &'a Mutex<AuthorityOwner>,
    handle: MutationHandle,
    binding: AuthorityBinding,
    intent: Intent,
    role: Role,
    ceiling: u64,
}
impl<'a> EndpointAuthority<'a> {
    /// Trusted local admin boundary, NOT remote Share/Sentinel/consent admission.
    pub fn adopt_local(
        owner: &'a Mutex<AuthorityOwner>,
        intent: Intent,
        role: Role,
        now: u64,
        ceiling: u64,
    ) -> Result<Self, E> {
        intent.validate()?;
        if ceiling < super::ENDPOINT_RESERVATION_BYTES {
            return Err(E::Capacity);
        }
        let binding = intent.authority(role);
        let (_, _, handle) = owner
            .lock()
            .map_err(|_| E::Unauthorized)?
            .install_grant_with_ceiling(
                binding,
                now,
                intent.expires_unix,
                ContactState::Active,
                ceiling,
            )?;
        Ok(Self {
            owner,
            handle,
            binding,
            intent,
            role,
            ceiling,
        })
    }
    pub fn revoke(&self) -> Result<(), E> {
        self.owner
            .lock()
            .map_err(|_| E::Unauthorized)?
            .revoke(self.handle)
    }
    pub(super) fn intent(&self) -> Intent {
        self.intent
    }
    pub(super) fn role(&self) -> Role {
        self.role
    }
    /// Host clock must be fresh, side-effect-free and non-reentrant. Never await
    /// or wait for a peer inside effect; this covers one bounded send/activation.
    pub(super) fn current<T>(
        &self,
        clock: &dyn Fn() -> u64,
        effect: impl FnOnce(&ExecutionPermit, u64) -> Result<T, E>,
    ) -> Result<T, E> {
        let owner = self.owner.lock().map_err(|_| E::Unauthorized)?;
        let now = clock();
        let permit = owner.issue_permit(self.handle, self.binding, now, self.ceiling)?;
        effect(&permit, now)
    }
    pub(super) fn after_verified<T>(
        &self,
        clock: &dyn Fn() -> u64,
        verify: impl FnOnce(u64) -> Result<(), E>,
        effect: impl FnOnce(&ExecutionPermit, u64) -> Result<T, E>,
    ) -> Result<T, E> {
        let owner = self.owner.lock().map_err(|_| E::Unauthorized)?;
        let before = clock();
        owner.issue_permit(self.handle, self.binding, before, self.ceiling)?;
        verify(before)?;
        let now = clock();
        let permit = owner.issue_permit(self.handle, self.binding, now, self.ceiling)?;
        effect(&permit, now)
    }
}
impl Drop for EndpointAuthority<'_> {
    fn drop(&mut self) {
        if let Ok(mut owner) = self.owner.lock() {
            if owner.revoke(self.handle).is_ok() {
                let _ = owner.retire_revoked(self.handle);
            }
        }
    }
}
