//! Signed actor scope is policy data, never authentication or installation power.
use super::PolicyError;
use qualia_cooperative_core::authoring::PrincipalKind;
use serde::{Deserialize, Serialize};

/// One explicit association; a policy may list several identifiers for the same
/// subject. Supported principal kinds do not define an exhaustive identity ontology.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActorPolicyScope {
    pub subject: u64,
    pub kind: PrincipalKind,
    pub controller: [u8; 32],
    pub project_digest: [u8; 32],
    pub selected_identifier_digest: [u8; 32],
    pub association_digest: [u8; 32],
    pub admission_digest: [u8; 32],
    pub admission_sequence: u64,
    pub authority_generation: u64,
    pub valid_until: u64,
}
impl ActorPolicyScope {
    pub(crate) fn validate(self) -> Result<(), PolicyError> {
        if self.subject == 0
            || self.controller == [0; 32]
            || [
                self.project_digest,
                self.selected_identifier_digest,
                self.association_digest,
                self.admission_digest,
            ]
            .contains(&[0; 32])
            || self.admission_sequence == 0
            || self.authority_generation == 0
            || self.valid_until == 0
        {
            return Err(PolicyError::Scope);
        }
        Ok(())
    }
    #[cfg(not(target_arch = "wasm32"))]
    pub fn from_evidence(
        e: crate::poet_host::native_authority::inference::InferenceAssociationEvidence,
    ) -> Self {
        Self {
            subject: e.subject,
            kind: e.kind,
            controller: e.controller,
            project_digest: e.project_digest,
            selected_identifier_digest: e.selected_identifier_digest,
            association_digest: e.association_digest,
            admission_digest: e.admission_digest,
            admission_sequence: e.admission_sequence,
            authority_generation: e.authority_generation,
            valid_until: e.valid_until,
        }
    }
    pub(crate) fn hash_into(self, h: &mut sha2::Sha256) {
        use sha2::Digest;
        h.update(self.subject.to_le_bytes());
        h.update([match self.kind {
            PrincipalKind::Human => 0,
            PrincipalKind::SoftwareAgent => 1,
            PrincipalKind::Organization => 2,
        }]);
        for d in [
            self.controller,
            self.project_digest,
            self.selected_identifier_digest,
            self.association_digest,
            self.admission_digest,
        ] {
            h.update(d);
        }
        for n in [
            self.admission_sequence,
            self.authority_generation,
            self.valid_until,
        ] {
            h.update(n.to_le_bytes());
        }
    }
}
