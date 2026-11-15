//! Controller-side issuance of RecipientRecord A and a separately versioned ticket.
//! Signing uses the existing CryptographicLibrary; PinAuthority stays the key owner.
use super::{
    ControllerRecognition, DisclosureAction, IndependentAdmission, PolicyError as E,
    RecognizedController,
};
#[cfg(not(target_arch = "wasm32"))]
use super::{
    check_recipient_against_independent, encode_decision_ticket_into, DecisionTicketFields,
    RecognitionAdmin, RecognitionHandle, DECISION_TICKET_BYTES, DISCLOSURE_DECISION_PURPOSE,
    MAX_DECISION_LEASE_SECS, RECIPIENT_BYTES, RECIPIENT_PURPOSE,
};
#[cfg(not(target_arch = "wasm32"))]
use crate::crypto::network::authority::{AuthoritySource, PinAuthority, PinHandle};
#[cfg(not(target_arch = "wasm32"))]
use crate::crypto::network::cose::{self, Binding, KeyIds};
#[cfg(not(target_arch = "wasm32"))]
use crate::crypto::network::digest::sha384;
use crate::crypto::network::entropy::fill_os;
use crate::crypto::network::errors::CryptoError;
#[cfg(not(target_arch = "wasm32"))]
use crate::crypto::network::key_provider::ProviderGrant;
#[cfg(not(target_arch = "wasm32"))]
use crate::crypto::network::object_stream::{encode_recipient, RecipientFields, RecipientRecord};
use crate::net::qdnf::types::OperationId;
#[cfg(not(target_arch = "wasm32"))]
use crate::net::qdnf::types::StrongDigest;
#[cfg(not(target_arch = "wasm32"))]
use crate::specialized_libs::cryptographic_library::{
    CryptographicLibrary, PairedSigningRequest, SigningAccess, SigningControl,
};
#[cfg(not(target_arch = "wasm32"))]
use zeroize::Zeroizing;

/// Host-authenticated managed signing identity. Not a second KeyManager.
#[cfg(not(target_arch = "wasm32"))]
pub struct ManagedSigners<'a> {
    pub library: &'a CryptographicLibrary,
    pub control: &'a SigningControl,
    pub grant: ProviderGrant,
    pub access: SigningAccess<'a>,
    pub key_ids: KeyIds,
    pub controller_epoch: u64,
    pub ml_key_id: &'a str,
    pub ed_key_id: &'a str,
    pub ml_policy_id: &'a str,
    pub ed_policy_id: &'a str,
}

/// Exact issuance inputs. Challenge is verifier-issued via [`fresh_challenge`].
#[cfg(not(target_arch = "wasm32"))]
pub struct IssueRequest<'a> {
    pub admin: &'a RecognitionAdmin,
    pub handle: RecognitionHandle,
    pub decision_pin: PinHandle,
    pub independent: IndependentAdmission,
    pub recipient: RecipientFields<'a>,
    pub transcript: StrongDigest,
    pub request_id: OperationId,
    pub challenge: [u8; 32],
    pub action: DisclosureAction,
    pub not_before: u64,
    pub expires: u64,
    pub audience: &'a [u8],
    pub signers: ManagedSigners<'a>,
}

/// Caller-owned output. Envelope slots must fit `cose::MAX_ENVELOPE_BYTES`.
#[cfg(not(target_arch = "wasm32"))]
pub struct IssueBuffers<'a> {
    pub recipient: &'a mut [u8],
    pub recipient_envelope: &'a mut [u8],
    pub ticket: &'a mut [u8],
    pub ticket_envelope: &'a mut [u8],
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IssuedLengths {
    pub recipient: usize,
    pub recipient_envelope: usize,
    pub ticket: usize,
    pub ticket_envelope: usize,
}

