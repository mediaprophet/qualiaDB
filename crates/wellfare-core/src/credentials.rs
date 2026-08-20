//! Verifiable-credential status cache and presentation building — Restricted vault records.
//!
//! Implements the master plan's credential items (audit CRE-01..09). Two honesty
//! boundaries are load-bearing and documented at their point of use:
//!
//! 1. [`evaluate_state`] is a **status cache**, not cryptographic verification. It classifies
//!    a credential from cheap, local facts (expiry, an issuer-trust allow-list, claim presence).
//!    It does NOT check an issuer signature, a revocation registry, or a proof — a
//!    `VerificationState::IssuerTrusted` value means "the issuer DID is on the caller's trust
//!    list", never "the credential's signature verified". Real proof verification lives in the
//!    identity / key-vault layer.
//!
//! 2. [`build_presentation`] is **plain JSON field selection**, NOT cryptographic selective
//!    disclosure. It simply drops the claim keys the holder did not select. It gives no
//!    unlinkability, no zero-knowledge property, and no proof that the omitted claims exist or
//!    that the disclosed ones were signed. The type is named [`FieldSelectedPresentation`] so it
//!    cannot be mistaken for ZK selective disclosure (plan §Q11).

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::record::{
    EpistemicStatus, EvidenceType, InstantBridge, RecordEnvelope, SensitivityClass,
};

/// Locally-derived verification status of a credential.
///
/// This is a **cache of cheap local checks**, ordered from weakest to strongest confidence,
/// with `Revoked`/`Expired` as terminal negative states. It is explicitly NOT a statement about
/// cryptographic validity (see the module header).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationState {
    /// No local evidence of validity (e.g. no claims, unknown issuer).
    Unverified,
    /// Has at least one claim — the shape looks like a credential. Not a schema-conformance proof.
    SchemaValid,
    /// The issuer DID is on the caller-supplied trust allow-list. Not a signature check.
    IssuerTrusted,
    /// The holder / issuer marked this credential revoked. Terminal.
    Revoked,
    /// The credential's `expires_at_unix` is in the past. Terminal.
    Expired,
}

impl Default for VerificationState {
    fn default() -> Self {
        Self::Unverified
    }
}

/// A verifiable-credential envelope as held in the WellFair vault.
///
/// `claims` is an ordered key/value list (not a map) so presentation selection is deterministic
/// and duplicate keys round-trip faithfully. The credential is Restricted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CredentialRecord {
    pub id: String,
    pub issuer_did: String,
    pub subject_did: String,
    pub credential_type: String,
    pub claims: Vec<(String, String)>,
    pub issued_at_unix: u32,
    /// High-resolution instant (T71 bridge). Preferred over `issued_at_unix`
    /// when present; the u32 field is kept for backward-compatible deserialization.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub issued_at_instant: Option<InstantBridge>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at_unix: Option<u32>,
    /// High-resolution expiry instant (T71 bridge). Preferred over
    /// `expires_at_unix` when present.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub expires_at_instant: Option<InstantBridge>,
    #[serde(default)]
    pub verification_state: VerificationState,
}

impl CredentialRecord {
    pub fn new(
        issuer_did: impl Into<String>,
        subject_did: impl Into<String>,
        credential_type: impl Into<String>,
        issued_at_unix: u32,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            issuer_did: issuer_did.into(),
            subject_did: subject_did.into(),
            credential_type: credential_type.into(),
            claims: Vec::new(),
            issued_at_unix,
            issued_at_instant: Some(InstantBridge::from_coarse(issued_at_unix)),
            expires_at_unix: None,
            expires_at_instant: None,
            verification_state: VerificationState::Unverified,
        }
    }

    /// Resolve the issued-at instant, preferring the high-resolution
    /// `InstantBridge` field when present (T71 bridge).
    pub fn issued_at(&self) -> InstantBridge {
        self.issued_at_instant
            .unwrap_or_else(|| InstantBridge::from_coarse(self.issued_at_unix))
    }

    /// Resolve the expiry instant, preferring the high-resolution
    /// `InstantBridge` field when present (T71 bridge).
    pub fn expires_at(&self) -> Option<InstantBridge> {
        self.expires_at_instant
            .or_else(|| self.expires_at_unix.map(|t| InstantBridge::from_coarse(t)))
    }

    /// Add a claim key/value pair (builder-style). Order is preserved.
    pub fn with_claim(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.claims.push((key.into(), value.into()));
        self
    }
}

