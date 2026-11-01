//! Cryptographic, source-specific extension to the health-category consent grant.
//!
//! The assertion signs one actor/source/modality/action/purpose tuple and the exact
//! `ConsentGrant` digest. It does not establish that the signing key belongs to a
//! human, provide a trusted clock, or persist revocation; those remain broker gates.

use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use sha2::{Digest, Sha256};

use crate::governance::consent_contract::ConsentGrant;

use super::authorization::{
    AuthorizationError, SensingAction, SensingModality, SensingRequest, SensingScopeAuthority,
};

const SCOPE_DOMAIN: &[u8] = b"qualia:sensing-scope:v1\0";
pub const SIGNED_SCOPE_VERSION: u8 = 1;
pub const SIGNED_SCOPE_ENCODED_LEN: usize = 211;

/// Structural transport errors. Successful decoding does not verify a
/// signature, establish identity, or authorize a sensing operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SignedScopeEncodingError {
    BufferTooSmall { required: usize, available: usize },
    WrongLength { expected: usize, actual: usize },
    UnsupportedVersion,
    InvalidTag,
    InvalidField,
}

/// Signed grant extension for exactly one sensing request. Fields are private to
/// prevent unsigned construction from becoming an accepted scope assertion.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SignedSensingScope {
    grant_id: [u8; 32],
    grant_digest: [u8; 32],
    actor_id: [u8; 32],
    source_id: u64,
    modality: SensingModality,
    action: SensingAction,
    purpose_hash: u64,
    signer_public_key: [u8; 32],
    signature: [u8; 64],
}

impl SignedSensingScope {
    /// Sign a source-bound extension using the same key that signed the grant.
    /// The grant signature is checked before the extension is constructed.
    pub fn sign(
        grant: &ConsentGrant,
        signer: &SigningKey,
        request: SensingRequest,
    ) -> Result<Self, AuthorizationError> {
        let signer_public_key = signer.verifying_key().to_bytes();
        if signer_public_key != grant.principal_did {
            return Err(AuthorizationError::IdentityUnverified);
        }
        verify_grant_signature(grant)?;

        let mut scope = Self {
            grant_id: grant.grant_id,
            grant_digest: grant.compute_digest(),
            actor_id: request.actor_id(),
            source_id: request.source_id(),
            modality: request.modality(),
            action: request.action(),
            purpose_hash: request.purpose_hash(),
            signer_public_key,
            signature: [0; 64],
        };
        let message = scope.message_digest();
        scope.signature = signer.sign(&message).to_bytes();
        Ok(scope)
    }

    /// Encode to a fixed-size versioned form in caller-owned storage. No bytes
    /// are written if the destination is too small; excess destination bytes
    /// remain untouched.
    pub fn encode_into(&self, out: &mut [u8]) -> Result<usize, SignedScopeEncodingError> {
        if out.len() < SIGNED_SCOPE_ENCODED_LEN {
            return Err(SignedScopeEncodingError::BufferTooSmall {
                required: SIGNED_SCOPE_ENCODED_LEN,
                available: out.len(),
            });
        }
        let mut offset = 0;
        out[offset] = SIGNED_SCOPE_VERSION;
        offset += 1;
        put_bytes(out, &mut offset, &self.grant_id);
        put_bytes(out, &mut offset, &self.grant_digest);
        put_bytes(out, &mut offset, &self.actor_id);
        put_bytes(out, &mut offset, &self.source_id.to_be_bytes());
        out[offset] = modality_tag(self.modality);
        offset += 1;
        out[offset] = action_tag(self.action);
        offset += 1;
        put_bytes(out, &mut offset, &self.purpose_hash.to_be_bytes());
        put_bytes(out, &mut offset, &self.signer_public_key);
        put_bytes(out, &mut offset, &self.signature);
        debug_assert_eq!(offset, SIGNED_SCOPE_ENCODED_LEN);
        Ok(offset)
    }

    /// Decode the exact versioned wire length. This is parsing only; call
    /// `verify_scope` with the matching signed grant before relying on it.
    pub fn decode(bytes: &[u8]) -> Result<Self, SignedScopeEncodingError> {
        if bytes.len() != SIGNED_SCOPE_ENCODED_LEN {
            return Err(SignedScopeEncodingError::WrongLength {
                expected: SIGNED_SCOPE_ENCODED_LEN,
                actual: bytes.len(),
            });
        }
        let mut offset = 0;
        if take_u8(bytes, &mut offset) != SIGNED_SCOPE_VERSION {
            return Err(SignedScopeEncodingError::UnsupportedVersion);
        }
        let grant_id = take_array::<32>(bytes, &mut offset);
        let grant_digest = take_array::<32>(bytes, &mut offset);
        let actor_id = take_array::<32>(bytes, &mut offset);
        let source_id = u64::from_be_bytes(take_array::<8>(bytes, &mut offset));
        let modality = modality_from_tag(take_u8(bytes, &mut offset))?;
        let action = action_from_tag(take_u8(bytes, &mut offset))?;
        let purpose_hash = u64::from_be_bytes(take_array::<8>(bytes, &mut offset));
        let signer_public_key = take_array::<32>(bytes, &mut offset);
        let signature = take_array::<64>(bytes, &mut offset);
        debug_assert_eq!(offset, SIGNED_SCOPE_ENCODED_LEN);
        if grant_id == [0; 32]
            || actor_id == [0; 32]
            || source_id == 0
            || purpose_hash == 0
            || signer_public_key == [0; 32]
        {
            return Err(SignedScopeEncodingError::InvalidField);
        }
        Ok(Self {
            grant_id,
            grant_digest,
            actor_id,
            source_id,
            modality,
            action,
            purpose_hash,
            signer_public_key,
            signature,
        })
    }

