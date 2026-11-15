//! Verifier-side install of a derived current grant. Distinct owner from the issuer.
use super::decision::DisclosureDecision;
use super::{
    check_recipient_against_independent, decode_decision_ticket, IndependentAdmission,
    PolicyError as E, RecognizedController, DECISION_TICKET_BYTES, DISCLOSURE_DECISION_PURPOSE,
    RECIPIENT_PURPOSE,
};
use crate::crypto::network::authority::{
    AuthorityReceipt, AuthoritySource, PinAuthority, PinHandle,
};
use crate::crypto::network::cose::{self, Binding};
use crate::crypto::network::digest::sha384;
use crate::crypto::network::entropy::fill_os;
use crate::crypto::network::errors::CryptoError;
use crate::crypto::network::object_stream::RecipientRecord;
use crate::net::peer::runtime::incarnation::CanonicalIncarnation;
use crate::net::qdnf::authority::{
    AuthorityBinding, AuthorityOwner, ContactState, CurrentAuthorityGuard, MutationHandle,
};
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::{OperationId, StrongDigest};
use std::sync::{Mutex, TryLockError};

/// Capacity-1 outstanding challenge. Not a serialized ticket; restart needs a new slot.
pub struct VerifierChallengeSlot {
    owner: usize,
    incarnation: CanonicalIncarnation,
    transcript: StrongDigest,
    request_id: OperationId,
    challenge: [u8; 32],
    state: SlotState,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SlotState {
    Empty,
    Outstanding,
    Consumed,
}

impl VerifierChallengeSlot {
    pub const fn new() -> Self {
        Self {
            owner: 0,
            incarnation: CanonicalIncarnation::UNASSIGNED,
            transcript: StrongDigest::ZERO,
            request_id: OperationId::ZERO,
            challenge: [0; 32],
            state: SlotState::Empty,
        }
    }

    /// One live challenge tied to this owner mutex, transcript and request.
    pub fn issue(
        &mut self,
        owner: &Mutex<AuthorityOwner>,
        transcript: StrongDigest,
        request_id: OperationId,
    ) -> Result<[u8; 32], E> {
        if self.state == SlotState::Outstanding {
            return Err(E::Capacity);
        }
        if transcript.is_zero() || request_id.is_zero() {
            return Err(E::Reference);
        }
        let incarnation = snapshot_incarnation(owner)?;
        let mut challenge = [0u8; 32];
        fill_os(&mut challenge).map_err(E::Crypto)?;
        if challenge.iter().all(|b| *b == 0) {
            return Err(E::Crypto(CryptoError::EntropyFailure));
        }
        self.owner = owner as *const Mutex<AuthorityOwner> as usize;
        self.incarnation = incarnation;
        self.transcript = transcript;
        self.request_id = request_id;
        self.challenge = challenge;
        self.state = SlotState::Outstanding;
        Ok(challenge)
    }