/// A holder-driven request for a subset of a credential's claims.
///
/// This models *which* claim keys a verifier asked for (or the holder chose to reveal). It carries
/// no cryptographic material — see [`build_presentation`] for the honesty boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PresentationRequest {
    /// The credential the presentation is derived from.
    pub credential_id: String,
    /// The claim keys to disclose. Keys absent from the credential are ignored.
    pub selected_claim_keys: Vec<String>,
}

impl PresentationRequest {
    pub fn new(credential_id: impl Into<String>, selected_claim_keys: Vec<String>) -> Self {
        Self {
            credential_id: credential_id.into(),
            selected_claim_keys,
        }
    }
}

/// The result of [`build_presentation`]: a credential reduced to the holder-selected claim keys.
///
/// **Honesty boundary — this is plain JSON field selection, not cryptographic selective
/// disclosure.** The omitted claims are simply not copied in. There is no unlinkability, no
/// zero-knowledge proof, and no cryptographic assurance that the disclosed claims were signed by
/// the issuer or that undisclosed claims exist. Do not treat a `FieldSelectedPresentation` as a
/// ZK / BBS+ selective-disclosure proof. The type name encodes this distinction on purpose.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldSelectedPresentation {
    pub credential_id: String,
    pub issuer_did: String,
    pub subject_did: String,
    pub credential_type: String,
    /// Only the claims whose keys were selected AND present on the credential.
    pub disclosed_claims: Vec<(String, String)>,
    /// The verification state copied from the source credential at build time (a cached hint,
    /// not a re-verification).
    pub verification_state: VerificationState,
}

/// Classify a credential from cheap local facts. **Status cache, NOT cryptographic verification**
/// (see the module header): no signature, revocation-registry, or proof check is performed.
///
/// Precedence:
/// - `Revoked` if the stored state is already `Revoked` (revocation is sticky).
/// - `Expired` if `expires_at_unix` is set and `<= now_unix`.
/// - else `IssuerTrusted` if `issuer_did` is in `trusted_issuers`.
/// - else `SchemaValid` if the credential carries at least one claim.
/// - else `Unverified`.
pub fn evaluate_state(
    cred: &CredentialRecord,
    trusted_issuers: &[String],
    now_unix: u32,
) -> VerificationState {
    if cred.verification_state == VerificationState::Revoked {
        return VerificationState::Revoked;
    }
    if let Some(expiry) = cred.expires_at_unix {
        if expiry <= now_unix {
            return VerificationState::Expired;
        }
    }
    if trusted_issuers.iter().any(|i| i == &cred.issuer_did) {
        return VerificationState::IssuerTrusted;
    }
    if !cred.claims.is_empty() {
        return VerificationState::SchemaValid;
    }
    VerificationState::Unverified
}

/// Build a field-selected presentation of `cred`, disclosing only claims whose keys appear in
/// `selected_claim_keys`. Keys requested but absent on the credential are silently skipped; the
/// disclosed order follows the credential's own claim order (deterministic).
///
/// **This is field selection, not selective disclosure** — see [`FieldSelectedPresentation`].
pub fn build_presentation(
    cred: &CredentialRecord,
    selected_claim_keys: &[String],
) -> FieldSelectedPresentation {
    let disclosed_claims: Vec<(String, String)> = cred
        .claims
        .iter()
        .filter(|(k, _)| selected_claim_keys.iter().any(|s| s == k))
        .cloned()
        .collect();
    FieldSelectedPresentation {
        credential_id: cred.id.clone(),
        issuer_did: cred.issuer_did.clone(),
        subject_did: cred.subject_did.clone(),
        credential_type: cred.credential_type.clone(),
        disclosed_claims,
        verification_state: cred.verification_state,
    }
}

