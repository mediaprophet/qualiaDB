//! Exact legacy record-v1 canonical encoding; no V3 interpretation.
use super::super::signature::{signature_parts, verify_signature};
use super::super::{policy_digest, PolicyError, SignedPolicy, TrustedPolicyIdentity};
use ed25519_dalek::{Signer, SigningKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct LegacyRecord {
    pub(crate) version: u32,
    pub(crate) serial: u64,
    pub(crate) generation: u64,
    pub(crate) next_session: u64,
    pub(crate) active: Option<SignedPolicy>,
    pub(crate) signature: [[u8; 32]; 2],
}
impl LegacyRecord {
    pub(crate) fn empty() -> Self {
        Self {
            version: 1,
            serial: 0,
            generation: 0,
            next_session: 0,
            active: None,
            signature: [[0; 32]; 2],
        }
    }
    pub(crate) fn digest(&self, trust: &TrustedPolicyIdentity) -> [u8; 32] {
        let mut h = Sha256::new();
        h.update(b"qualia:sentinel:policy-store:v1\0");
        h.update(self.version.to_le_bytes());
        h.update(trust.key);
        for v in [
            trust.principal,
            trust.audience,
            trust.controller_generation,
            self.serial,
            self.generation,
            self.next_session,
        ] {
            h.update(v.to_le_bytes());
        }
        match &self.active {
            None => h.update([0]),
            Some(policy) => {
                h.update([1]);
                h.update(policy_digest(&policy.document));
                h.update(policy.signature[0]);
                h.update(policy.signature[1]);
            }
        }
        h.finalize().into()
    }
    pub(crate) fn sign(&mut self, trust: &TrustedPolicyIdentity, key: &SigningKey) {
        self.signature = signature_parts(key.sign(&self.digest(trust)).to_bytes());
    }
    pub(crate) fn verify(&self, trust: &TrustedPolicyIdentity) -> Result<(), PolicyError> {
        if self.version != 1 || self.serial == 0 || self.generation == 0 {
            return Err(PolicyError::InvalidPolicy);
        }
        verify_signature(trust.key, self.digest(trust), self.signature)?;
        if let Some(policy) = &self.active {
            if policy.document.policy_generation != self.generation {
                return Err(PolicyError::StaleGeneration);
            }
            // Keep expired installations loadable for explicit replacement or
            // revocation; session admission separately checks the current time.
            policy.verify(trust, policy.document.not_before)?;
        }
        Ok(())
    }
}