    fn consume(
        &mut self,
        owner: &Mutex<AuthorityOwner>,
        transcript: StrongDigest,
        request_id: OperationId,
        challenge: [u8; 32],
    ) -> Result<(), E> {
        match self.state {
            SlotState::Empty => return Err(E::Crypto(CryptoError::Unauthorized)),
            SlotState::Consumed => return Err(E::Authority(QdnfError::Replay)),
            SlotState::Outstanding => {}
        }
        let incarnation = snapshot_incarnation(owner)?;
        if self.owner != owner as *const Mutex<AuthorityOwner> as usize
            || self.transcript != transcript
            || self.request_id != request_id
            || self.challenge != challenge
        {
            return Err(E::Authority(QdnfError::Replay));
        }
        if self.incarnation != incarnation {
            return Err(E::Authority(QdnfError::StaleGeneration));
        }
        self.state = SlotState::Consumed;
        Ok(())
    }
}

impl Default for VerifierChallengeSlot {
    fn default() -> Self {
        Self::new()
    }
}

fn snapshot_incarnation(owner: &Mutex<AuthorityOwner>) -> Result<CanonicalIncarnation, E> {
    let inner = owner.try_lock().map_err(lock_err)?;
    Ok(inner.existing_incarnation())
}

fn lock_err(e: TryLockError<std::sync::MutexGuard<'_, AuthorityOwner>>) -> E {
    match e {
        TryLockError::WouldBlock => E::Authority(QdnfError::WouldBlock),
        TryLockError::Poisoned(_) => E::Authority(QdnfError::Unauthorized),
    }
}

/// Inputs for verifier install. Controller library/pins/owner must be a different mutex.
/// `'owner` is the only lifetime retained by [`RemoteDisclosureGrant`].
pub struct InstallRequest<'owner, 'tmp> {
    pub owner: &'owner Mutex<AuthorityOwner>,
    pub pins: &'tmp PinAuthority,
    pub recognition: &'tmp super::ControllerRecognition,
    pub recognized: RecognizedController,
    pub decision_pin: PinHandle,
    pub independent: IndependentAdmission,
    pub recipient_bytes: &'tmp [u8],
    pub recipient_envelope: &'tmp [u8],
    pub ticket_bytes: &'tmp [u8],
    pub ticket_envelope: &'tmp [u8],
    pub audience: &'tmp [u8],
    pub now: u64,
    pub ceiling: u64,
}

/// Private current grant. No public constructor and no `ExecutionPermit` escape.
pub struct RemoteDisclosureGrant<'a> {
    pub(super) owner: &'a Mutex<AuthorityOwner>,
    pub(super) handle: MutationHandle,
    pub(super) binding: AuthorityBinding,
    pub(super) ceiling: u64,
    pub(super) decision: DisclosureDecision,
    pub(super) recognized: RecognizedController,
    pub(super) receipt_a: AuthorityReceipt,
    pub(super) receipt_ticket: AuthorityReceipt,
    pub(super) descriptor_a: [u8; 48],
    pub(super) descriptor_ticket: [u8; 48],
    pub(super) revoked: bool,
    pub(super) closed: bool,
}

impl<'a> RemoteDisclosureGrant<'a> {
    pub fn receipt_a(&self) -> &AuthorityReceipt {
        &self.receipt_a
    }
    pub fn receipt_ticket(&self) -> &AuthorityReceipt {
        &self.receipt_ticket
    }
    pub fn decision(&self) -> &DisclosureDecision {
        &self.decision
    }
    pub fn binding(&self) -> AuthorityBinding {
        self.binding
    }
    pub fn recognized(&self) -> RecognizedController {
        self.recognized
    }
}

