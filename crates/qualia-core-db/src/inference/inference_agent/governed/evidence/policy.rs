use super::{ObservationRequirements, ScoreRequirement};
use crate::{
    inference::inference_agent::governed::{
        policy_digest, PolicyDocument, PolicyError, TrustedPolicyIdentity,
    },
    q_hash,
};
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
pub const AUDIENCE_V3: u64 = q_hash("qualia:inference:candidate-evidence:v3");
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidencePolicy {
    pub version: u32,
    pub audience: u64,
    /// Unchanged V2 contents and validation; never a migrated V2 signature.
    pub base: PolicyDocument,
    pub observations: ObservationRequirements,
}
impl EvidencePolicy {
    pub(super) fn validate(&self) -> Result<(), PolicyError> {
        if self.version != 3 || self.audience != AUDIENCE_V3 {
            return Err(PolicyError::InvalidPolicy);
        }
        self.base.validate()?;
        self.observations.validate()
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedEvidencePolicy {
    pub document: EvidencePolicy,
    pub signature: [[u8; 32]; 2],
}
/// Signature/schema verification only, not live issuer/actor/revocation authority.
pub struct VerifiedEvidencePolicy<'a> {
    pub(super) signed: &'a SignedEvidencePolicy,
}
impl SignedEvidencePolicy {
    pub(crate) fn sign(document: EvidencePolicy, key: &SigningKey) -> Result<Self, PolicyError> {
        document.validate()?;
        let bytes = key.sign(&policy_digest_v3(&document)).to_bytes();
        let mut signature = [[0; 32]; 2];
        signature[0].copy_from_slice(&bytes[..32]);
        signature[1].copy_from_slice(&bytes[32..]);
        Ok(Self {
            document,
            signature,
        })
    }
    pub fn verify<'a>(
        &'a self,
        trust: &TrustedPolicyIdentity,
        now: u64,
    ) -> Result<VerifiedEvidencePolicy<'a>, PolicyError> {
        self.document.validate()?;
        let d = &self.document;
        if d.audience != trust.audience
            || d.base.principal != trust.principal
            || d.base.controller_generation != trust.controller_generation
        {
            return Err(PolicyError::WrongAuthority);
        }
        let mut bytes = [0; 64];
        bytes[..32].copy_from_slice(&self.signature[0]);
        bytes[32..].copy_from_slice(&self.signature[1]);
        VerifyingKey::from_bytes(&trust.key)
            .map_err(|_| PolicyError::WrongAuthority)?
            .verify_strict(&policy_digest_v3(d), &Signature::from_bytes(&bytes))
            .map_err(|_| PolicyError::InvalidSignature)?;
        d.base.valid_at(now)?;
        Ok(VerifiedEvidencePolicy { signed: self })
    }
}
/// Validating callers must invoke document.validate before trusting this digest.
/// The validated nested V2 digest covers every existing actor/rule/Quin field.
pub fn policy_digest_v3(d: &EvidencePolicy) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(b"qualia:sentinel:installed-evidence-policy:v3\0");
    h.update(d.version.to_le_bytes());
    h.update(d.audience.to_le_bytes());
    h.update(policy_digest(&d.base));
    let r = &d.observations;
    h.update(r.allowed_sources.to_le_bytes());
    h.update([r.allowed_artifact_origins]);
    h.update(r.max_payload_bytes.to_le_bytes());
    h.update([u8::from(r.allow_empty_artifact)]);
    match r.score {
        ScoreRequirement::Optional => h.update([0]),
        ScoreRequirement::Required(m) => h.update([m as u8]),
    };
    h.update((r.score_program.as_slice().len() as u64).to_le_bytes());
    for op in r.score_program.as_slice() {
        h.update(op.canonical());
    }
    h.finalize().into()
}
