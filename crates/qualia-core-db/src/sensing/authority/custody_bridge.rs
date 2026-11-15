//! Cryptographic and consent custody bridge for EMF v4 and QAC v1 sensing streams.
//!
//! Bridges `sensing::acquisition` (Schemas 6/7 `AcquisitionView` and `AcquisitionDerivation`,
//! original byte preservation mode `0x0104`), `sensing::authorization` (`SensingRequest`,
//! `SensingModality`, `SensingAction`, `IdentityAuthority`, `SensingScopeAuthority`,
//! `RevocationAuthority`, `TrustedUnixClock`), `sensing::sidecar_attestation`, and
//! `sensing::local_custody` (on non-`wasm32` targets).
//!
//! Enforces anti-substitution binding across `authority_receipt_digest` (Schema 4 key 6,
//! Schema 7 key 8, Schema 137 key 4, Schema 207 key 5), `original_blake3`, envelope
//! `index_commitment`, `source_id`, `actor_id`, `purpose_hash`, and `SensingAction`.

use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};

use crate::governance::consent_contract::ConsentGrant;
use crate::sensing::acquisition::{self, Derivation, Original};
use crate::sensing::authorization::{
    authorize_sensing, AuthorizationError, IdentityAuthority, RevalidatedSensingUse,
    RevocationAuthority, SensingAction, SensingModality, SensingPermit, SensingRequest,
    SensingScopeAuthority, TrustedUnixClock,
};
use crate::sensing::sidecar_attestation::{
    SidecarReceipt, SidecarReceiptAuthority, SidecarReceiptError, SIDECAR_BYTES,
};

#[cfg(not(target_arch = "wasm32"))]
use crate::sensing::local_custody::{
    publish_bundle, read_bundle, CustodyAuthority, CustodyError, PublishedBundle, ReadBundle,
};
#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;

/// Domain separator for BLAKE3 authority receipt digests stored in Schema 4/7/137/207.
pub const AUTHORITY_RECEIPT_DOMAIN: &[u8] = b"QUALIA-SENSING-AUTHORITY-RECEIPT-v1\0";

/// Domain separator for Ed25519 signing over an authority receipt binding.
pub const AUTHORITY_SIGNATURE_DOMAIN: &[u8] = b"QUALIA-SENSING-AUTHORITY-SIG-v1\0";

/// Maximum receipts tracked in a bounded zero-heap anti-replay window.
pub const MAX_RECEIPT_REPLAY_WINDOW: usize = 64;

/// Schema target where an `authority_receipt_digest` is anchored.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u16)]
pub enum AuthorityReceiptTarget {
    /// Schema 4 key 6 (`Provenance.authority_receipt_digest`).
    ProvenanceSchema4 = 4,
    /// Schema 7 key 8 (`AcquisitionDerivation.authority_receipt`).
    AcquisitionDerivationSchema7 = 7,
    /// Schema 137 key 4 (`BioContext.consent_provenance_ref` -> Schema 4 key 6).
    BioContextSchema137 = 137,
    /// Schema 207 key 5 (`Haptic.authority_receipt_digest`).
    HapticSchema207 = 207,
}

impl AuthorityReceiptTarget {
    pub const fn schema_id(self) -> u16 {
        self as u16
    }
}

/// Errors returned by the cryptographic and consent custody bridge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CustodyBridgeError {
    Authorization(AuthorizationError),
    InvalidIdentifier,
    InvalidTimeWindow,
    InvalidSignature,
    InvalidSigner,
    ReceiptDigestMismatch,
    OriginalDigestMismatch,
    IndexCommitmentMismatch,
    SourceIdMismatch,
    ActorIdMismatch,
    PurposeHashMismatch,
    ModalityMismatch,
    ActionEscalation {
        granted: SensingAction,
        requested: SensingAction,
    },
    TargetSchemaMismatch {
        expected: AuthorityReceiptTarget,
        actual: AuthorityReceiptTarget,
    },
    ReplayDetected,
    ReplayWindowFull,
    DerivationInputMismatch,
    SidecarReceipt(SidecarReceiptError),
}

impl From<AuthorizationError> for CustodyBridgeError {
    fn from(err: AuthorizationError) -> Self {
        Self::Authorization(err)
    }
}

