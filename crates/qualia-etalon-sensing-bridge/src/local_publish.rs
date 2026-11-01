//! Publish a prepared sensing record using a caller-held signed local head.
//!
//! Host policy must protect the root and lock identity, establish locality and
//! rollback-resistant freshness, and hold its policy lease through replacement.
//! Windows head replacement can return an indeterminate outcome. A denied or
//! uncertain head commit can leave a published bundle requiring recovery.
use crate::{
    publish_store::{
        publish_prepared_store_with_optional_quota, ExactPublicationAuthority, PublishStoreError,
        PublishedSensingBundle, StoreConsent,
    },
    store_receipt::{prepare_store_receipt, StoreBindingAuthority},
    BridgedObservation,
};
use ed25519_dalek::SigningKey;
use qualia_core_db::{
    governance::consent_contract::ConsentGrant,
    sensing::{
        authorization::{
            IdentityAuthority, RevocationAuthority, SensingPermit, SensingRequest,
            SensingScopeAuthority, TrustedUnixClock,
        },
        local_custody::{
            chain::{
                ChainAuthority, ChainAuthorityError, ChainHead, ChainPublicationError, HeadCommit,
            },
            local_head::{LocalHeadError, LocalHeadLease, LocalHeadPolicy},
            quota::CustodyQuota,
            CustodyAuthority,
        },
        sidecar_attestation::{
            SidecarReceipt, SidecarReceiptAuthority, SIDECAR_BYTES, SIDECAR_RECEIPT_BYTES,
        },
    },
};
use std::path::Path;

/// Exact semantic/policy binding in addition to local head and bundle policy.
/// Implementers must enforce subject/guardian relations for this exact grant
/// and maintain a policy lease across all callbacks and the head replacement.
pub trait LocalStorePolicy: LocalHeadPolicy + CustodyAuthority {
    fn permits_exact(
        &mut self,
        grant: &ConsentGrant,
        request: &SensingRequest,
        root: &Path,
        observation: &BridgedObservation,
        receipt: &SidecarReceipt,
        sidecar: &[u8; SIDECAR_BYTES],
        payload: &[u8],
    ) -> bool;
}

/// Prepare once, bind the resulting signed bytes to the local lease, and use
/// the normal publication guard's fresh consent checks. Acquiring the lease is
/// explicit and occurs outside this function under the caller's root policy.
#[allow(clippy::too_many_arguments)]
pub fn prepare_and_publish_local_store<I, S, R, C, P>(
    root: &Path,
    observation: &BridgedObservation,
    payload: &[u8],
    payload_budget: usize,
    bundle_budget: usize,
    permit: SensingPermit,
    consent: &mut StoreConsent<'_, I, S, R, C>,
    binding: &impl StoreBindingAuthority,
    lease: &mut LocalHeadLease<'_>,
    policy: &mut P,
    chain_id: [u8; 32],
    sequence: u64,
    previous: Option<[u8; 32]>,
    signer: &SigningKey,
) -> Result<PublishedSensingBundle, PublishStoreError>
where
    I: IdentityAuthority,
    S: SensingScopeAuthority,
    R: RevocationAuthority,
    C: TrustedUnixClock,
    P: LocalStorePolicy,
{
    prepare_and_publish_local_store_inner::<I, S, R, C, P, 0>(
        root,
        observation,
        payload,
        payload_budget,
        bundle_budget,
        permit,
        consent,
        binding,
        lease,
        policy,
        chain_id,
        sequence,
        previous,
        signer,
        None,
    )
}

/// Opt into conservative aggregate quota admission for this exact local
/// publication. The caller must authenticate the ledger's current-root usage
/// snapshot and hold its aggregate-quota lease for the ledger's whole lifetime,
/// including this operation and recovery. The chain lease alone is insufficient.
///
/// Receipt signing happens once and the core quota wrapper charges once after
/// guarded admission. Every attempted publication remains charged on failure
/// or unwind; there are no refunds or automatic retries. A published bundle
/// with denied/indeterminate head commit remains available in the error for
/// host-controlled recovery. Rebuild a ledger only from fresh authenticated
/// usage including committed, orphan and uncertain attempts. This cooperative
/// ledger is not a durable/cross-process quota or a filesystem allocation cap.
#[allow(clippy::too_many_arguments)]
pub fn prepare_and_publish_local_store_with_quota<I, S, R, C, P, const N: usize>(
    root: &Path,
    observation: &BridgedObservation,
    payload: &[u8],
    payload_budget: usize,
    bundle_budget: usize,
    permit: SensingPermit,
    consent: &mut StoreConsent<'_, I, S, R, C>,
    binding: &impl StoreBindingAuthority,
    lease: &mut LocalHeadLease<'_>,
    policy: &mut P,
    quota: &mut CustodyQuota<'_, N>,
    chain_id: [u8; 32],
    sequence: u64,
    previous: Option<[u8; 32]>,
    signer: &SigningKey,
) -> Result<PublishedSensingBundle, PublishStoreError>
where
    I: IdentityAuthority,
    S: SensingScopeAuthority,
    R: RevocationAuthority,
    C: TrustedUnixClock,
    P: LocalStorePolicy,
{
    prepare_and_publish_local_store_inner(
        root,
        observation,
        payload,
        payload_budget,
        bundle_budget,
        permit,
        consent,
        binding,
        lease,
        policy,
        chain_id,
        sequence,
        previous,
        signer,
        Some(quota),
    )
}

