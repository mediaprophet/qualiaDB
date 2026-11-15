//! Inspect a checkpoint using the signed head of a held local lease.
//!
//! The host must maintain root protection, locality, rollback-resistant
//! freshness and a read/policy lease through return. A file lock and two equal
//! snapshots do not independently prove those properties.
use super::{
    chain::{ChainAuthorityError, ChainHead},
    local_head::{LocalHeadError, LocalHeadLease, LocalHeadPolicy},
    recovery::{
        inspect_recovery_bundle, MembershipAssertion, RecoveryAuthority, RecoveryError,
        RecoveryReport,
    },
    CustodyAuthority, SidecarReceipt, SidecarReceiptAuthority, SIDECAR_BYTES,
};
use std::path::Path;

/// Current exact recovery permission and authenticated membership evidence.
/// Missing ancestry/exclusion evidence must return `Unknown`. Implementations
/// must bind their proof to the root, chain, receipt and supplied head.
pub trait LocalRecoveryPolicy: LocalHeadPolicy + CustodyAuthority {
    fn permit_recovery_read(
        &mut self,
        root: &Path,
        chain: &[u8; 32],
        checkpoint: &[u8; 32],
        byte_budget: usize,
    ) -> bool;

    fn verify_recovery_membership(
        &mut self,
        root: &Path,
        chain: &[u8; 32],
        head: ChainHead,
        receipt: &SidecarReceipt,
    ) -> Result<MembershipAssertion, ChainAuthorityError>;
}

/// Use one caller-held lease without opening another head or acquiring another
/// lock. No writes, head changes, scans, repair or deletion are performed.
/// The checkpoint bytes and head are verified through existing custody APIs;
/// the policy supplies authenticated membership and anti-rollback authority.
#[allow(clippy::too_many_arguments)]
pub fn inspect_local_recovery<P: LocalRecoveryPolicy>(
    root: &Path,
    chain: &[u8; 32],
    checkpoint: &[u8; 32],
    byte_budget: usize,
    sidecar: &mut [u8; SIDECAR_BYTES],
    payload: &mut [u8],
    lease: &mut LocalHeadLease<'_>,
    policy: &mut P,
) -> Result<RecoveryReport, RecoveryError> {
    sidecar.fill(0);
    payload.fill(0);
    if !lease.matches_scope(root, chain) {
        return Err(RecoveryError::Authority(ChainAuthorityError::NotAuthorized));
    }
    let mut authority = BoundLocalRecovery {
        lease,
        policy,
        root,
        chain,
        checkpoint,
        byte_budget,
    };
    inspect_recovery_bundle(
        root,
        chain,
        checkpoint,
        byte_budget,
        sidecar,
        payload,
        &mut authority,
    )
}

struct BoundLocalRecovery<'lease, 'key, 'scope, P> {
    lease: &'lease mut LocalHeadLease<'key>,
    policy: &'lease mut P,
    root: &'scope Path,
    chain: &'scope [u8; 32],
    checkpoint: &'scope [u8; 32],
    byte_budget: usize,
}

impl<P: LocalRecoveryPolicy> BoundLocalRecovery<'_, '_, '_, P> {
    fn exact(&self, root: &Path, chain: &[u8; 32], checkpoint: &[u8; 32], budget: usize) -> bool {
        root == self.root
            && chain == self.chain
            && checkpoint == self.checkpoint
            && budget == self.byte_budget
            && self.lease.matches_scope(root, chain)
    }

    fn authorize_head_read(&mut self, root: &Path, chain: &[u8; 32]) -> bool {
        self.exact(root, chain, self.checkpoint, self.byte_budget)
            && self
                .policy
                .permit_recovery_read(root, chain, self.checkpoint, self.byte_budget)
            && self
                .policy
                .permit_read(root, self.checkpoint, self.byte_budget)
    }
}

impl<P: LocalRecoveryPolicy> SidecarReceiptAuthority for BoundLocalRecovery<'_, '_, '_, P> {
    fn permits(
        &self,
        receipt: &SidecarReceipt,
        sidecar: &[u8; SIDECAR_BYTES],
        payload: &[u8],
    ) -> bool {
        receipt.chain_id() == self.chain
            && &receipt.checkpoint() == self.checkpoint
            && self.policy.permits(receipt, sidecar, payload)
    }
}

impl<P: LocalRecoveryPolicy> CustodyAuthority for BoundLocalRecovery<'_, '_, '_, P> {
    fn permit_publish(
        &mut self,
        _: &Path,
        _: &SidecarReceipt,
        _: &[u8; SIDECAR_BYTES],
        _: &[u8],
        _: usize,
    ) -> bool {
        false
    }

    fn permit_read(&mut self, root: &Path, checkpoint: &[u8; 32], byte_budget: usize) -> bool {
        self.exact(root, self.chain, checkpoint, byte_budget)
            && self.policy.permit_read(root, checkpoint, byte_budget)
    }
}

impl<P: LocalRecoveryPolicy> RecoveryAuthority for BoundLocalRecovery<'_, '_, '_, P> {
    fn permit_recovery(
        &mut self,
        root: &Path,
        chain: &[u8; 32],
        checkpoint: &[u8; 32],
        byte_budget: usize,
    ) -> bool {
        self.exact(root, chain, checkpoint, byte_budget)
            && self
                .policy
                .permit_recovery_read(root, chain, checkpoint, byte_budget)
    }

    fn recovery_head(
        &mut self,
        root: &Path,
        chain: &[u8; 32],
    ) -> Result<ChainHead, ChainAuthorityError> {
        if !self.authorize_head_read(root, chain) {
            return Err(ChainAuthorityError::NotAuthorized);
        }
        self.lease.current(self.policy).map_err(map_head_error)
    }

    fn membership(
        &mut self,
        root: &Path,
        head: ChainHead,
        receipt: &SidecarReceipt,
    ) -> Result<MembershipAssertion, ChainAuthorityError> {
        if receipt.chain_id() != self.chain
            || &receipt.checkpoint() != self.checkpoint
            || !self.authorize_head_read(root, receipt.chain_id())
        {
            return Err(ChainAuthorityError::NotAuthorized);
        }
        self.policy
            .verify_recovery_membership(root, self.chain, head, receipt)
    }
}

fn map_head_error(error: LocalHeadError) -> ChainAuthorityError {
    match error {
        LocalHeadError::PermissionDenied => ChainAuthorityError::NotAuthorized,
        LocalHeadError::InvalidEncoding
        | LocalHeadError::InvalidSigner
        | LocalHeadError::InvalidSignature
        | LocalHeadError::InvalidChain => ChainAuthorityError::InvalidState,
        _ => ChainAuthorityError::Unavailable,
    }
}

#[cfg(test)]
#[path = "local_recovery_tests.rs"]
mod tests;
