use super::super::{
    evidence::{policy_digest_v3, SignedEvidencePolicy, AUDIENCE_V3},
    policy_digest,
    signature::{signature_parts, verify_signature},
    PolicyError, SignedPolicy, TrustedPolicyIdentity, MAX_POLICY_JSON_BYTES,
};
use super::legacy::LegacyRecord;
use ed25519_dalek::{Signer, SigningKey};
use serde::{Deserialize, Serialize, Serializer};
use sha2::{Digest, Sha256};

#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "policy_version", content = "policy", deny_unknown_fields)]
pub(crate) enum InstalledPolicy {
    #[serde(rename = "2")]
    V2(SignedPolicy),
    #[serde(rename = "3")]
    V3(SignedEvidencePolicy),
}
impl InstalledPolicy {
    pub(crate) fn digest(&self) -> [u8; 32] {
        match self {
            Self::V2(p) => policy_digest(&p.document),
            Self::V3(p) => policy_digest_v3(&p.document),
        }
    }
    fn verify(&self, trust: &TrustedPolicyIdentity, generation: u64) -> Result<(), PolicyError> {
        match self {
            Self::V2(p) => {
                if p.document.policy_generation != generation {
                    return Err(PolicyError::StaleGeneration);
                }
                p.verify(trust, p.document.not_before)
            }
            Self::V3(p) => {
                let base = &p.document.base;
                if base.policy_generation != generation {
                    return Err(PolicyError::StaleGeneration);
                }
                if base.audience != trust.audience {
                    return Err(PolicyError::WrongAuthority);
                }
                let evidence_trust = TrustedPolicyIdentity {
                    audience: AUDIENCE_V3,
                    ..*trust
                };
                p.verify(&evidence_trust, base.not_before).map(|_| ())
            }
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct VersionedRecord {
    version: u32,
    serial: u64,
    generation: u64,
    next_session: u64,
    active: Option<InstalledPolicy>,
    signature: [[u8; 32]; 2],
}
impl VersionedRecord {
    fn digest(&self, trust: &TrustedPolicyIdentity) -> [u8; 32] {
        let mut h = Sha256::new();
        h.update(b"qualia:sentinel:policy-store:v2\0");
        h.update(self.version.to_le_bytes());
        h.update(trust.key);
        for n in [
            trust.principal,
            trust.audience,
            trust.controller_generation,
            self.serial,
            self.generation,
            self.next_session,
        ] {
            h.update(n.to_le_bytes());
        }
        match &self.active {
            None => h.update([0]),
            Some(InstalledPolicy::V2(p)) => {
                h.update([2]);
                h.update(policy_digest(&p.document));
                h.update(p.signature[0]);
                h.update(p.signature[1]);
            }
            Some(InstalledPolicy::V3(p)) => {
                h.update([3]);
                h.update(policy_digest_v3(&p.document));
                h.update(p.signature[0]);
                h.update(p.signature[1]);
            }
        }
        h.finalize().into()
    }
    fn verify(&self, trust: &TrustedPolicyIdentity) -> Result<(), PolicyError> {
        if self.version != 2 || self.serial == 0 || self.generation == 0 {
            return Err(PolicyError::InvalidPolicy);
        }
        verify_signature(trust.key, self.digest(trust), self.signature)?;
        if let Some(p) = &self.active {
            p.verify(trust, self.generation)?;
        }
        Ok(())
    }
}

#[derive(Clone)]
pub(crate) enum Record {
    Legacy(LegacyRecord),
    Versioned(VersionedRecord),
}
// Delegation preserves the exact legacy field ordering and representation.
impl Serialize for Record {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Legacy(r) => r.serialize(s),
            Self::Versioned(r) => r.serialize(s),
        }
    }
}
impl Record {
    pub(crate) fn empty() -> Self {
        Self::Legacy(LegacyRecord::empty())
    }
    pub(crate) fn decode(bytes: &[u8]) -> Result<Self, PolicyError> {
        if bytes.len() > MAX_POLICY_JSON_BYTES {
            return Err(PolicyError::Capacity);
        }
        // Probe allocates no JSON tree. The concrete second pass denies unknown
        // fields and enforces PolicyList bounds; no untagged trial decoding.
        #[derive(Deserialize)]
        struct Version {
            version: u32,
        }
        let v: Version = serde_json::from_slice(bytes).map_err(|_| PolicyError::Encoding)?;
        match v.version {
            1 => serde_json::from_slice(bytes)
                .map(Self::Legacy)
                .map_err(|_| PolicyError::Encoding),
            2 => serde_json::from_slice(bytes)
                .map(Self::Versioned)
                .map_err(|_| PolicyError::Encoding),
            _ => Err(PolicyError::InvalidPolicy),
        }
    }
    pub(crate) fn serial(&self) -> u64 {
        match self {
            Self::Legacy(r) => r.serial,
            Self::Versioned(r) => r.serial,
        }
    }
    pub(crate) fn generation(&self) -> u64 {
        match self {
            Self::Legacy(r) => r.generation,
            Self::Versioned(r) => r.generation,
        }
    }
    pub(crate) fn next_session(&self) -> u64 {
        match self {
            Self::Legacy(r) => r.next_session,
            Self::Versioned(r) => r.next_session,
        }
    }
    pub(crate) fn set_serial(&mut self, n: u64) {
        match self {
            Self::Legacy(r) => r.serial = n,
            Self::Versioned(r) => r.serial = n,
        }
    }
    pub(crate) fn set_generation(&mut self, n: u64) {
        match self {
            Self::Legacy(r) => r.generation = n,
            Self::Versioned(r) => r.generation = n,
        }
    }
    pub(crate) fn set_next_session(&mut self, n: u64) {
        match self {
            Self::Legacy(r) => r.next_session = n,
            Self::Versioned(r) => r.next_session = n,
        }
    }
    pub(crate) fn installed(&self) -> bool {
        match self {
            Self::Legacy(r) => r.active.is_some(),
            Self::Versioned(r) => r.active.is_some(),
        }
    }
    pub(crate) fn v2(&self) -> Option<&SignedPolicy> {
        match self {
            Self::Legacy(r) => r.active.as_ref(),
            Self::Versioned(r) => match &r.active {
                Some(InstalledPolicy::V2(p)) => Some(p),
                _ => None,
            },
        }
    }
    pub(crate) fn v3(&self) -> Option<&SignedEvidencePolicy> {
        match self {
            Self::Versioned(r) => match &r.active {
                Some(InstalledPolicy::V3(p)) => Some(p),
                _ => None,
            },
            _ => None,
        }
    }
    pub(crate) fn policy_digest(&self) -> Option<[u8; 32]> {
        match self {
            Self::Legacy(r) => r.active.as_ref().map(|p| policy_digest(&p.document)),
            Self::Versioned(r) => r.active.as_ref().map(InstalledPolicy::digest),
        }
    }
    pub(crate) fn clear(&mut self) {
        match self {
            Self::Legacy(r) => r.active = None,
            Self::Versioned(r) => r.active = None,
        }
    }
    pub(crate) fn install_v2(&mut self, p: SignedPolicy) {
        match self {
            Self::Legacy(r) => r.active = Some(p),
            Self::Versioned(r) => r.active = Some(InstalledPolicy::V2(p)),
        }
    }
    /// Only explicit V3 installation changes record representation. No old
    /// policy signature is reinterpreted or synthesized during load/revoke.
    pub(crate) fn install_v3(&mut self, p: SignedEvidencePolicy) {
        match self {
            Self::Versioned(r) => r.active = Some(InstalledPolicy::V3(p)),
            Self::Legacy(r) => {
                *self = Self::Versioned(VersionedRecord {
                    version: 2,
                    serial: r.serial,
                    generation: r.generation,
                    next_session: r.next_session,
                    active: Some(InstalledPolicy::V3(p)),
                    signature: [[0; 32]; 2],
                })
            }
        }
    }
    pub(crate) fn digest(&self, trust: &TrustedPolicyIdentity) -> [u8; 32] {
        match self {
            Self::Legacy(r) => r.digest(trust),
            Self::Versioned(r) => r.digest(trust),
        }
    }
    pub(crate) fn sign(&mut self, trust: &TrustedPolicyIdentity, key: &SigningKey) {
        match self {
            Self::Legacy(r) => r.sign(trust, key),
            Self::Versioned(r) => {
                r.signature = signature_parts(key.sign(&r.digest(trust)).to_bytes())
            }
        }
    }
    pub(crate) fn verify(&self, trust: &TrustedPolicyIdentity) -> Result<(), PolicyError> {
        match self {
            Self::Legacy(r) => r.verify(trust),
            Self::Versioned(r) => r.verify(trust),
        }
    }
}