/// Live table lookup: independent facts must equal this enrollment, not a
/// co-substituted source/policy. Handle snapshot pin/generation must still match.
pub(super) fn join_enrollment(
    recognition: &ControllerRecognition,
    snapshot: &RecognizedController,
    independent: &IndependentAdmission,
    action: DisclosureAction,
    now: u64,
) -> Result<RecognizedController, E> {
    let live = recognition.recognize(
        snapshot.principal(),
        independent.source,
        independent.source_length,
        independent.policy,
        independent.publisher,
        independent.purpose,
        OperationId(independent.operation),
        action,
        now,
    )?;
    if live.pin() != snapshot.pin()
        || live.controller_generation() != snapshot.controller_generation()
        || live.source() != snapshot.source()
        || live.source_length() != snapshot.source_length()
        || live.policy() != snapshot.policy()
    {
        return Err(E::Reference);
    }
    Ok(live)
}

/// OS-CSPRNG challenge. All-zero is refused here and again in the ticket codec.
pub fn fresh_challenge() -> Result<[u8; 32], E> {
    let mut challenge = [0u8; 32];
    fill_os(&mut challenge).map_err(E::Crypto)?;
    if challenge.iter().all(|b| *b == 0) {
        return Err(E::Crypto(CryptoError::EntropyFailure));
    }
    Ok(challenge)
}

/// Issue A and the disclosure ticket from THIS recognition table only.
///
/// `consume_decision_id` advances before signing. A later signing failure does
/// not mint `DisclosureDecision` and does not reuse that id.
#[cfg(not(target_arch = "wasm32"))]
pub fn issue_disclosure(
    recognition: &mut ControllerRecognition,
    pins: &PinAuthority,
    request: IssueRequest<'_>,
    now: u64,
    buffers: IssueBuffers<'_>,
) -> Result<IssuedLengths, E> {
    issue_inner(recognition, pins, request, now, buffers)
}

