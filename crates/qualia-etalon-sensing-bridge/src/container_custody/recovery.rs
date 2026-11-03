//! Container-bound read-only recovery using an existing signed-head lease.
use crate::{
    read_store::StoredReadAuthority,
    sidecar::{decode_sidecar, SidecarError, UntrustedSidecar},
};
use qualia_core_db::sensing::{
    container::ResolvedSensingMeasurement,
    local_custody::{
        local_head::{LocalHeadError, LocalHeadLease},
        local_recovery::{inspect_local_recovery, LocalRecoveryPolicy},
        recovery::{RecoveryError, RecoveryReport},
    },
    sidecar_attestation::{SidecarReceiptError, SIDECAR_BYTES},
};
use std::path::Path;

#[derive(Debug)]
pub enum ContainerRecoveryError {
    PayloadBudget,
    Recovery(RecoveryError),
    Decode(SidecarError),
    RecordMismatch,
    PayloadMismatch,
    NotAuthorized,
    Receipt(SidecarReceiptError),
    Head(LocalHeadError),
    HeadChanged,
}

/// Membership disposition comes from the existing recovery inspector and its
/// host-authenticated head/proofs. Historical metadata is never a current grant.
pub struct RecoveredContainerObservation {
    pub recovery: RecoveryReport,
    pub historical: UntrustedSidecar,
}

/// Inspect one stored checkpoint and bind it to the exact selected container
/// row and same-resolver payload. The host must hold root, chain and policy
/// leases through return. No scan, repair, deletion or head update is performed.
/// Errors and unwind clear the entire caller output; success clears its tail.
#[allow(clippy::too_many_arguments)]
pub fn recover_container_observation<P: LocalRecoveryPolicy + StoredReadAuthority>(
    root: &Path,
    chain: &[u8; 32],
    checkpoint: &[u8; 32],
    bundle_budget: usize,
    payload_budget: usize,
    selected: &ResolvedSensingMeasurement<'_>,
    payload: &mut [u8],
    lease: &mut LocalHeadLease<'_>,
    policy: &mut P,
) -> Result<RecoveredContainerObservation, ContainerRecoveryError> {
    payload.fill(0);
    let mut output = ClearPayload {
        bytes: payload,
        keep: false,
    };
    if selected.payload().len() > payload_budget || selected.payload().len() > output.bytes.len() {
        return Err(ContainerRecoveryError::PayloadBudget);
    }
    let mut sidecar = [0; SIDECAR_BYTES];
    let report = inspect_local_recovery(
        root,
        chain,
        checkpoint,
        bundle_budget,
        &mut sidecar,
        &mut output.bytes[..selected.payload().len()],
        lease,
        policy,
    )
    .map_err(ContainerRecoveryError::Recovery)?;
    let historical =
        decode_sidecar(&sidecar, selected.binding()).map_err(ContainerRecoveryError::Decode)?;
    if *historical.record() != selected.record() {
        return Err(ContainerRecoveryError::RecordMismatch);
    }
    if report.payload_bytes != selected.payload().len()
        || &output.bytes[..report.payload_bytes] != selected.payload()
    {
        return Err(ContainerRecoveryError::PayloadMismatch);
    }
    if !policy.permit_decoded(
        root,
        checkpoint,
        &selected.binding(),
        &report.receipt,
        &historical,
        &sidecar,
        &output.bytes[..report.payload_bytes],
    ) {
        return Err(ContainerRecoveryError::NotAuthorized);
    }
    // Decode/binding policy can take time. Recheck recovery authority before
    // another protected head lookup and retain the inspector's exact snapshot.
    if !policy.permit_recovery_read(root, chain, checkpoint, bundle_budget)
        || !policy.permit_read(root, checkpoint, bundle_budget)
    {
        return Err(ContainerRecoveryError::NotAuthorized);
    }
    if lease
        .current(policy)
        .map_err(ContainerRecoveryError::Head)?
        != report.head
    {
        return Err(ContainerRecoveryError::HeadChanged);
    }
    report
        .receipt
        .verify_with_authority(
            &sidecar,
            &output.bytes[..report.payload_bytes],
            report.payload_bytes,
            policy,
        )
        .map_err(ContainerRecoveryError::Receipt)?;
    if !policy.permit_recovery_read(root, chain, checkpoint, bundle_budget)
        || !policy.permit_read(root, checkpoint, bundle_budget)
    {
        return Err(ContainerRecoveryError::NotAuthorized);
    }
    output.keep = true;
    Ok(RecoveredContainerObservation {
        recovery: report,
        historical,
    })
}

struct ClearPayload<'a> {
    bytes: &'a mut [u8],
    keep: bool,
}
impl Drop for ClearPayload<'_> {
    fn drop(&mut self) {
        if !self.keep {
            self.bytes.fill(0);
        }
    }
}