pub fn credential_record_id(uuid: &str) -> String {
    format!("urn:wellfair:credential:{uuid}")
}

/// Build the Restricted record envelope for a credential.
///
/// Credentials are third-party attestations, so evidence is `ClinicianObserved`-style external
/// assertion rather than self-report; we classify them as `Inferred` provenance because the local
/// vault did not observe the underlying fact directly — it holds an issuer's claim. Epistemic
/// status is `Asserted` (the issuer asserts it); trust is expressed via `verification_state`, not
/// by weakening the epistemic status.
pub fn build_credential_envelope(
    cred: &CredentialRecord,
    owner_did: &str,
    author_did: &str,
    asserted_unix: u32,
    blob_hash: Option<String>,
) -> RecordEnvelope {
    RecordEnvelope {
        id: credential_record_id(&cred.id),
        owner_did: owner_did.to_string(),
        author_did: author_did.to_string(),
        proxy_did: None,
        epistemic_status: EpistemicStatus::Asserted,
        evidence_type: EvidenceType::Inferred,
        sensitivity: SensitivityClass::Restricted,
        asserted_time_unix: asserted_unix,
        asserted_instant: None,
        valid_time_start_unix: Some(cred.issued_at_unix),
        valid_time_start_instant: cred.issued_at_instant,
        valid_time_end_unix: cred.expires_at_unix,
        valid_time_end_instant: cred.expires_at_instant,
        predecessor_id: None,
        blob_hash,
        tombstone: false,
    }
}

pub fn credential_summary(cred: &CredentialRecord) -> String {
    serde_json::json!({
        "credential_type": cred.credential_type,
        "issuer_did": cred.issuer_did,
        "subject_did": cred.subject_did,
        "claim_count": cred.claims.len(),
        "issued_at_unix": cred.issued_at_unix,
        "expires_at_unix": cred.expires_at_unix,
        "verification_state": cred.verification_state,
    })
    .to_string()
}

