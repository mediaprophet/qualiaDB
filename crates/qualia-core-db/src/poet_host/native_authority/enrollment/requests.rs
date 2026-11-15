use super::{
    super::{
        admission::{live, strong_key, BoundedBytes},
        ed25519, random, SubjectAdmission,
    },
    NativeSigner,
};
use qualia_cooperative_core::{authoring::AuthoringLimits, projects::ProjectRole};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
pub const ENROLLMENT_CONTEXT: &[u8] = b"qualia.native-controller-enrollment.v1";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnrollmentRequest {
    pub version: u32,
    pub nonce: [u8; 32],
    pub project_id: String,
    pub admission_sequence: u64,
    pub admission_digest: [u8; 32],
    pub submitted_at_unix: u64,
    pub expires_at_unix: u64,
    pub candidate: SubjectAdmission,
    pub signature: Vec<u8>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnrollmentStatus {
    Pending,
    Approved,
    Declined,
    Revoked,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnrollmentRecord {
    pub request: EnrollmentRequest,
    pub status: EnrollmentStatus,
    pub decided_by_subject: Option<String>,
    pub decision_evidence_ref: Option<String>,
    pub human_verification_ref: Option<String>,
    pub decided_at_unix: Option<u64>,
}
impl EnrollmentRequest {
    /// Caller supplies explicit capacity/identifiers/windows; no automatic human assertion.
    pub fn signed(
        project_id: String,
        admission_sequence: u64,
        admission_digest: [u8; 32],
        submitted_at_unix: u64,
        expires_at_unix: u64,
        candidate: SubjectAdmission,
        signer: &NativeSigner,
    ) -> Result<Self, String> {
        if signer.public_key() != candidate.controller_public_key {
            return Err("candidate controller differs from signer".into());
        }
        let mut request = Self {
            version: 1,
            nonce: random()?,
            project_id,
            admission_sequence,
            admission_digest,
            submitted_at_unix,
            expires_at_unix,
            candidate,
            signature: Vec::new(),
        };
        request.validate(submitted_at_unix)?;
        request.signature = signer
            .sign(&request.digest()?, ENROLLMENT_CONTEXT)?
            .to_vec();
        Ok(request)
    }
    pub fn id(&self) -> String {
        format!("urn:qualia:enrollment:{}", hex::encode(self.nonce))
    }
    pub fn digest(&self) -> Result<[u8; 32], String> {
        // Borrow the payload so even invalid caller input cannot force an
        // unbounded clone before the serializer enforces its byte budget.
        #[derive(Serialize)]
        struct Proof<'a> {
            version: u32,
            nonce: &'a [u8; 32],
            project_id: &'a str,
            admission_sequence: u64,
            admission_digest: &'a [u8; 32],
            submitted_at_unix: u64,
            expires_at_unix: u64,
            candidate: &'a SubjectAdmission,
            signature: &'a [u8],
        }
        let unsigned = Proof {
            version: self.version,
            nonce: &self.nonce,
            project_id: &self.project_id,
            admission_sequence: self.admission_sequence,
            admission_digest: &self.admission_digest,
            submitted_at_unix: self.submitted_at_unix,
            expires_at_unix: self.expires_at_unix,
            candidate: &self.candidate,
            signature: &[],
        };
        let mut writer = BoundedBytes {
            bytes: Vec::new(),
            limit: 128 * 1024,
        };
        serde_json::to_writer(&mut writer, &unsigned).map_err(|e| e.to_string())?;
        Ok(Sha256::digest(&writer.bytes).into())
    }
    pub(super) fn validate(&self, now: u64) -> Result<(), String> {
        let c = &self.candidate;
        let text =
            |s: &str| !s.trim().is_empty() && s.len() <= AuthoringLimits::default().max_text_bytes;
        if self.version != 1
            || self.nonce == [0; 32]
            || !text(&self.project_id)
            || self.admission_sequence == 0
            || self.admission_digest == [0; 32]
            || !live(now, self.submitted_at_unix, self.expires_at_unix)
            || self.expires_at_unix - self.submitted_at_unix > 86400
            || !text(&c.subject)
            || !strong_key(&c.controller_public_key)
            || !text(&c.admission_ref)
            || c.revoked
            || !c.reviews.is_empty()
            || c.membership.project_id != self.project_id
            || !text(&c.membership.id)
            || !text(&c.membership.member_did)
            || c.membership.role == ProjectRole::Steward
            || !live(now, c.valid_from_unix, c.valid_until_unix)
            || c.identifiers.is_empty()
            || c.identifiers.len() > AuthoringLimits::default().max_identifiers
        {
            return Err(
                "invalid/expired enrollment or attempted requested privilege escalation".into(),
            );
        }
        let mut member_bound = false;
        for (index, i) in c.identifiers.iter().enumerate() {
            if i.project_id != self.project_id
                || [&i.value, &i.scheme, &i.issuer, &i.evidence_ref]
                    .into_iter()
                    .any(|s| !text(s))
                || !live(now, i.valid_from_unix, i.valid_until_unix)
                || c.identifiers[..index]
                    .iter()
                    .any(|p| p.value == i.value && p.scheme == i.scheme && p.issuer == i.issuer)
            {
                return Err("enrollment identifiers are invalid, duplicate or expired".into());
            }
            member_bound |= i.value == c.membership.member_did;
        }
        if !member_bound {
            return Err("membership identifier lacks qualified association".into());
        }
        Ok(())
    }
    pub(super) fn verify(&self, now: u64) -> Result<(), String> {
        self.validate(now)?;
        let signature: [u8; 64] = self
            .signature
            .as_slice()
            .try_into()
            .map_err(|_| "invalid enrollment proof length")?;
        ed25519::verify_with_context(
            &self.candidate.controller_public_key,
            &self.digest()?,
            ENROLLMENT_CONTEXT,
            &signature,
        )
        .map_err(|_| "enrollment possession proof invalid".to_string())
    }
}