impl From<SidecarReceiptError> for CustodyBridgeError {
    fn from(err: SidecarReceiptError) -> Self {
        Self::SidecarReceipt(err)
    }
}

/// Unsigned descriptor of all fields committed into a stream's `authority_receipt_digest`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AuthorityBindingClaims {
    pub target_schema: AuthorityReceiptTarget,
    pub grant_id: [u8; 32],
    pub grant_digest: [u8; 32],
    pub actor_id: [u8; 32],
    pub source_id: u64,
    pub modality: SensingModality,
    pub action: SensingAction,
    pub purpose_hash: u64,
    pub original_blake3: [u8; 32],
    pub index_commitment: [u8; 32],
    pub issued_at_unix: u64,
    pub expires_at_unix: u64,
    pub receipt_nonce: u64,
}

impl AuthorityBindingClaims {
    pub fn validate(&self) -> Result<(), CustodyBridgeError> {
        if self.grant_id == [0u8; 32]
            || self.grant_digest == [0u8; 32]
            || self.actor_id == [0u8; 32]
            || self.source_id == 0
            || self.purpose_hash == 0
            || self.original_blake3 == [0u8; 32]
            || self.index_commitment == [0u8; 32]
            || self.receipt_nonce == 0
        {
            return Err(CustodyBridgeError::InvalidIdentifier);
        }
        if self.issued_at_unix >= self.expires_at_unix {
            return Err(CustodyBridgeError::InvalidTimeWindow);
        }
        Ok(())
    }
}

/// Signed, anti-substitution authority receipt whose BLAKE3 digest populates
/// `authority_receipt_digest` in Schema 4, Schema 7, Schema 137, or Schema 207.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SignedAuthorityReceipt {
    claims: AuthorityBindingClaims,
    signer_public_key: [u8; 32],
    signature: [u8; 64],
}

impl SignedAuthorityReceipt {
    /// Mint a signed authority receipt from a freshly revalidated sensing use,
    /// binding it to the exact `original_blake3` payload digest and envelope
    /// `index_commitment`.
    pub fn issue(
        revalidated: &RevalidatedSensingUse,
        grant_expires_at_unix: u64,
        target_schema: AuthorityReceiptTarget,
        original_blake3: [u8; 32],
        index_commitment: [u8; 32],
        receipt_nonce: u64,
        signing_key: &SigningKey,
    ) -> Result<Self, CustodyBridgeError> {
        let req = revalidated.request();
        let claims = AuthorityBindingClaims {
            target_schema,
            grant_id: revalidated.grant_id(),
            grant_digest: revalidated.grant_digest(),
            actor_id: req.actor_id(),
            source_id: req.source_id(),
            modality: req.modality(),
            action: req.action(),
            purpose_hash: req.purpose_hash(),
            original_blake3,
            index_commitment,
            issued_at_unix: revalidated.checked_at_unix(),
            expires_at_unix: grant_expires_at_unix,
            receipt_nonce,
        };
        claims.validate()?;
        let signer_public_key = signing_key.verifying_key().to_bytes();
        let signing_bytes = Self::signing_preimage(&claims, &signer_public_key);
        let signature = signing_key.sign(&signing_bytes).to_bytes();
        Ok(Self {
            claims,
            signer_public_key,
            signature,
        })
    }

    pub const fn claims(&self) -> &AuthorityBindingClaims {
        &self.claims
    }

    pub const fn signer_public_key(&self) -> &[u8; 32] {
        &self.signer_public_key
    }

    pub const fn signature(&self) -> &[u8; 64] {
        &self.signature
    }

    /// Compute the 32-byte BLAKE3 `authority_receipt_digest` stored in Schema 4/7/137/207.
    pub fn receipt_digest(&self) -> [u8; 32] {
        let mut hasher = blake3::Hasher::new();
        hasher.update(AUTHORITY_RECEIPT_DOMAIN);
        Self::hash_claims(&mut hasher, &self.claims);
        hasher.update(&self.signer_public_key);
        hasher.update(&self.signature);
        *hasher.finalize().as_bytes()
    }

    /// Verify cryptographic signature and structural claim invariants.
    pub fn verify_signature(&self) -> Result<(), CustodyBridgeError> {
        self.claims.validate()?;
        let vk = VerifyingKey::from_bytes(&self.signer_public_key)
            .map_err(|_| CustodyBridgeError::InvalidSigner)?;
        let preimage = Self::signing_preimage(&self.claims, &self.signer_public_key);
        let sig = Signature::from_bytes(&self.signature);
        vk.verify_strict(&preimage, &sig)
            .map_err(|_| CustodyBridgeError::InvalidSignature)
    }

