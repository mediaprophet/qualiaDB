#![allow(dead_code)]
use super::{DigestScope, PolicyDocument, PolicyError};
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Host configuration, not an identity asserted by a policy/request. The host
/// must obtain this key/principal binding from its authenticated configuration.
#[derive(Clone, Copy)]
pub struct TrustedPolicyIdentity {
    pub(crate) key: [u8; 32],
    pub(crate) principal: u64,
    pub(crate) audience: u64,
    pub(crate) controller_generation: u64,
}
impl TrustedPolicyIdentity {
    pub fn from_host_configuration(
        key: [u8; 32],
        principal: u64,
        audience: u64,
        controller_generation: u64,
    ) -> Result<Self, PolicyError> {
        if principal == 0
            || audience == 0
            || controller_generation == 0
            || key == [0; 32]
            || VerifyingKey::from_bytes(&key).is_err()
        {
            return Err(PolicyError::WrongAuthority);
        }
        Ok(Self {
            key,
            principal,
            audience,
            controller_generation,
        })
    }
    pub fn controller_key(&self) -> [u8; 32] {
        self.key
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedPolicy {
    pub document: PolicyDocument,
    pub signature: [[u8; 32]; 2],
}
impl SignedPolicy {
    pub(crate) fn sign(document: PolicyDocument, key: &SigningKey) -> Result<Self, PolicyError> {
        document.validate()?;
        let signature = signature_parts(key.sign(&policy_digest(&document)).to_bytes());
        Ok(Self {
            document,
            signature,
        })
    }
    pub fn verify(&self, trust: &TrustedPolicyIdentity, now: u64) -> Result<(), PolicyError> {
        self.document.validate()?;
        let doc = &self.document;
        if doc.principal != trust.principal
            || doc.audience != trust.audience
            || doc.controller_generation != trust.controller_generation
        {
            return Err(PolicyError::WrongAuthority);
        }
        verify_signature(trust.key, policy_digest(doc), self.signature)?;
        doc.valid_at(now)
    }
}

pub(crate) fn signature_parts(bytes: [u8; 64]) -> [[u8; 32]; 2] {
    let mut parts = [[0; 32]; 2];
    parts[0].copy_from_slice(&bytes[..32]);
    parts[1].copy_from_slice(&bytes[32..]);
    parts
}
pub(crate) fn verify_signature(
    key: [u8; 32],
    digest: [u8; 32],
    parts: [[u8; 32]; 2],
) -> Result<(), PolicyError> {
    let mut bytes = [0; 64];
    bytes[..32].copy_from_slice(&parts[0]);
    bytes[32..].copy_from_slice(&parts[1]);
    VerifyingKey::from_bytes(&key)
        .map_err(|_| PolicyError::WrongAuthority)?
        .verify_strict(&digest, &Signature::from_bytes(&bytes))
        .map_err(|_| PolicyError::InvalidSignature)
}

/// Canonical digest: domain, fixed LE scalar fields, tagged digest scopes, and
/// length-prefixed ordered collections. JSON formatting never changes identity.
pub fn policy_digest(doc: &PolicyDocument) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(b"qualia:sentinel:installed-policy:v2\0");
    h.update(doc.version.to_le_bytes());
    for v in [
        doc.audience,
        doc.principal,
        doc.controller_generation,
        doc.policy_generation,
        doc.not_before,
        doc.expires_at,
    ] {
        h.update(v.to_le_bytes());
    }
    h.update((doc.rules.actors.as_slice().len() as u64).to_le_bytes());
    for actor in doc.rules.actors.as_slice() {
        actor.hash_into(&mut h);
    }
    for scope in [doc.rules.model, doc.rules.request] {
        match scope {
            DigestScope::Any => h.update([0]),
            DigestScope::Exact(d) => {
                h.update([1]);
                h.update(d);
            }
        }
    }
    h.update((doc.rules.graph_scopes.as_slice().len() as u64).to_le_bytes());
    for v in doc.rules.graph_scopes.as_slice() {
        h.update(v.to_le_bytes());
    }
    h.update((doc.rules.program.as_slice().len() as u64).to_le_bytes());
    for op in doc.rules.program.as_slice() {
        h.update(op.canonical());
    }
    for list in [
        doc.rules.norms.as_slice(),
        doc.rules.facts.as_slice(),
        doc.rules.epistemic.as_slice(),
        doc.rules.trace.as_slice(),
    ] {
        h.update((list.len() as u64).to_le_bytes());
        for q in list {
            for v in [
                q.subject,
                q.predicate,
                q.object,
                q.context,
                q.metadata,
                q.parity,
            ] {
                h.update(v.to_le_bytes());
            }
        }
    }
    h.update(doc.rules.required_claim.to_le_bytes());
    h.update(doc.rules.temporal_invariant.to_le_bytes());
    h.update([u8::from(doc.rules.non_derogable)]);
    h.finalize().into()
}
