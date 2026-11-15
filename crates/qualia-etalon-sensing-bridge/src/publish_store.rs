//! Prepare and publish one signed sensing bundle under fresh Store checks.
//!
//! The host still supplies a protected local root, quota, exclusive lease,
//! authenticated head and atomic head update. This composition does not
//! establish directory durability, confidentiality or rollback protection.
//! The host must hold a policy lease or equivalent transaction across the
//! final checks and the entire head commit. Rechecks alone do not make expiry
//! or revocation atomic with a blocking filesystem or host operation.
use crate::{
    store_receipt::{prepare_store_receipt, StoreBindingAuthority, StoreReceiptError},
    BridgedObservation,
};
use ed25519_dalek::SigningKey;
use qualia_core_db::{
    governance::consent_contract::ConsentGrant,
    sensing::{
        authorization::{
            authorize_sensing, IdentityAuthority, RevocationAuthority, SensingPermit,
            SensingRequest, SensingScopeAuthority, TrustedUnixClock,
        },
        local_custody::{
            chain::{
                publish_chain_bundle, ChainAuthority, ChainAuthorityError, ChainHead,
                ChainPublication, ChainPublicationError, HeadCommit,
            },
            quota::{publish_chain_bundle_with_quota, CustodyQuota, QuotaPublicationError},
            CustodyAuthority, PublishedBundle,
        },
        sidecar_attestation::{
            SidecarReceipt, SidecarReceiptAuthority, SIDECAR_BYTES, SIDECAR_RECEIPT_BYTES,
        },
    },
};
use std::path::Path;

/// A host-owned authorization for the exact publication and its chain update.
/// It must bind source/subject/guardian relationships, payload location, root,
/// policy commitment, chain lease and actual current host policy. The host
/// must maintain its policy lease through the complete head commit.
pub trait ExactPublicationAuthority: ChainAuthority {
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

/// References to the authorities used for a fresh consent check at each
/// publication boundary. The clock and revocation source must be current.
pub struct StoreConsent<'a, I, S, R, C> {
    pub grant: &'a ConsentGrant,
    pub identity: &'a I,
    pub scope: &'a S,
    pub revocations: &'a R,
    pub clock: &'a mut C,
}

#[derive(Debug)]
pub enum PublishStoreError {
    Prepare(StoreReceiptError),
    Publication(ChainPublicationError),
    /// Quota preflight/admission or a charged publication failure. A retained
    /// charge is never refunded here, including for PublishedUncommitted.
    Quota(QuotaPublicationError<ChainPublicationError>),
    /// The bundle was published but the fresh Store check failed before the
    /// head commit. Recovery must reconcile the host's durable chain state.
    CommitAuthorizationDenied {
        bundle: PublishedBundle,
    },
    /// The same denied commit after quota admission. Both the published
    /// bundle and the retained charge require host-controlled reconciliation.
    QuotaCommitAuthorizationDenied {
        checkpoint: [u8; 32],
        bytes: usize,
        bundle: PublishedBundle,
    },
}

pub struct PublishedSensingBundle {
    pub publication: ChainPublication,
    pub receipt: SidecarReceipt,
}