    fn signing_preimage(
        claims: &AuthorityBindingClaims,
        signer_public_key: &[u8; 32],
    ) -> [u8; 32] {
        let mut hasher = blake3::Hasher::new();
        hasher.update(AUTHORITY_SIGNATURE_DOMAIN);
        Self::hash_claims(&mut hasher, claims);
        hasher.update(signer_public_key);
        *hasher.finalize().as_bytes()
    }

    fn hash_claims(hasher: &mut blake3::Hasher, claims: &AuthorityBindingClaims) {
        hasher.update(&claims.target_schema.schema_id().to_le_bytes());
        hasher.update(&claims.grant_id);
        hasher.update(&claims.grant_digest);
        hasher.update(&claims.actor_id);
        hasher.update(&claims.source_id.to_le_bytes());
        hasher.update(&[modality_code(claims.modality), action_code(claims.action)]);
        hasher.update(&claims.purpose_hash.to_le_bytes());
        hasher.update(&claims.original_blake3);
        hasher.update(&claims.index_commitment);
        hasher.update(&claims.issued_at_unix.to_le_bytes());
        hasher.update(&claims.expires_at_unix.to_le_bytes());
        hasher.update(&claims.receipt_nonce.to_le_bytes());
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SeenReceiptEntry {
    receipt_digest: [u8; 32],
    grant_id: [u8; 32],
    receipt_nonce: u64,
    action: SensingAction,
}

/// Zero-heap anti-replay tracker for `SignedAuthorityReceipt` consumption.
#[derive(Clone, Debug)]
pub struct ReceiptReplayWindow {
    entries: [Option<SeenReceiptEntry>; MAX_RECEIPT_REPLAY_WINDOW],
    len: usize,
}

impl Default for ReceiptReplayWindow {
    fn default() -> Self {
        Self::new()
    }
}

impl ReceiptReplayWindow {
    pub const fn new() -> Self {
        Self {
            entries: [None; MAX_RECEIPT_REPLAY_WINDOW],
            len: 0,
        }
    }

    pub const fn len(&self) -> usize {
        self.len
    }

    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Record a receipt consumption, rejecting any replay of the same receipt digest
    /// or `(grant_id, receipt_nonce)` pair.
    pub fn consume(&mut self, receipt: &SignedAuthorityReceipt) -> Result<[u8; 32], CustodyBridgeError> {
        let digest = receipt.receipt_digest();
        let claims = receipt.claims();
        for slot in self.entries.iter().take(self.len).flatten() {
            if slot.receipt_digest == digest
                || (slot.grant_id == claims.grant_id && slot.receipt_nonce == claims.receipt_nonce)
            {
                return Err(CustodyBridgeError::ReplayDetected);
            }
        }
        if self.len >= MAX_RECEIPT_REPLAY_WINDOW {
            return Err(CustodyBridgeError::ReplayWindowFull);
        }
        self.entries[self.len] = Some(SeenReceiptEntry {
            receipt_digest: digest,
            grant_id: claims.grant_id,
            receipt_nonce: claims.receipt_nonce,
            action: claims.action,
        });
        self.len += 1;
        Ok(digest)
    }
}

/// Expected stream and operation context verified against a `SignedAuthorityReceipt`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExpectedStreamBinding {
    pub expected_target: AuthorityReceiptTarget,
    pub stored_receipt_digest: [u8; 32],
    pub original_blake3: [u8; 32],
    pub index_commitment: [u8; 32],
    pub requested_operation: SensingRequest,
}

/// Verify and consume a `SignedAuthorityReceipt` against live consent authorities,
/// anti-replay state, and the exact stream digests (`original_blake3`, `index_commitment`,
/// `authority_receipt_digest`).
///
/// Prevents:
/// - Replay of an already-consumed receipt or nonce.
/// - Cross-stream or cross-file receipt substitution (`original_blake3` / `index_commitment`).
/// - Action escalation (e.g., using an `Acquire` or `Process` grant for `Export` or `Share`).
/// - Revoked or expired grant usage (re-runs `authorize_sensing` and `SensingPermit::revalidate`).
#[allow(clippy::too_many_arguments)]
pub fn verify_and_consume_stream_receipt(
    receipt: &SignedAuthorityReceipt,
    binding: &ExpectedStreamBinding,
    grant: &ConsentGrant,
    identity: &impl IdentityAuthority,
    sensor_scope: &impl SensingScopeAuthority,
    revocations: &impl RevocationAuthority,
    clock: &mut impl TrustedUnixClock,
    replay_window: &mut ReceiptReplayWindow,
) -> Result<RevalidatedSensingUse, CustodyBridgeError> {
    receipt.verify_signature()?;

    let computed_digest = receipt.receipt_digest();
    if computed_digest != binding.stored_receipt_digest {
        return Err(CustodyBridgeError::ReceiptDigestMismatch);
    }

    let claims = receipt.claims();
    if claims.target_schema != binding.expected_target {
        return Err(CustodyBridgeError::TargetSchemaMismatch {
            expected: binding.expected_target,
            actual: claims.target_schema,
        });
    }
    if claims.action != binding.requested_operation.action() {
        return Err(CustodyBridgeError::ActionEscalation {
            granted: claims.action,
            requested: binding.requested_operation.action(),
        });
    }
    if claims.source_id != binding.requested_operation.source_id() {
        return Err(CustodyBridgeError::SourceIdMismatch);
    }
    if claims.actor_id != binding.requested_operation.actor_id() {
        return Err(CustodyBridgeError::ActorIdMismatch);
    }
    if claims.purpose_hash != binding.requested_operation.purpose_hash() {
        return Err(CustodyBridgeError::PurposeHashMismatch);
    }
    if claims.modality != binding.requested_operation.modality() {
        return Err(CustodyBridgeError::ModalityMismatch);
    }
    if claims.original_blake3 != binding.original_blake3 {
        return Err(CustodyBridgeError::OriginalDigestMismatch);
    }
    if claims.index_commitment != binding.index_commitment {
        return Err(CustodyBridgeError::IndexCommitmentMismatch);
    }
    if claims.grant_id != grant.grant_id || claims.grant_digest != grant.compute_digest() {
        return Err(CustodyBridgeError::Authorization(
            AuthorizationError::InvalidGrant,
        ));
    }

    let permit: SensingPermit = authorize_sensing(
        grant,
        binding.requested_operation,
        identity,
        sensor_scope,
        revocations,
        clock,
    )?;
    let revalidated = permit.revalidate(grant, identity, sensor_scope, revocations, clock)?;
    if revalidated.checked_at_unix() >= claims.expires_at_unix {
        return Err(CustodyBridgeError::Authorization(
            AuthorizationError::GrantExpired,
        ));
    }

    replay_window.consume(receipt)?;
    Ok(revalidated)
}

/// Verify an `acquisition::Original` (mode `0x0104` preserved original bytes) against
/// a `SignedAuthorityReceipt` and live consent authorities.
#[allow(clippy::too_many_arguments)]
pub fn verify_acquisition_original_custody<'a>(
    original: Original<'a>,
    index_commitment: [u8; 32],
    stored_receipt_digest: [u8; 32],
    target_schema: AuthorityReceiptTarget,
    receipt: &SignedAuthorityReceipt,
    requested_operation: SensingRequest,
    grant: &ConsentGrant,
    identity: &impl IdentityAuthority,
    sensor_scope: &impl SensingScopeAuthority,
    revocations: &impl RevocationAuthority,
    clock: &mut impl TrustedUnixClock,
    replay_window: &mut ReceiptReplayWindow,
) -> Result<(Original<'a>, RevalidatedSensingUse), CustodyBridgeError> {
    let actual_blake3 = *blake3::hash(original.bytes()).as_bytes();
    if actual_blake3 != original.digest() {
        return Err(CustodyBridgeError::OriginalDigestMismatch);
    }
    let expected = ExpectedStreamBinding {
        expected_target: target_schema,
        stored_receipt_digest,
        original_blake3: actual_blake3,
        index_commitment,
        requested_operation,
    };
    let revalidated = verify_and_consume_stream_receipt(
        receipt,
        &expected,
        grant,
        identity,
        sensor_scope,
        revocations,
        clock,
        replay_window,
    )?;
    Ok((original, revalidated))
}

