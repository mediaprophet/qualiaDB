use super::ContainerCustodySelection;
use crate::{
    local_publish::{prepare_and_publish_local_store_with_quota, LocalStorePolicy},
    publish_store::{PublishStoreError, PublishedSensingBundle, StoreConsent},
    store_receipt::StoreBindingAuthority,
};
use ed25519_dalek::SigningKey;
use qualia_core_db::sensing::{
    authorization::{
        IdentityAuthority, RevocationAuthority, SensingPermit, SensingScopeAuthority,
        TrustedUnixClock,
    },
    local_custody::{local_head::LocalHeadLease, quota::CustodyQuota},
};
use std::path::Path;

/// Publish this exact selected row's payload; callers cannot substitute another
/// byte slice. Current Store policy still binds the signed bytes and container
/// location, and all local-head/quota/uncertainty contracts remain in force.
#[allow(clippy::too_many_arguments)]
pub fn prepare_and_publish_container_store_with_quota<I, S, R, C, P, const N: usize>(
    root: &Path,
    selection: &ContainerCustodySelection<'_, '_>,
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
    prepare_and_publish_local_store_with_quota(
        root,
        selection.observation(),
        selection.selected().payload(),
        payload_budget,
        bundle_budget,
        permit,
        consent,
        binding,
        lease,
        policy,
        quota,
        chain_id,
        sequence,
        previous,
        signer,
    )
}