    pub const fn grant_id(self) -> [u8; 32] {
        self.grant_id
    }
    pub const fn grant_digest(self) -> [u8; 32] {
        self.grant_digest
    }
    pub const fn actor_id(self) -> [u8; 32] {
        self.actor_id
    }
    pub const fn source_id(self) -> u64 {
        self.source_id
    }
    pub const fn modality(self) -> SensingModality {
        self.modality
    }
    pub const fn action(self) -> SensingAction {
        self.action
    }
    pub const fn purpose_hash(self) -> u64 {
        self.purpose_hash
    }

    fn message_digest(&self) -> [u8; 32] {
        let mut hash = Sha256::new();
        hash.update(SCOPE_DOMAIN);
        hash.update(self.grant_id);
        hash.update(self.grant_digest);
        hash.update(self.actor_id);
        hash.update(self.source_id.to_be_bytes());
        hash.update([modality_tag(self.modality)]);
        hash.update([action_tag(self.action)]);
        hash.update(self.purpose_hash.to_be_bytes());
        hash.update(self.signer_public_key);
        hash.finalize().into()
    }

    fn verify_for(
        &self,
        grant: &ConsentGrant,
        expected_grant_digest: &[u8; 32],
        request: &SensingRequest,
    ) -> Result<(), AuthorizationError> {
        if self.grant_id != grant.grant_id
            || self.grant_digest != *expected_grant_digest
            || self.grant_digest != grant.compute_digest()
        {
            return Err(AuthorizationError::InvalidGrant);
        }
        if self.signer_public_key != grant.principal_did {
            return Err(AuthorizationError::IdentityUnverified);
        }
        if self.actor_id != request.actor_id()
            || self.source_id != request.source_id()
            || self.modality != request.modality()
            || self.action != request.action()
            || self.purpose_hash != request.purpose_hash()
        {
            return Err(AuthorizationError::UnsupportedSensorScope);
        }
        verify_grant_signature(grant)?;
        let key = VerifyingKey::from_bytes(&self.signer_public_key)
            .map_err(|_| AuthorizationError::IdentityUnverified)?;
        key.verify_strict(
            &self.message_digest(),
            &Signature::from_bytes(&self.signature),
        )
        .map_err(|_| AuthorizationError::IdentityUnverified)
    }
}

impl SensingScopeAuthority for SignedSensingScope {
    fn verify_scope(
        &self,
        grant: &ConsentGrant,
        grant_digest: &[u8; 32],
        request: &SensingRequest,
    ) -> Result<(), AuthorizationError> {
        self.verify_for(grant, grant_digest, request)
    }
}

/// Verify the grant's signature independently of expiry. The broker checks
/// validity time using its required trusted clock before consulting this scope.
fn verify_grant_signature(grant: &ConsentGrant) -> Result<(), AuthorizationError> {
    let key = VerifyingKey::from_bytes(&grant.principal_did)
        .map_err(|_| AuthorizationError::InvalidGrant)?;
    key.verify_strict(
        &grant.compute_digest(),
        &Signature::from_bytes(&grant.signature),
    )
    .map_err(|_| AuthorizationError::InvalidGrant)
}

fn modality_tag(value: SensingModality) -> u8 {
    match value {
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

fn action_tag(value: SensingAction) -> u8 {
    match value {
        SensingAction::Acquire => 1,
        SensingAction::Process => 2,
        SensingAction::Store => 3,
        SensingAction::Export => 4,
        SensingAction::Share => 5,
    }
}

fn modality_from_tag(tag: u8) -> Result<SensingModality, SignedScopeEncodingError> {
    match tag {
        1 => Ok(SensingModality::Wifi),
        2 => Ok(SensingModality::Bluetooth),
        3 => Ok(SensingModality::Cellular),
        4 => Ok(SensingModality::Sdr),
        5 => Ok(SensingModality::Radar),
        6 => Ok(SensingModality::VisibleCamera),
        7 => Ok(SensingModality::NearInfraredCamera),
        8 => Ok(SensingModality::ThermalCamera),
        9 => Ok(SensingModality::Microphone),
        10 => Ok(SensingModality::FieldInstrument),
        11 => Ok(SensingModality::Nfc),
        _ => Err(SignedScopeEncodingError::InvalidTag),
    }
}

fn action_from_tag(tag: u8) -> Result<SensingAction, SignedScopeEncodingError> {
    match tag {
        1 => Ok(SensingAction::Acquire),
        2 => Ok(SensingAction::Process),
        3 => Ok(SensingAction::Store),
        4 => Ok(SensingAction::Export),
        5 => Ok(SensingAction::Share),
        _ => Err(SignedScopeEncodingError::InvalidTag),
    }
}

fn put_bytes<const N: usize>(out: &mut [u8], offset: &mut usize, value: &[u8; N]) {
    out[*offset..*offset + N].copy_from_slice(value);
    *offset += N;
}

fn take_u8(bytes: &[u8], offset: &mut usize) -> u8 {
    let value = bytes[*offset];
    *offset += 1;
    value
}

fn take_array<const N: usize>(bytes: &[u8], offset: &mut usize) -> [u8; N] {
    let mut value = [0; N];
    value.copy_from_slice(&bytes[*offset..*offset + N]);
    *offset += N;
    value
}

#[cfg(test)]
#[path = "signed_scope_tests.rs"]
mod tests;