/// Verify a Schema 7 `acquisition::Derivation` against its `SignedAuthorityReceipt`,
/// confirming that the receipt is anchored to `AuthorityReceiptTarget::AcquisitionDerivationSchema7`,
/// the action is `SensingAction::Process`, and the output `Original` digest matches.
#[allow(clippy::too_many_arguments)]
pub fn verify_acquisition_derivation_custody<'a>(
    derivation: Derivation<'a>,
    index_commitment: [u8; 32],
    stored_receipt_digest: [u8; 32],
    receipt: &SignedAuthorityReceipt,
    requested_operation: SensingRequest,
    grant: &ConsentGrant,
    identity: &impl IdentityAuthority,
    sensor_scope: &impl SensingScopeAuthority,
    revocations: &impl RevocationAuthority,
    clock: &mut impl TrustedUnixClock,
    replay_window: &mut ReceiptReplayWindow,
) -> Result<(Derivation<'a>, RevalidatedSensingUse), CustodyBridgeError> {
    if requested_operation.action() != SensingAction::Process {
        return Err(CustodyBridgeError::ActionEscalation {
            granted: receipt.claims().action,
            requested: requested_operation.action(),
        });
    }
    for input in derivation.inputs() {
        let rehashed = *blake3::hash(input.bytes()).as_bytes();
        if rehashed != input.digest() {
            return Err(CustodyBridgeError::DerivationInputMismatch);
        }
    }
    let output_digest = *blake3::hash(derivation.output().bytes()).as_bytes();
    if output_digest != derivation.output().digest() {
        return Err(CustodyBridgeError::OriginalDigestMismatch);
    }
    let expected = ExpectedStreamBinding {
        expected_target: AuthorityReceiptTarget::AcquisitionDerivationSchema7,
        stored_receipt_digest,
        original_blake3: output_digest,
        index_commitment,
        requested_operation,
    };
    let revalidated = verify_and_consume_stream_receipt(
        receipt,
        &expected,
        grant,
        identity,
        sensor_scope,
        revocations,
        clock,
        replay_window,
    )?;
    Ok((derivation, revalidated))
}