/// The passed permit is consumed while preparing the receipt; fresh consent
/// and host policy are rechecked before publication and head commit. The host
/// must maintain the lease and implement atomic, durable head semantics.
#[allow(clippy::too_many_arguments)]
pub fn prepare_and_publish_store<I, S, R, C, H>(
    root: &Path,
    observation: &BridgedObservation,
    payload: &[u8],
    payload_budget: usize,
    bundle_budget: usize,
    permit: SensingPermit,
    consent: &mut StoreConsent<'_, I, S, R, C>,
    binding: &impl StoreBindingAuthority,
    host: &mut H,
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
    H: ExactPublicationAuthority,
{
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
    publish_prepared_store(
        root,
        observation,
        payload,
        bundle_budget,
        request,
        consent,
        host,
        &sidecar,
        receipt,
    )
}

/// Internal phase so local-head composition can bind the prepared receipt
/// before constructing its publication authority.
#[allow(clippy::too_many_arguments)]
pub(crate) fn publish_prepared_store<I, S, R, C, H>(
    root: &Path,
    observation: &BridgedObservation,
    payload: &[u8],
    bundle_budget: usize,
    request: SensingRequest,
    consent: &mut StoreConsent<'_, I, S, R, C>,
    host: &mut H,
    sidecar: &[u8; SIDECAR_BYTES],
    receipt: SidecarReceipt,
) -> Result<PublishedSensingBundle, PublishStoreError>
where
    I: IdentityAuthority,
    S: SensingScopeAuthority,
    R: RevocationAuthority,
    C: TrustedUnixClock,
    H: ExactPublicationAuthority,
{
    publish_prepared_store_with_optional_quota::<I, S, R, C, H, 0>(
        root,
        observation,
        payload,
        bundle_budget,
        request,
        consent,
        host,
        sidecar,
        receipt,
        None,
    )
}

/// One guarded publication phase shared by quota and no-quota callers. The
/// optional ledger is charged only by the core quota publication wrapper.
#[allow(clippy::too_many_arguments)]
pub(crate) fn publish_prepared_store_with_optional_quota<I, S, R, C, H, const N: usize>(
    root: &Path,
    observation: &BridgedObservation,
    payload: &[u8],
    bundle_budget: usize,
    request: SensingRequest,
    consent: &mut StoreConsent<'_, I, S, R, C>,
    host: &mut H,
    sidecar: &[u8; SIDECAR_BYTES],
    receipt: SidecarReceipt,
    quota: Option<&mut CustodyQuota<'_, N>>,
) -> Result<PublishedSensingBundle, PublishStoreError>
where
    I: IdentityAuthority,
    S: SensingScopeAuthority,
    R: RevocationAuthority,
    C: TrustedUnixClock,
    H: ExactPublicationAuthority,
{
    let mut guard = PublicationGuard {
        root,
        observation,
        sidecar,
        payload,
        receipt,
        request,
        consent,
        host,
        denied_at_commit: false,
    };
    let result = match quota {
        Some(quota) => publish_chain_bundle_with_quota(
            root,
            sidecar,
            payload,
            &receipt,
            bundle_budget,
            quota,
            &mut guard,
        )
        .map_err(PublishStoreError::Quota),
        None => publish_chain_bundle(root, sidecar, payload, &receipt, bundle_budget, &mut guard)
            .map_err(PublishStoreError::Publication),
    };
    match result {
        Ok(publication) => Ok(PublishedSensingBundle {
            publication,
            receipt,
        }),
        Err(PublishStoreError::Publication(ChainPublicationError::PublishedUncommitted {
            bundle,
            ..
        })) if guard.denied_at_commit => {
            Err(PublishStoreError::CommitAuthorizationDenied { bundle })
        }
        Err(PublishStoreError::Quota(QuotaPublicationError::AttemptCharged {
            checkpoint,
            bytes,
            error: ChainPublicationError::PublishedUncommitted { bundle, .. },
        })) if guard.denied_at_commit => Err(PublishStoreError::QuotaCommitAuthorizationDenied {
            checkpoint,
            bytes,
            bundle,
        }),
        Err(error) => Err(error),
    }
}

struct PublicationGuard<'a, 'b, I, S, R, C, H> {
    root: &'a Path,
    observation: &'a BridgedObservation,
    sidecar: &'a [u8; SIDECAR_BYTES],
    payload: &'a [u8],
    receipt: SidecarReceipt,
    request: SensingRequest,
    consent: &'a mut StoreConsent<'b, I, S, R, C>,
    host: &'a mut H,
    denied_at_commit: bool,
}

impl<
        I: IdentityAuthority,
        S: SensingScopeAuthority,
        R: RevocationAuthority,
        C: TrustedUnixClock,
        H: ExactPublicationAuthority,
    > PublicationGuard<'_, '_, I, S, R, C, H>
{
    fn check(&mut self) -> bool {
        if !self.host.permits_exact(
            self.consent.grant,
            &self.request,
            self.root,
            self.observation,
            &self.receipt,
            self.sidecar,
            self.payload,
        ) {
            return false;
        }
        let permit = match authorize_sensing(
            self.consent.grant,
            self.request,
            self.consent.identity,
            self.consent.scope,
            self.consent.revocations,
            self.consent.clock,
        ) {
            Ok(permit) => permit,
            Err(_) => return false,
        };
        permit
            .revalidate(
                self.consent.grant,
                self.consent.identity,
                self.consent.scope,
                self.consent.revocations,
                self.consent.clock,
            )
            .is_ok()
    }
}

impl<
        I: IdentityAuthority,
        S: SensingScopeAuthority,
        R: RevocationAuthority,
        C: TrustedUnixClock,
        H: ExactPublicationAuthority,
    > SidecarReceiptAuthority for PublicationGuard<'_, '_, I, S, R, C, H>
{
    fn permits(
        &self,
        receipt: &SidecarReceipt,
        sidecar: &[u8; SIDECAR_BYTES],
        payload: &[u8],
    ) -> bool {
        receipt == &self.receipt
            && sidecar == self.sidecar
            && payload == self.payload
            && self.host.permits(receipt, sidecar, payload)
    }
}

impl<
        I: IdentityAuthority,
        S: SensingScopeAuthority,
        R: RevocationAuthority,
        C: TrustedUnixClock,
        H: ExactPublicationAuthority,
    > CustodyAuthority for PublicationGuard<'_, '_, I, S, R, C, H>
{
    fn permit_publish(
        &mut self,
        root: &Path,
        receipt: &SidecarReceipt,
        sidecar: &[u8; SIDECAR_BYTES],
        payload: &[u8],
        total: usize,
    ) -> bool {
        root == self.root
            && receipt == &self.receipt
            && sidecar == self.sidecar
            && payload == self.payload
            && self
                .host
                .permit_publish(root, receipt, sidecar, payload, total)
            && self.check()
    }
    fn permit_read(&mut self, _: &Path, _: &[u8; 32], _: usize) -> bool {
        false
    }
}

impl<
        I: IdentityAuthority,
        S: SensingScopeAuthority,
        R: RevocationAuthority,
        C: TrustedUnixClock,
        H: ExactPublicationAuthority,
    > ChainAuthority for PublicationGuard<'_, '_, I, S, R, C, H>
{
    fn authenticated_head(
        &mut self,
        root: &Path,
        chain: &[u8; 32],
    ) -> Result<ChainHead, ChainAuthorityError> {
        if root != self.root || chain != self.receipt.chain_id() {
            return Err(ChainAuthorityError::NotAuthorized);
        }
        self.host.authenticated_head(root, chain)
    }
    fn compare_and_commit(
        &mut self,
        root: &Path,
        chain: &[u8; 32],
        expected: ChainHead,
        next: ChainHead,
        receipt: &SidecarReceipt,
    ) -> HeadCommit {
        if root != self.root
            || chain != self.receipt.chain_id()
            || receipt != &self.receipt
            || !self.check()
        {
            self.denied_at_commit = true;
            return HeadCommit::Indeterminate;
        }
        self.host
            .compare_and_commit(root, chain, expected, next, receipt)
    }
}
