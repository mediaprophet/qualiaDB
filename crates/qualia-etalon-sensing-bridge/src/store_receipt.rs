//! Prepare a signed sidecar under a separately revalidated Store authorization.
//!
//! This module does not write to storage. Its outputs must not be treated as a
//! durable or current authorization: the actual store must revalidate policy,
//! bind the resolved payload and enforce its own atomicity and custody rules.
use crate::{
    sidecar::{encode_sidecar, SidecarError, SIDECAR_SIZE},
    BridgedObservation,
};
use ed25519_dalek::SigningKey;
use qualia_core_db::{
    governance::consent_contract::ConsentGrant,
    sensing::{
        authorization::{
            AuthorizationError, IdentityAuthority, RevocationAuthority, SensingAction,
            SensingPermit, SensingRequest, SensingScopeAuthority, TrustedUnixClock,
        },
        sidecar_attestation::{
            SidecarReceipt, SidecarReceiptError, SIDECAR_BYTES, SIDECAR_RECEIPT_BYTES,
        },
    },
};

/// Host authority resolves the full request, data subject and relationships,
/// and binds the returned nonzero commitment to this exact grant, request,
/// canonical sidecar, payload, signing key and chain position. Its decision is
/// not a store; the returned commitment itself is included in the receipt.
pub trait StoreBindingAuthority {
    fn authorize_binding(
        &self,
        grant: &ConsentGrant,
        request: &SensingRequest,
        observation: &BridgedObservation,
        sidecar: &[u8; SIDECAR_BYTES],
        payload: &[u8],
        signer_public_key: &[u8; 32],
        chain_id: &[u8; 32],
        sequence: u64,
        previous: Option<&[u8; 32]>,
    ) -> Option<[u8; 32]>;
}

#[derive(Debug, PartialEq, Eq)]
pub enum StoreReceiptError {
    OutputTooSmall,
    PayloadBudgetExceeded,
    ScopeMismatch,
    BindingDenied,
    Consent(AuthorizationError),
    Sidecar(SidecarError),
    Receipt(SidecarReceiptError),
}

/// Prepare a sidecar and signed receipt in caller buffers. A failed operation
/// leaves both outputs unchanged; extra output capacity is also unchanged.
/// Revalidation is after host binding and immediately before signing. The
/// permit is consumed even if revalidation fails. No bytes are persisted, and
/// the fresh decision has no continuing validity after this function returns.
#[allow(clippy::too_many_arguments)]
pub fn prepare_store_receipt(
    authority: &impl StoreBindingAuthority,
    permit: SensingPermit,
    grant: &ConsentGrant,
    identity: &impl IdentityAuthority,
    sensor_scope: &impl SensingScopeAuthority,
    revocations: &impl RevocationAuthority,
    clock: &mut impl TrustedUnixClock,
    observation: &BridgedObservation,
    payload: &[u8],
    payload_byte_budget: usize,
    chain_id: [u8; 32],
    sequence: u64,
    previous: Option<[u8; 32]>,
    signer: &SigningKey,
    sidecar_out: &mut [u8],
    receipt_out: &mut [u8],
) -> Result<SidecarReceipt, StoreReceiptError> {
    if sidecar_out.len() < SIDECAR_BYTES || receipt_out.len() < SIDECAR_RECEIPT_BYTES {
        return Err(StoreReceiptError::OutputTooSmall);
    }
    if payload.len() > payload_byte_budget {
        return Err(StoreReceiptError::PayloadBudgetExceeded);
    }
    if SIDECAR_SIZE != SIDECAR_BYTES {
        return Err(StoreReceiptError::ScopeMismatch);
    }
    let request = permit.request();
    if request.action() != SensingAction::Store
        || observation.record().source_id().value() != Some(request.source_id())
        || u64::try_from(payload.len()).ok() != Some(observation.record().raw_payload().length())
    {
        return Err(StoreReceiptError::ScopeMismatch);
    }
    let mut sidecar = [0u8; SIDECAR_BYTES];
    let written = encode_sidecar(observation, &mut sidecar).map_err(StoreReceiptError::Sidecar)?;
    if written != SIDECAR_BYTES {
        return Err(StoreReceiptError::ScopeMismatch);
    }
    let signer_public_key = signer.verifying_key().to_bytes();
    let commitment = authority
        .authorize_binding(
            grant,
            &request,
            observation,
            &sidecar,
            payload,
            &signer_public_key,
            &chain_id,
            sequence,
            previous.as_ref(),
        )
        .filter(|value| value.iter().any(|byte| *byte != 0))
        .ok_or(StoreReceiptError::BindingDenied)?;
    let _fresh_use = permit
        .revalidate(grant, identity, sensor_scope, revocations, clock)
        .map_err(StoreReceiptError::Consent)?;
    let receipt = SidecarReceipt::sign(
        &sidecar,
        payload,
        payload_byte_budget,
        commitment,
        chain_id,
        sequence,
        previous,
        signer,
    )
    .map_err(StoreReceiptError::Receipt)?;
    let mut encoded_receipt = [0u8; SIDECAR_RECEIPT_BYTES];
    receipt
        .encode(&mut encoded_receipt)
        .map_err(StoreReceiptError::Receipt)?;
    sidecar_out[..SIDECAR_BYTES].copy_from_slice(&sidecar);
    receipt_out[..SIDECAR_RECEIPT_BYTES].copy_from_slice(&encoded_receipt);
    Ok(receipt)
}