/// Bridge binding a `SidecarReceipt` (`sensing::sidecar_attestation`) to a verified
/// `SignedAuthorityReceipt` whose `policy_commitment` equals `receipt.receipt_digest()`.
pub struct ConsentBoundSidecarAuthority {
    expected_signer: [u8; 32],
    expected_policy_commitment: [u8; 32],
    expected_chain_id: [u8; 32],
}

impl ConsentBoundSidecarAuthority {
    pub fn new(
        authority_receipt: &SignedAuthorityReceipt,
        expected_chain_id: [u8; 32],
    ) -> Result<Self, CustodyBridgeError> {
        authority_receipt.verify_signature()?;
        if expected_chain_id == [0u8; 32] {
            return Err(CustodyBridgeError::InvalidIdentifier);
        }
        Ok(Self {
            expected_signer: *authority_receipt.signer_public_key(),
            expected_policy_commitment: authority_receipt.receipt_digest(),
            expected_chain_id,
        })
    }
}

impl SidecarReceiptAuthority for ConsentBoundSidecarAuthority {
    fn permits(
        &self,
        receipt: &SidecarReceipt,
        _sidecar: &[u8; SIDECAR_BYTES],
        _payload: &[u8],
    ) -> bool {
        receipt.signer() == &self.expected_signer
            && receipt.policy_commitment() == &self.expected_policy_commitment
            && receipt.chain_id() == &self.expected_chain_id
    }
}

#[cfg(not(target_arch = "wasm32"))]
/// Native-only adapter bridging `ConsentBoundSidecarAuthority` and `RevalidatedSensingUse`
/// (`SensingAction::Store`) to `sensing::local_custody::CustodyAuthority`.
pub struct LocalCustodyStoreBridge {
    sidecar_auth: ConsentBoundSidecarAuthority,
    allowed_checkpoint: Option<[u8; 32]>,
    max_publish_bytes: usize,
    publish_consumed: bool,
}