/// Verify A + ticket against independent facts, consume the challenge, install the grant.
pub fn install_remote_disclosure<'owner, 'tmp>(
    request: InstallRequest<'owner, 'tmp>,
    challenge: &mut VerifierChallengeSlot,
    workspace: &mut cose::Workspace,
) -> Result<RemoteDisclosureGrant<'owner>, E> {
    if request.ceiling == 0 {
        return Err(E::Limit);
    }
    request.recognized.current_at(request.now)?;
    let record = RecipientRecord::parse(request.recipient_bytes).map_err(E::Crypto)?;
    check_recipient_against_independent(&record, &request.independent)?;
    record.valid_at(request.now).map_err(E::Crypto)?;
    if request.ticket_bytes.len() != DECISION_TICKET_BYTES {
        return Err(E::Encoding);
    }
    let ticket = decode_decision_ticket(request.ticket_bytes)?;
    let live = super::issuer::join_enrollment(
        request.recognition,
        &request.recognized,
        &request.independent,
        ticket.action,
        request.now,
    )?;

    let rec_id = record.id();
    let rec_binding = expected_binding(
        request.recipient_envelope,
        &rec_id.0,
        RECIPIENT_PURPOSE,
        request.audience,
    )?;
    let receipt_a = request
        .pins
        .verify_source(
            AuthoritySource::LocalPin(request.recognized.pin()),
            request.recipient_envelope,
            &rec_binding,
            request.now,
            workspace,
        )
        .map_err(E::Crypto)?;
    request
        .pins
        .recheck(&receipt_a, &rec_id.0, request.now)
        .map_err(E::Crypto)?;
    if receipt_a.principal() != request.recognized.principal() {
        return Err(E::Reference);
    }

    if ticket.recipient_record_id != rec_id {
        return Err(E::Reference);
    }
    let ticket_digest = sha384(request.ticket_bytes);
    let tkt_binding = expected_binding(
        request.ticket_envelope,
        &ticket_digest.0,
        DISCLOSURE_DECISION_PURPOSE,
        request.audience,
    )?;
    let receipt_ticket = request
        .pins
        .verify_source(
            AuthoritySource::LocalPin(request.decision_pin),
            request.ticket_envelope,
            &tkt_binding,
            request.now,
            workspace,
        )
        .map_err(E::Crypto)?;
    request
        .pins
        .recheck(&receipt_ticket, &ticket_digest.0, request.now)
        .map_err(E::Crypto)?;
    if receipt_ticket.principal() != request.recognized.principal() {
        return Err(E::Reference);
    }

    let decision = DisclosureDecision::bind(&live, &ticket, request.now)?;
    challenge.consume(
        request.owner,
        ticket.transcript,
        ticket.request_id,
        ticket.challenge,
    )?;

    let binding = AuthorityBinding {
        principal: request.recognized.principal(),
        recipient: request.independent.recipient,
        instrument: request.independent.instrument,
        scope: request.independent.scope,
        purpose: request.independent.purpose,
        operation: OperationId(request.independent.operation),
    };
    if binding.principal == binding.recipient {
        return Err(E::Authority(QdnfError::Conflict));
    }

    let handle = install_under_current(
        request.owner,
        binding,
        request.now,
        decision.expires(),
        request.ceiling,
    )?;

    Ok(RemoteDisclosureGrant {
        owner: request.owner,
        handle,
        binding,
        ceiling: request.ceiling,
        decision,
        recognized: live,
        receipt_a,
        receipt_ticket,
        descriptor_a: rec_id.0,
        descriptor_ticket: ticket_digest.0,
        revoked: false,
        closed: false,
    })
}

/// Unverified envelope syntax supplies key ids/epoch. Purpose, audience and
/// descriptor are independently required; `verify_source` still matches the pin.
fn expected_binding<'a>(
    envelope: &'a [u8],
    descriptor: &[u8; 48],
    purpose: &[u8],
    audience: &[u8],
) -> Result<Binding<'a>, E> {
    let parsed = cose::decode_pair(envelope).map_err(E::Crypto)?;
    let binding = *parsed.binding();
    if binding.descriptor_digest != descriptor
        || binding.purpose != purpose
        || binding.audience != audience
    {
        return Err(E::Crypto(CryptoError::Unauthorized));
    }
    Ok(binding)
}

fn install_under_current(
    owner: &Mutex<AuthorityOwner>,
    binding: AuthorityBinding,
    now: u64,
    expires: u64,
    ceiling: u64,
) -> Result<MutationHandle, E> {
    // CurrentAuthorityGuard does not expose install_grant_with_ceiling. Prove the
    // mutex is free, drop the guard, then install through the owner. That gap is
    // recorded in the R2 summary; we do not invent a guard method here.
    {
        let _guard = CurrentAuthorityGuard::try_lock(owner).map_err(E::Authority)?;
    }
    let mut inner = owner.try_lock().map_err(lock_err)?;
    let (_, _, handle) = inner
        .install_grant_with_ceiling(binding, now, expires, ContactState::Active, ceiling)
        .map_err(E::Authority)?;
    Ok(handle)
}
