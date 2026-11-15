//! Signed adoption of canonical cooperative records; never inferred from a DID.
use crate::crypto::network::ed25519;
use qualia_cooperative_core::{
    authoring::*,
    projects::{Project, ProjectMembership},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::Write;

use qualia_cooperative_core::authoring::package_review;

pub const ADMISSION_CONTEXT: &[u8] = b"qualia.native-project-admission.v1";
pub const SESSION_CONTEXT: &[u8] = b"qualia.native-authoring-session.v1";

#[derive(Clone, Debug)]
pub struct TrustedProject {
    pub project_id: String,
    pub issuer_identifier: String,
    pub public_key: [u8; 32],
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdmissionSnapshot {
    pub version: u32,
    pub sequence: u64,
    pub issuer_identifier: String,
    pub project: Project,
    pub valid_from_unix: u64,
    pub valid_until_unix: u64,
    pub revoked: bool,
    pub subjects: Vec<SubjectAdmission>,
    /// Explicit exact-package grants. Empty preserves pre-extension signed bytes.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub package_reviews: Vec<package_review::PackageReviewGrant>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubjectAdmission {
    pub subject: String,
    /// The project's qualified capacity assertion, not legal identity proof.
    pub kind: PrincipalKind,
    pub controller_public_key: [u8; 32],
    pub membership: ProjectMembership,
    pub identifiers: Vec<IdentifierAssociationEvidence>,
    pub reviews: Vec<ReviewAdmission>,
    pub admission_ref: String,
    pub valid_from_unix: u64,
    pub valid_until_unix: u64,
    pub revoked: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewAdmission {
    pub id: String,
    pub document_id: String,
    pub admission_ref: String,
    pub valid_from_unix: u64,
    pub valid_until_unix: u64,
    pub revoked: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedAdmission {
    pub snapshot: AdmissionSnapshot,
    pub signature: Vec<u8>,
}

pub(super) struct BoundedBytes {
    pub bytes: Vec<u8>,
    pub limit: usize,
}
impl Write for BoundedBytes {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if self
            .bytes
            .len()
            .checked_add(bytes.len())
            .is_none_or(|n| n > self.limit)
        {
            return Err(std::io::Error::other(
                "native authority byte budget exceeded",
            ));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

pub fn admission_digest(
    snapshot: &AdmissionSnapshot,
    max_bytes: usize,
) -> Result<[u8; 32], String> {
    let mut writer = BoundedBytes {
        bytes: Vec::new(),
        limit: max_bytes,
    };
    serde_json::to_writer(&mut writer, snapshot).map_err(|e| e.to_string())?;
    Ok(Sha256::digest(&writer.bytes).into())
}

pub(super) fn live(now: u64, from: u64, until: u64) -> bool {
    from < until && from <= now && now < until
}

impl SignedAdmission {
    pub(super) fn verify(
        &self,
        anchor: &TrustedProject,
        max_bytes: usize,
    ) -> Result<[u8; 32], String> {
        let snapshot = &self.snapshot;
        if snapshot.version != 1
            || snapshot.sequence == 0
            || snapshot.project.id != anchor.project_id
            || snapshot.issuer_identifier != anchor.issuer_identifier
            || anchor.project_id.is_empty()
            || anchor.issuer_identifier.is_empty()
            || !strong_key(&anchor.public_key)
            || snapshot.subjects.len() > 256
            || snapshot.valid_from_unix >= snapshot.valid_until_unix
        {
            return Err("invalid pinned project admission binding/bounds".into());
        }
        let digest = admission_digest(snapshot, max_bytes)?;
        let signature: [u8; 64] = self
            .signature
            .as_slice()
            .try_into()
            .map_err(|_| "admission signature must contain 64 bytes")?;
        ed25519::verify_with_context(&anchor.public_key, &digest, ADMISSION_CONTEXT, &signature)
            .map_err(|_| "project admission signature verification failed")?;
        let limits = AuthoringLimits::default();
        let text = |s: &str| !s.is_empty() && s.len() <= limits.max_text_bytes;
        for (i, subject) in snapshot.subjects.iter().enumerate() {
            if !text(&subject.subject)
                || !text(&subject.admission_ref)
                || !strong_key(&subject.controller_public_key)
                || subject.membership.project_id != anchor.project_id
                || !text(&subject.membership.id)
                || !text(&subject.membership.member_did)
                || subject.valid_from_unix >= subject.valid_until_unix
                || subject.identifiers.is_empty()
                || subject.identifiers.len() > limits.max_identifiers
                || subject.reviews.len() > limits.max_review_grants
                || snapshot.subjects[..i].iter().any(|s| {
                    s.subject == subject.subject
                        || s.controller_public_key == subject.controller_public_key
                })
            {
                return Err("ambiguous or invalid admitted subject/membership".into());
            }
            for (index, identifier) in subject.identifiers.iter().enumerate() {
                if identifier.project_id != anchor.project_id
                    || identifier.valid_from_unix >= identifier.valid_until_unix
                    || subject.identifiers[..index].iter().any(|other| {
                        other.value == identifier.value
                            && other.scheme == identifier.scheme
                            && other.issuer == identifier.issuer
                    })
                    || [
                        &identifier.value,
                        &identifier.scheme,
                        &identifier.issuer,
                        &identifier.evidence_ref,
                    ]
                    .into_iter()
                    .any(|s| !text(s))
                {
                    return Err("invalid qualified identifier association".into());
                }
            }
            if !subject
                .identifiers
                .iter()
                .any(|i| i.value == subject.membership.member_did)
            {
                return Err("membership identifier lacks a qualified association".into());
            }
            for (index, review) in subject.reviews.iter().enumerate() {
                if !text(&review.id)
                    || !text(&review.document_id)
                    || !text(&review.admission_ref)
                    || review.valid_from_unix >= review.valid_until_unix
                    || subject.reviews[..index].iter().any(|r| r.id == review.id)
                {
                    return Err("ambiguous or invalid explicit review grant".into());
                }
            }
        }
        super::package_review::validate_admission_packages(snapshot)?;
        Ok(digest)
    }
}

// The shared contextual verifier predates strict small-order key rejection.
// Admission must reject weak keys before treating a signature as possession.
pub(super) fn strong_key(public: &[u8; 32]) -> bool {
    ed25519_dalek::VerifyingKey::from_bytes(public).is_ok_and(|key| !key.is_weak())
}