#[cfg(not(target_arch = "wasm32"))]
impl LocalCustodyStoreBridge {
    pub fn for_store(
        revalidated: RevalidatedSensingUse,
        authority_receipt: &SignedAuthorityReceipt,
        expected_chain_id: [u8; 32],
        allowed_checkpoint: Option<[u8; 32]>,
        max_publish_bytes: usize,
    ) -> Result<Self, CustodyBridgeError> {
        if revalidated.request().action() != SensingAction::Store {
            return Err(CustodyBridgeError::ActionEscalation {
                granted: revalidated.request().action(),
                requested: SensingAction::Store,
            });
        }
        if authority_receipt.claims().action != SensingAction::Store {
            return Err(CustodyBridgeError::ActionEscalation {
                granted: authority_receipt.claims().action,
                requested: SensingAction::Store,
            });
        }
        let sidecar_auth = ConsentBoundSidecarAuthority::new(authority_receipt, expected_chain_id)?;
        Ok(Self {
            sidecar_auth,
            allowed_checkpoint,
            max_publish_bytes,
            publish_consumed: false,
        })
    }

    /// Publish a sidecar custody bundle under this single-use `Store` authorization.
    pub fn publish_authorized_bundle(
        &mut self,
        root: &Path,
        receipt: &SidecarReceipt,
        sidecar: &[u8; SIDECAR_BYTES],
        payload: &[u8],
    ) -> Result<PublishedBundle, CustodyError> {
        publish_bundle(
            root,
            sidecar,
            payload,
            receipt,
            self.max_publish_bytes,
            self,
        )
    }

    /// Read back a verified custody bundle when its checkpoint matches the authorized checkpoint.
    pub fn read_authorized_bundle(
        &mut self,
        root: &Path,
        checkpoint: &[u8; 32],
        sidecar: &mut [u8; SIDECAR_BYTES],
        buffer: &mut [u8],
    ) -> Result<ReadBundle, CustodyError> {
        read_bundle(
            root,
            checkpoint,
            self.max_publish_bytes,
            sidecar,
            buffer,
            self,
        )
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl SidecarReceiptAuthority for LocalCustodyStoreBridge {
    fn permits(
        &self,
        receipt: &SidecarReceipt,
        sidecar: &[u8; SIDECAR_BYTES],
        payload: &[u8],
    ) -> bool {
        self.sidecar_auth.permits(receipt, sidecar, payload)
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl CustodyAuthority for LocalCustodyStoreBridge {
    fn permit_publish(
        &mut self,
        _root: &Path,
        receipt: &SidecarReceipt,
        sidecar: &[u8; SIDECAR_BYTES],
        payload: &[u8],
        total_bytes: usize,
    ) -> bool {
        if self.publish_consumed || total_bytes > self.max_publish_bytes {
            return false;
        }
        if !self.sidecar_auth.permits(receipt, sidecar, payload) {
            return false;
        }
        self.publish_consumed = true;
        true
    }

    fn permit_read(&mut self, _root: &Path, checkpoint: &[u8; 32], max_bytes: usize) -> bool {
        max_bytes <= self.max_publish_bytes
            && self
                .allowed_checkpoint
                .is_some_and(|expected| &expected == checkpoint)
    }
}

const fn modality_code(modality: SensingModality) -> u8 {
    match modality {
        SensingModality::Wifi => 1,
        SensingModality::Bluetooth => 2,
        SensingModality::Cellular => 3,
        SensingModality::Sdr => 4,
        SensingModality::Radar => 5,
        SensingModality::VisibleCamera => 6,
        SensingModality::NearInfraredCamera => 7,
        SensingModality::ThermalCamera => 8,
        SensingModality::Microphone => 9,
        SensingModality::FieldInstrument => 10,
        SensingModality::Nfc => 11,
    }
}

const fn action_code(action: SensingAction) -> u8 {
    match action {
        SensingAction::Acquire => 1,
        SensingAction::Process => 2,
        SensingAction::Store => 3,
        SensingAction::Export => 4,
        SensingAction::Share => 5,
    }
}

/// Helper to bind an `acquisition::Context` calibration/uncertainty link to a known digest.
pub const fn verify_artifact_link_digest(
    link: acquisition::ArtifactLink,
    expected_digest: &[u8; 32],
) -> bool {
    link.identifier.value().is_some() && constant_time_eq_32(&link.digest, expected_digest)
}

const fn constant_time_eq_32(a: &[u8; 32], b: &[u8; 32]) -> bool {
    let mut diff = 0u8;
    let mut i = 0;
    while i < 32 {
        diff |= a[i] ^ b[i];
        i += 1;
    }
    diff == 0
}
