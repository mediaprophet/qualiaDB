//! Connect a held local-head lease to chain-aware bundle publication.
//!
//! The caller must hold a protected local root and rollback-resistant external
//! state. The host policy qualifies lock, root, filesystem, lease, current
//! permission and durable freshness. This adapter does not establish them.
use super::{
    chain::{ChainAuthority, ChainAuthorityError, ChainHead, HeadCommit},
    local_head::{LocalHeadError, LocalHeadLease, LocalHeadPolicy},
    CustodyAuthority, SidecarReceipt, SidecarReceiptAuthority, SIDECAR_BYTES,
};
use std::path::Path;

/// A publication-only authority bound to one exact receipt and its bytes.
/// Binding does not verify, store, authenticate, or grant access by itself.
pub struct LocalChainAuthority<'lease, 'key, 'data, P> {
    lease: &'lease mut LocalHeadLease<'key>,
    policy: &'lease mut P,
    receipt: &'data SidecarReceipt,
    sidecar: &'data [u8; SIDECAR_BYTES],
    payload: &'data [u8],
    payload_budget: usize,
}

impl<'lease, 'key, 'data, P: LocalHeadPolicy + CustodyAuthority>
    LocalChainAuthority<'lease, 'key, 'data, P>
{
    pub fn new(
        lease: &'lease mut LocalHeadLease<'key>,
        policy: &'lease mut P,
        receipt: &'data SidecarReceipt,
        sidecar: &'data [u8; SIDECAR_BYTES],
        payload: &'data [u8],
        payload_budget: usize,
    ) -> Result<Self, LocalHeadError> {
        if payload.len() > payload_budget {
            return Err(LocalHeadError::InvalidBudget);
        }
        Ok(Self {
            lease,
            policy,
            receipt,
            sidecar,
            payload,
            payload_budget,
        })
    }

    fn bound(
        &self,
        root: &Path,
        receipt: &SidecarReceipt,
        sidecar: &[u8; SIDECAR_BYTES],
        payload: &[u8],
    ) -> bool {
        self.lease.matches_scope(root, receipt.chain_id())
            && self.receipt == receipt
            && self.sidecar == sidecar
            && self.payload == payload
    }
}

impl<P: LocalHeadPolicy + CustodyAuthority> SidecarReceiptAuthority
    for LocalChainAuthority<'_, '_, '_, P>
{
    fn permits(
        &self,
        receipt: &SidecarReceipt,
        sidecar: &[u8; SIDECAR_BYTES],
        payload: &[u8],
    ) -> bool {
        self.receipt == receipt
            && self.sidecar == sidecar
            && self.payload == payload
            && self.policy.permits(receipt, sidecar, payload)
    }
}

impl<P: LocalHeadPolicy + CustodyAuthority> CustodyAuthority
    for LocalChainAuthority<'_, '_, '_, P>
{
    fn permit_publish(
        &mut self,
        root: &Path,
        receipt: &SidecarReceipt,
        sidecar: &[u8; SIDECAR_BYTES],
        payload: &[u8],
        total_bytes: usize,
    ) -> bool {
        self.bound(root, receipt, sidecar, payload)
            && self
                .policy
                .permit_publish(root, receipt, sidecar, payload, total_bytes)
    }

    fn permit_read(&mut self, _: &Path, _: &[u8; 32], _: usize) -> bool {
        false
    }
}

impl<P: LocalHeadPolicy + CustodyAuthority> ChainAuthority for LocalChainAuthority<'_, '_, '_, P> {
    fn authenticated_head(
        &mut self,
        root: &Path,
        chain_id: &[u8; 32],
    ) -> Result<ChainHead, ChainAuthorityError> {
        if !self.lease.matches_scope(root, chain_id) || self.receipt.chain_id() != chain_id {
            return Err(ChainAuthorityError::NotAuthorized);
        }
        self.lease
            .current(self.policy)
            .map_err(|error| match error {
                LocalHeadError::PermissionDenied => ChainAuthorityError::NotAuthorized,
                LocalHeadError::InvalidEncoding
                | LocalHeadError::InvalidSigner
                | LocalHeadError::InvalidSignature
                | LocalHeadError::InvalidChain => ChainAuthorityError::InvalidState,
                _ => ChainAuthorityError::Unavailable,
            })
    }

    fn compare_and_commit(
        &mut self,
        root: &Path,
        chain_id: &[u8; 32],
        expected: ChainHead,
        next: ChainHead,
        receipt: &SidecarReceipt,
    ) -> HeadCommit {
        if !self.lease.matches_scope(root, chain_id)
            || receipt != self.receipt
            || receipt.chain_id() != chain_id
            || next
                != (ChainHead::Present {
                    sequence: receipt.sequence(),
                    checkpoint: receipt.checkpoint(),
                })
        {
            return HeadCommit::Conflict;
        }
        // Errors are conservatively indeterminate: callers reconcile the
        // authenticated durable head and the published bundle before retrying.
        self.lease
            .compare_and_commit(
                expected,
                receipt,
                self.sidecar,
                self.payload,
                self.payload_budget,
                self.policy,
            )
            .unwrap_or(HeadCommit::Indeterminate)
    }
}

#[cfg(test)]
#[path = "local_head_adapter_tests.rs"]
mod tests;