#[allow(clippy::too_many_arguments)]
fn prepare_and_publish_local_store_inner<I, S, R, C, P, const N: usize>(
    root: &Path,
    observation: &BridgedObservation,
    payload: &[u8],
    payload_budget: usize,
    bundle_budget: usize,
    permit: SensingPermit,
    consent: &mut StoreConsent<'_, I, S, R, C>,
    binding: &impl StoreBindingAuthority,
    lease: &mut LocalHeadLease<'_>,
    policy: &mut P,
    chain_id: [u8; 32],
    sequence: u64,
    previous: Option<[u8; 32]>,
    signer: &SigningKey,
    quota: Option<&mut CustodyQuota<'_, N>>,
) -> Result<PublishedSensingBundle, PublishStoreError>
where
    I: IdentityAuthority,
    S: SensingScopeAuthority,
    R: RevocationAuthority,
    C: TrustedUnixClock,
    P: LocalStorePolicy,
{
    if !lease.matches_scope(root, &chain_id) {
        return Err(PublishStoreError::Publication(
            ChainPublicationError::Authority(ChainAuthorityError::NotAuthorized),
        ));
    }
    let request = permit.request();
    let mut sidecar = [0u8; SIDECAR_BYTES];
    let mut encoded_receipt = [0u8; SIDECAR_RECEIPT_BYTES];
    let receipt = prepare_store_receipt(
        binding,
        permit,
        consent.grant,
        consent.identity,
        consent.scope,
        consent.revocations,
        consent.clock,
        observation,
        payload,
        payload_budget,
        chain_id,
        sequence,
        previous,
        signer,
        &mut sidecar,
        &mut encoded_receipt,
    )
    .map_err(PublishStoreError::Prepare)?;
    let mut host = BoundLocalHost {
        lease,
        policy,
        receipt: &receipt,
        sidecar: &sidecar,
        payload,
        payload_budget,
    };
    publish_prepared_store_with_optional_quota(
        root,
        observation,
        payload,
        bundle_budget,
        request,
        consent,
        &mut host,
        &sidecar,
        receipt,
        quota,
    )
}

struct BoundLocalHost<'lease, 'key, 'data, P> {
    lease: &'lease mut LocalHeadLease<'key>,
    policy: &'lease mut P,
    receipt: &'data SidecarReceipt,
    sidecar: &'data [u8; SIDECAR_BYTES],
    payload: &'data [u8],
    payload_budget: usize,
}

impl<P: LocalStorePolicy> BoundLocalHost<'_, '_, '_, P> {
    fn bound(
        &self,
        root: &Path,
        receipt: &SidecarReceipt,
        sidecar: &[u8; SIDECAR_BYTES],
        payload: &[u8],
    ) -> bool {
        self.lease.matches_scope(root, receipt.chain_id())
            && receipt == self.receipt
            && sidecar == self.sidecar
            && payload == self.payload
    }
}

impl<P: LocalStorePolicy> SidecarReceiptAuthority for BoundLocalHost<'_, '_, '_, P> {
    fn permits(
        &self,
        receipt: &SidecarReceipt,
        sidecar: &[u8; SIDECAR_BYTES],
        payload: &[u8],
    ) -> bool {
        receipt == self.receipt
            && sidecar == self.sidecar
            && payload == self.payload
            && self.policy.permits(receipt, sidecar, payload)
    }
}

impl<P: LocalStorePolicy> CustodyAuthority for BoundLocalHost<'_, '_, '_, P> {
    fn permit_publish(
        &mut self,
        root: &Path,
        receipt: &SidecarReceipt,
        sidecar: &[u8; SIDECAR_BYTES],
        payload: &[u8],
        total: usize,
    ) -> bool {
        self.bound(root, receipt, sidecar, payload)
            && self
                .policy
                .permit_publish(root, receipt, sidecar, payload, total)
    }
    fn permit_read(&mut self, _: &Path, _: &[u8; 32], _: usize) -> bool {
        false
    }
}

impl<P: LocalStorePolicy> ChainAuthority for BoundLocalHost<'_, '_, '_, P> {
    fn authenticated_head(
        &mut self,
        root: &Path,
        chain: &[u8; 32],
    ) -> Result<ChainHead, ChainAuthorityError> {
        if !self.lease.matches_scope(root, chain) || chain != self.receipt.chain_id() {
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
        chain: &[u8; 32],
        expected: ChainHead,
        next: ChainHead,
        receipt: &SidecarReceipt,
    ) -> HeadCommit {
        if !self.bound(root, receipt, self.sidecar, self.payload)
            || chain != receipt.chain_id()
            || next
                != (ChainHead::Present {
                    sequence: receipt.sequence(),
                    checkpoint: receipt.checkpoint(),
                })
        {
            return HeadCommit::Conflict;
        }
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

impl<P: LocalStorePolicy> ExactPublicationAuthority for BoundLocalHost<'_, '_, '_, P> {
    fn permits_exact(
        &mut self,
        grant: &ConsentGrant,
        request: &SensingRequest,
        root: &Path,
        observation: &BridgedObservation,
        receipt: &SidecarReceipt,
        sidecar: &[u8; SIDECAR_BYTES],
        payload: &[u8],
    ) -> bool {
        self.bound(root, receipt, sidecar, payload)
            && self.policy.permits_exact(
                grant,
                request,
                root,
                observation,
                receipt,
                sidecar,
                payload,
            )
    }
}
