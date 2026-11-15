//! Signed, caller-buffered custody receipts for experimental sensing sidecars.
//!
//! The signature binds exact sidecar and payload bytes; it does not validate
//! their format, scientific truth, permission, signer authority or chain
//! completeness. The host must decode the sidecar against a resolved payload
//! and enforce current authority, revocation, scope, and checkpoint policy.
//! This module performs no storage, network, or device I/O.

use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use sha2::{Digest, Sha256};

pub const SIDECAR_BYTES: usize = 768;
pub const SIDECAR_RECEIPT_BYTES: usize = 272;
const MAGIC: &[u8; 4] = b"SCR1";
const SIGN_DOMAIN: &[u8] = b"qualia:sensing:sidecar-receipt:v1\0";
const SIDECAR_DOMAIN: &[u8] = b"qualia:sensing:sidecar:v1\0";
const PAYLOAD_DOMAIN: &[u8] = b"qualia:sensing:sidecar-payload:v1\0";
const CHECKPOINT_DOMAIN: &[u8] = b"qualia:sensing:sidecar-checkpoint:v1\0";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SidecarReceiptError {
    InvalidEncoding,
    InvalidChain,
    InvalidIdentifier,
    BufferTooSmall,
    PayloadBudgetExceeded,
    SidecarMismatch,
    PayloadMismatch,
    InvalidSigner,
    InvalidSignature,
    NotAuthorized,
}

/// Immutable signed receipt. A zero policy commitment represents no supplied
/// policy reference; authority decides whether that is permissible.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SidecarReceipt {
    signer: [u8; 32],
    sidecar_digest: [u8; 32],
    payload_digest: [u8; 32],
    policy_commitment: [u8; 32],
    chain_id: [u8; 32],
    previous: [u8; 32],
    sequence: u64,
    signature: [u8; 64],
}

/// Verify current signer, subject/guardian policy and chain state. This must
/// independently interpret the decoded sidecar and bind the payload location;
/// neither possession of a key nor a signed policy reference grants access.
pub trait SidecarReceiptAuthority {
    fn permits(
        &self,
        receipt: &SidecarReceipt,
        sidecar: &[u8; SIDECAR_BYTES],
        payload: &[u8],
    ) -> bool;
}

impl SidecarReceipt {
    #[allow(clippy::too_many_arguments)]
    pub fn sign(
        sidecar: &[u8; SIDECAR_BYTES],
        payload: &[u8],
        payload_byte_budget: usize,
        policy_commitment: [u8; 32],
        chain_id: [u8; 32],
        sequence: u64,
        previous: Option<[u8; 32]>,
        signing_key: &SigningKey,
    ) -> Result<Self, SidecarReceiptError> {
        if payload.len() > payload_byte_budget {
            return Err(SidecarReceiptError::PayloadBudgetExceeded);
        }
        let previous = previous.unwrap_or([0; 32]);
        validate_chain(&chain_id, sequence, &previous)?;
        let mut receipt = Self {
            signer: signing_key.verifying_key().to_bytes(),
            sidecar_digest: digest(SIDECAR_DOMAIN, sidecar),
            payload_digest: digest(PAYLOAD_DOMAIN, payload),
            policy_commitment,
            chain_id,
            previous,
            sequence,
            signature: [0; 64],
        };
        receipt.signature = signing_key.sign(&receipt.signing_digest()).to_bytes();
        Ok(receipt)
    }

    pub const fn signer(&self) -> &[u8; 32] {
        &self.signer
    }
    pub const fn sidecar_digest(&self) -> &[u8; 32] {
        &self.sidecar_digest
    }
    pub const fn payload_digest(&self) -> &[u8; 32] {
        &self.payload_digest
    }
    pub const fn policy_commitment(&self) -> &[u8; 32] {
        &self.policy_commitment
    }
    pub const fn chain_id(&self) -> &[u8; 32] {
        &self.chain_id
    }
    pub const fn previous(&self) -> &[u8; 32] {
        &self.previous
    }
    pub const fn sequence(&self) -> u64 {
        self.sequence
    }

    /// Checks bytes and signature only. It grants no policy authority.
    pub fn verify_signature(
        &self,
        sidecar: &[u8; SIDECAR_BYTES],
        payload: &[u8],
        payload_byte_budget: usize,
    ) -> Result<(), SidecarReceiptError> {
        if payload.len() > payload_byte_budget {
            return Err(SidecarReceiptError::PayloadBudgetExceeded);
        }
        validate_chain(&self.chain_id, self.sequence, &self.previous)?;
        if digest(SIDECAR_DOMAIN, sidecar) != self.sidecar_digest {
            return Err(SidecarReceiptError::SidecarMismatch);
        }
        if digest(PAYLOAD_DOMAIN, payload) != self.payload_digest {
            return Err(SidecarReceiptError::PayloadMismatch);
        }
        let key = VerifyingKey::from_bytes(&self.signer)
            .map_err(|_| SidecarReceiptError::InvalidSigner)?;
        key.verify_strict(
            &self.signing_digest(),
            &Signature::from_bytes(&self.signature),
        )
        .map_err(|_| SidecarReceiptError::InvalidSignature)
    }