pub fn presentation_summary(pres: &FieldSelectedPresentation) -> String {
    serde_json::json!({
        "credential_id": pres.credential_id,
        "credential_type": pres.credential_type,
        "issuer_did": pres.issuer_did,
        "disclosed_keys": pres
            .disclosed_claims
            .iter()
            .map(|(k, _)| k.clone())
            .collect::<Vec<_>>(),
        "verification_state": pres.verification_state,
        // Names the honesty boundary in the stored projection itself.
        "disclosure_kind": "field_selection_not_zk",
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_cred() -> CredentialRecord {
        CredentialRecord::new(
            "did:wf:issuer",
            "did:wf:subject",
            "ProofOfAddress",
            1_700_000_000,
        )
        .with_claim("full_name", "Jane Roe")
        .with_claim("street", "1 Camper Lane")
        .with_claim("postcode", "3000")
    }

    #[test]
    fn credential_envelope_kind_and_class() {
        let cred = sample_cred();
        let env = build_credential_envelope(
            &cred,
            "did:wf:owner",
            "did:wf:owner",
            1_700_000_100,
            Some("blobhash".into()),
        );
        assert!(env.id.contains(":credential:"));
        assert_eq!(env.sensitivity, SensitivityClass::Restricted);
        assert_eq!(env.evidence_type, EvidenceType::Inferred);
        assert_eq!(env.epistemic_status, EpistemicStatus::Asserted);
        // The central journal dispatcher must classify a credential id as "credential",
        // not fall through to the generic "record" arm.
        assert_eq!(
            crate::conditions::journal_kind_for_record_id(&env.id),
            "credential"
        );
    }

    #[test]
    fn envelope_valid_time_end_tracks_expiry() {
        let mut cred = sample_cred();
        cred.expires_at_unix = Some(1_800_000_000);
        let env = build_credential_envelope(&cred, "did:wf:owner", "did:wf:owner", 1, None);
        assert_eq!(env.valid_time_start_unix, Some(1_700_000_000));
        assert_eq!(env.valid_time_end_unix, Some(1_800_000_000));
    }

    #[test]
    fn evaluate_state_expired_beats_trusted() {
        let mut cred = sample_cred();
        cred.expires_at_unix = Some(1_000);
        let trusted = vec!["did:wf:issuer".to_string()];
        // now is past expiry — Expired must win even though the issuer is trusted.
        assert_eq!(
            evaluate_state(&cred, &trusted, 2_000),
            VerificationState::Expired
        );
    }

    #[test]
    fn evaluate_state_issuer_trusted_when_not_expired() {
        let mut cred = sample_cred();
        // Far-future but within u32 range (u32::MAX ≈ year 2106).
        cred.expires_at_unix = Some(4_000_000_000);
        let trusted = vec!["did:wf:issuer".to_string()];
        assert_eq!(
            evaluate_state(&cred, &trusted, 1_700_000_000),
            VerificationState::IssuerTrusted
        );
    }

    #[test]
    fn evaluate_state_schema_valid_when_untrusted_issuer_has_claims() {
        let cred = sample_cred(); // untrusted issuer, has claims, no expiry
        assert_eq!(
            evaluate_state(&cred, &[], 1_700_000_000),
            VerificationState::SchemaValid
        );
    }

    #[test]
    fn evaluate_state_unverified_when_no_claims_and_untrusted() {
        let cred = CredentialRecord::new("did:wf:issuer", "did:wf:subject", "Empty", 1_700_000_000);
        assert_eq!(
            evaluate_state(&cred, &[], 1_700_000_000),
            VerificationState::Unverified
        );
    }

    #[test]
    fn evaluate_state_revoked_is_sticky() {
        let mut cred = sample_cred();
        cred.verification_state = VerificationState::Revoked;
        let trusted = vec!["did:wf:issuer".to_string()];
        // Even a trusted, unexpired, claim-bearing credential stays Revoked.
        assert_eq!(
            evaluate_state(&cred, &trusted, 1_700_000_000),
            VerificationState::Revoked
        );
    }

    #[test]
    fn build_presentation_returns_only_selected_claims() {
        let cred = sample_cred();
        let selected = vec!["full_name".to_string(), "postcode".to_string()];
        let pres = build_presentation(&cred, &selected);

        // Only the two selected claims are present, in credential order.
        assert_eq!(pres.disclosed_claims.len(), 2);
        assert_eq!(pres.disclosed_claims[0].0, "full_name");
        assert_eq!(pres.disclosed_claims[1].0, "postcode");

        // The unselected claim is omitted entirely — no key, no value.
        assert!(!pres.disclosed_claims.iter().any(|(k, _)| k == "street"));
        let json = presentation_summary(&pres);
        assert!(!json.contains("street"));
        assert!(!json.contains("1 Camper Lane"));

        // Provenance metadata is carried through.
        assert_eq!(pres.credential_id, cred.id);
        assert_eq!(pres.issuer_did, "did:wf:issuer");
        // The honesty boundary is named in the summary projection.
        assert!(json.contains("field_selection_not_zk"));
    }

    #[test]
    fn build_presentation_ignores_unknown_selected_keys() {
        let cred = sample_cred();
        let selected = vec!["full_name".to_string(), "does_not_exist".to_string()];
        let pres = build_presentation(&cred, &selected);
        assert_eq!(pres.disclosed_claims.len(), 1);
        assert_eq!(pres.disclosed_claims[0].0, "full_name");
    }

    #[test]
    fn build_presentation_empty_selection_discloses_nothing() {
        let cred = sample_cred();
        let pres = build_presentation(&cred, &[]);
        assert!(pres.disclosed_claims.is_empty());
    }

    #[test]
    fn credential_summary_includes_primary_fields() {
        let cred = sample_cred();
        let summary = credential_summary(&cred);
        assert!(summary.contains("ProofOfAddress"));
        assert!(summary.contains("did:wf:issuer"));
        // Claim count, not raw claim values, in the summary.
        assert!(summary.contains("\"claim_count\":3"));
    }
}