#[cfg(not(target_arch = "wasm32"))]
fn issue_inner(
    recognition: &mut ControllerRecognition,
    pins: &PinAuthority,
    request: IssueRequest<'_>,
    now: u64,
    buffers: IssueBuffers<'_>,
) -> Result<IssuedLengths, E> {
    let recognized = recognition.snapshot(request.handle)?;
    recognized.current_at(now)?;
    validate_lease(
        &recognized,
        request.action,
        request.not_before,
        request.expires,
    )?;
    if request.challenge.iter().all(|b| *b == 0)
        || request.audience.is_empty()
        || request.signers.controller_epoch == 0
    {
        return Err(E::Reference);
    }

    if buffers.recipient.len() < RECIPIENT_BYTES || buffers.ticket.len() < DECISION_TICKET_BYTES {
        return Err(E::Capacity);
    }
    encode_recipient(request.recipient, buffers.recipient).map_err(E::Crypto)?;
    let record =
        RecipientRecord::parse(&buffers.recipient[..RECIPIENT_BYTES]).map_err(E::Crypto)?;
    check_recipient_against_independent(&record, &request.independent)?;
    join_enrollment(
        recognition,
        &recognized,
        &request.independent,
        request.action,
        now,
    )?;
    record.valid_at(now).map_err(E::Crypto)?;

    let decision_id = recognition.consume_decision_id(request.admin, request.handle)?;
    let ticket_fields = DecisionTicketFields {
        recipient_record_id: record.id(),
        transcript: request.transcript,
        request_id: request.request_id,
        challenge: request.challenge,
        controller_generation: recognized.controller_generation(),
        policy: recognized.policy(),
        action: request.action,
        not_before: request.not_before,
        expires: request.expires,
        decision_id,
    };
    encode_decision_ticket_into(ticket_fields, buffers.ticket)?;

    let recipient_id = record.id();
    let ticket_digest = sha384(&buffers.ticket[..DECISION_TICKET_BYTES]);
    let mut workspace = cose::Workspace::new();
    let rec_n = match sign_with_managed(
        &request.signers,
        Binding {
            descriptor_digest: &recipient_id.0,
            key_ids: request.signers.key_ids,
            controller_epoch: request.signers.controller_epoch,
            purpose: RECIPIENT_PURPOSE,
            audience: request.audience,
        },
        now,
        buffers.recipient_envelope,
        &mut workspace,
    ) {
        Ok(n) => n,
        Err(e) => {
            wipe_unpublished(buffers);
            return Err(e);
        }
    };
    let tkt_n = match sign_with_managed(
        &request.signers,
        Binding {
            descriptor_digest: &ticket_digest.0,
            key_ids: request.signers.key_ids,
            controller_epoch: request.signers.controller_epoch,
            purpose: DISCLOSURE_DECISION_PURPOSE,
            audience: request.audience,
        },
        now,
        buffers.ticket_envelope,
        &mut workspace,
    ) {
        Ok(n) => n,
        Err(e) => {
            wipe_unpublished(buffers);
            return Err(e);
        }
    };
    super::ticket_package_len(DECISION_TICKET_BYTES, tkt_n)?;

    let rec_binding = Binding {
        descriptor_digest: &recipient_id.0,
        key_ids: request.signers.key_ids,
        controller_epoch: request.signers.controller_epoch,
        purpose: RECIPIENT_PURPOSE,
        audience: request.audience,
    };
    let tkt_binding = Binding {
        descriptor_digest: &ticket_digest.0,
        key_ids: request.signers.key_ids,
        controller_epoch: request.signers.controller_epoch,
        purpose: DISCLOSURE_DECISION_PURPOSE,
        audience: request.audience,
    };
    if let Err(e) = pins
        .verify_source(
            AuthoritySource::LocalPin(recognized.pin()),
            &buffers.recipient_envelope[..rec_n],
            &rec_binding,
            now,
            &mut workspace,
        )
        .and_then(|_| {
            pins.verify_source(
                AuthoritySource::LocalPin(request.decision_pin),
                &buffers.ticket_envelope[..tkt_n],
                &tkt_binding,
                now,
                &mut workspace,
            )
        })
        .map_err(E::Crypto)
    {
        wipe_unpublished(buffers);
        return Err(e);
    }

    Ok(IssuedLengths {
        recipient: RECIPIENT_BYTES,
        recipient_envelope: rec_n,
        ticket: DECISION_TICKET_BYTES,
        ticket_envelope: tkt_n,
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn validate_lease(
    recognized: &RecognizedController,
    action: DisclosureAction,
    not_before: u64,
    expires: u64,
) -> Result<(), E> {
    if expires <= not_before {
        return Err(E::Encoding);
    }
    let lease = expires - not_before;
    if lease > MAX_DECISION_LEASE_SECS || lease > recognized.max_decision_lifetime() {
        return Err(E::Limit);
    }
    if expires > recognized.expires_unix() {
        return Err(E::Limit);
    }
    if !recognized.allowed_ops().contains(action) {
        return Err(E::Reference);
    }
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
fn wipe_unpublished(buffers: IssueBuffers<'_>) {
    buffers.recipient_envelope.fill(0);
    buffers.ticket_envelope.fill(0);
    buffers.ticket.fill(0);
}

/// Central managed pair. Scratch is `Zeroizing`; `out` is written only on success.
#[cfg(not(target_arch = "wasm32"))]
pub(super) fn sign_with_managed(
    signers: &ManagedSigners<'_>,
    binding: Binding<'_>,
    now: u64,
    out: &mut [u8],
    workspace: &mut cose::Workspace,
) -> Result<usize, E> {
    let request = PairedSigningRequest {
        ml_key_id: signers.ml_key_id,
        ed_key_id: signers.ed_key_id,
        ml_policy_id: signers.ml_policy_id,
        ed_policy_id: signers.ed_policy_id,
        binding,
        grant: signers.grant,
        access: signers.access,
    };
    let clock = || now;
    let cap = signers
        .library
        .begin_paired_signing(request, signers.control, &clock)
        .map_err(E::Crypto)?;
    let mut scratch = Zeroizing::new([0u8; cose::MAX_ENVELOPE_BYTES]);
    let n = match cose::sign_pair(&binding, cap, workspace, &mut scratch[..]) {
        Ok(n) => n,
        Err(e) => return Err(E::Crypto(e)),
    };
    let dest = out.get_mut(..n).ok_or(E::Capacity)?;
    dest.copy_from_slice(&scratch[..n]);
    Ok(n)
}