    pub fn verify_with_authority(
        &self,
        sidecar: &[u8; SIDECAR_BYTES],
        payload: &[u8],
        payload_byte_budget: usize,
        authority: &impl SidecarReceiptAuthority,
    ) -> Result<(), SidecarReceiptError> {
        self.verify_signature(sidecar, payload, payload_byte_budget)?;
        if !authority.permits(self, sidecar, payload) {
            return Err(SidecarReceiptError::NotAuthorized);
        }
        Ok(())
    }

    /// Hash of this complete signed receipt for the next chain position. This
    /// does not prove that earlier receipts are available or were authorized.
    pub fn checkpoint(&self) -> [u8; 32] {
        let mut bytes = [0; SIDECAR_RECEIPT_BYTES];
        self.write_to_array(&mut bytes);
        digest(CHECKPOINT_DOMAIN, &bytes)
    }

    pub fn encode(&self, out: &mut [u8]) -> Result<usize, SidecarReceiptError> {
        if out.len() < SIDECAR_RECEIPT_BYTES {
            return Err(SidecarReceiptError::BufferTooSmall);
        }
        let mut bytes = [0; SIDECAR_RECEIPT_BYTES];
        self.write_to_array(&mut bytes);
        out[..SIDECAR_RECEIPT_BYTES].copy_from_slice(&bytes);
        Ok(SIDECAR_RECEIPT_BYTES)
    }

    /// Structural decoding only; callers must verify bytes and authority.
    pub fn decode(input: &[u8]) -> Result<Self, SidecarReceiptError> {
        if input.len() != SIDECAR_RECEIPT_BYTES
            || &input[..4] != MAGIC
            || u16::from_le_bytes([input[4], input[5]]) != 1
            || u16::from_le_bytes([input[6], input[7]]) as usize != SIDECAR_RECEIPT_BYTES
        {
            return Err(SidecarReceiptError::InvalidEncoding);
        }
        let receipt = Self {
            signer: array32(input, 8),
            sidecar_digest: array32(input, 40),
            payload_digest: array32(input, 72),
            policy_commitment: array32(input, 104),
            chain_id: array32(input, 136),
            previous: array32(input, 168),
            sequence: u64::from_le_bytes(
                input[200..208]
                    .try_into()
                    .map_err(|_| SidecarReceiptError::InvalidEncoding)?,
            ),
            signature: input[208..272]
                .try_into()
                .map_err(|_| SidecarReceiptError::InvalidEncoding)?,
        };
        validate_chain(&receipt.chain_id, receipt.sequence, &receipt.previous)?;
        Ok(receipt)
    }

    fn signing_digest(&self) -> [u8; 32] {
        let mut h = Sha256::new();
        h.update(SIGN_DOMAIN);
        h.update(self.signer);
        h.update(self.sidecar_digest);
        h.update(self.payload_digest);
        h.update(self.policy_commitment);
        h.update(self.chain_id);
        h.update(self.previous);
        h.update(self.sequence.to_le_bytes());
        h.finalize().into()
    }

    fn write_to_array(&self, out: &mut [u8; SIDECAR_RECEIPT_BYTES]) {
        out[..4].copy_from_slice(MAGIC);
        out[4..6].copy_from_slice(&1u16.to_le_bytes());
        out[6..8].copy_from_slice(&(SIDECAR_RECEIPT_BYTES as u16).to_le_bytes());
        out[8..40].copy_from_slice(&self.signer);
        out[40..72].copy_from_slice(&self.sidecar_digest);
        out[72..104].copy_from_slice(&self.payload_digest);
        out[104..136].copy_from_slice(&self.policy_commitment);
        out[136..168].copy_from_slice(&self.chain_id);
        out[168..200].copy_from_slice(&self.previous);
        out[200..208].copy_from_slice(&self.sequence.to_le_bytes());
        out[208..272].copy_from_slice(&self.signature);
    }
}

fn validate_chain(
    chain: &[u8; 32],
    sequence: u64,
    previous: &[u8; 32],
) -> Result<(), SidecarReceiptError> {
    if chain.iter().all(|b| *b == 0) {
        return Err(SidecarReceiptError::InvalidIdentifier);
    }
    let genesis = previous.iter().all(|b| *b == 0);
    if (sequence == 0) != genesis {
        return Err(SidecarReceiptError::InvalidChain);
    }
    Ok(())
}

fn digest(domain: &[u8], bytes: &[u8]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(domain);
    h.update((bytes.len() as u64).to_le_bytes());
    h.update(bytes);
    h.finalize().into()
}

fn array32(input: &[u8], offset: usize) -> [u8; 32] {
    let mut out = [0; 32];
    out.copy_from_slice(&input[offset..offset + 32]);
    out
}

#[cfg(test)]
#[path = "sidecar_attestation_tests.rs"]
mod tests;
