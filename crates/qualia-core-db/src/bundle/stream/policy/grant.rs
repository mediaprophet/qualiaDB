//! Explicit local admin adoption through the existing concrete QDNF grant owner.
mod scoped;
pub use scoped::LocalEffectControl;

use super::{BoundClosure, PolicyError as E};
use crate::crypto::network::{authority::PinAuthority, digest::sha384};
use crate::net::qdnf::{
    authority::{AuthorityBinding, AuthorityOwner, ContactState, MutationHandle},
    types::{OperationId, StrongDigest},
};

/// Trusted local administrative decision, NOT a wire grant or consent proof.
/// The host establishes rights for ALL bytes (header/index/gaps/all entries),
/// exact recipient and purpose through its existing governance owners first.
/// No partial/mixed/unknown coverage profile is supported by this entry point.
pub struct LocalGrantRequest {
    pub principal: StrongDigest,
    pub recipient: StrongDigest,
    pub purpose: StrongDigest,
    pub operation: OperationId,
    pub not_before: u64,
    pub expires: u64,
    pub resource_ceiling: u64,
}
/// Concrete owner borrow is retained from installation through final use. The
/// private handle cannot be rebound to an unrelated similarly populated owner.
/// Holding this guard excludes other grant mutations; hosts must keep it short.
pub struct LocalWholeSourceGrant<'a, 'data> {
    owner: &'a mut AuthorityOwner,
    closure: &'a BoundClosure<'data>,
    handle: MutationHandle,
    binding: AuthorityBinding,
    ceiling: u64,
    spent: bool,
    closed: bool,
}
fn whole_scope(closure: &BoundClosure<'_>) -> StrongDigest {
    let identity = closure.identity();
    let manifest = closure.closure();
    let fields = manifest.manifest().fields();
    let mut frame = [0u8; 512];
    let prefix = b"QBDL-LOCAL-WHOLE-SOURCE-V1\0";
    frame[..prefix.len()].copy_from_slice(prefix);
    let mut n = prefix.len();
    for digest in [
        identity.descriptor_id(),
        identity.context_id(),
        manifest.manifest_digest(),
        fields.source,
        fields.policy,
        fields.consumer,
    ] {
        frame[n..n + 48].copy_from_slice(&digest.0);
        n += 48;
    }
    frame[n..n + 8].copy_from_slice(&fields.source_length.to_be_bytes());
    n += 8;
    frame[n] = 1; // Whole source, including framing and gaps. No partial variant.
    sha384(&frame[..n + 1])
}
/// Trusted admin bootstrap, analogous to AuthorityOwner's existing installer.
/// This is NOT remote Share/Sentinel/consent integration. No policy evaluator or
/// wire parser may call this merely because a manifest signature verified.
pub fn adopt_local_whole_source<'a, 'data>(
    owner: &'a mut AuthorityOwner,
    pins: &PinAuthority,
    closure: &'a BoundClosure<'data>,
    request: LocalGrantRequest,
) -> Result<LocalWholeSourceGrant<'a, 'data>, E> {
    closure.recheck(pins, request.not_before)?;
    let source_length = closure.closure().manifest().fields().source_length;
    if request.resource_ceiling < source_length {
        return Err(E::Limit);
    }
    let binding = AuthorityBinding {
        principal: request.principal,
        recipient: request.recipient,
        purpose: request.purpose,
        operation: request.operation,
        instrument: closure.closure().manifest_digest(),
        scope: whole_scope(closure),
    };
    let (_, _, handle) = owner
        .install_grant_with_ceiling(
            binding,
            request.not_before,
            request.expires,
            ContactState::Active,
            request.resource_ceiling,
        )
        .map_err(E::Authority)?;
    Ok(LocalWholeSourceGrant {
        owner,
        closure,
        handle,
        binding,
        ceiling: request.resource_ceiling,
        spent: false,
        closed: false,
    })
}
impl LocalWholeSourceGrant<'_, '_> {
    /// Host cancellation/revocation is terminal and goes through the SAME owner.
    pub fn cancel(&mut self) -> Result<(), E> {
        self.spent = true;
        self.owner.revoke(self.handle).map_err(E::Authority)
    }
    pub fn binding(&self) -> AuthorityBinding {
        self.binding
    }
    /// Drain this synchronous guard, revoke and explicitly reclaim its record.
    pub fn close(mut self) -> Result<(), E> {
        self.release()
    }
    fn release(&mut self) -> Result<(), E> {
        if self.closed {
            return Ok(());
        }
        self.spent = true;
        self.owner.revoke(self.handle).map_err(E::Authority)?;
        self.owner
            .retire_revoked(self.handle)
            .map_err(E::Authority)?;
        self.closed = true;
        Ok(())
    }
    /// One bounded synchronous host effect. The host clock is trusted and must
    /// be freshly sampled; this is not a wire timestamp. Callback must not await
    /// or retain authority. Failure/unwind is terminal. No ExecutionPermit escapes.
    /// This does not certify payload proofs, resource reservations or durable IO.
    pub fn accept_local<T>(
        &mut self,
        pins: &PinAuthority,
        clock: impl Fn() -> u64,
        accept: impl FnOnce(&BoundClosure<'_>) -> Result<T, E>,
    ) -> Result<T, E> {
        if self.spent {
            return Err(E::Spent);
        }
        self.spent = true;
        let now = clock();
        self.closure.recheck(pins, now)?;
        let _permit = self
            .owner
            .issue_permit(self.handle, self.binding, now, self.ceiling)
            .map_err(E::Authority)?;
        // The second sample closes verification work time; concrete borrows keep
        // both authority owners unchanged through this final check and effect.
        let now = clock();
        self.closure.recheck(pins, now)?;
        self.owner
            .issue_permit(self.handle, self.binding, now, self.ceiling)
            .map_err(E::Authority)?;
        accept(self.closure)
    }
}

impl Drop for LocalWholeSourceGrant<'_, '_> {
    fn drop(&mut self) {
        // Exclusive owner borrow and synchronous callback exclude pending uses.
        let _ = self.release();
    }
}
