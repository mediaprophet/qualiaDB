//! Authority attestation — the general domain model of which a "government letter" is one preset.
//!
//! A `GovernmentLetter` (see `welfare_support.rs`) is only the narrow special case where the
//! authorizing body happens to be a government. The general concept is an **authority
//! attestation**: some **authority** (a government, but equally a pathology company, a university,
//! a bank, an NGO) attests something about a **subject**, optionally through an **agent-in-capacity**
//! (the natural person or software agent acting *for* that authority in a stated role), and delivers
//! it in one of several orthogonal **representations** (a PDF, a verifiable credential, or a PDF with
//! an embedded credential).
//!
//! The authority *type* is a URI-ish string id, NOT a closed enum — `"authority:government"` is just
//! a well-known upper-level constant. Downstream owners coin their own type ids freely
//! (`"authority:pathology:acme-labs"`, etc.) without touching this file.
//!
//! Like the other WellFair Restricted records, the attestation body lives out-of-band as a blob
//! (`blob_hash`) — the PDF / credential bytes live in the content-addressed blob store, and this
//! struct is the indexable envelope over them, flagging whether the owner must act.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::record::{
    EpistemicStatus, EvidenceType, InstantBridge, RecordEnvelope, SensitivityClass,
};

/// Well-known upper-level authority type ids.
///
/// These are an *open* vocabulary: the field on [`Authority`] is a plain `String`, so callers may
/// use one of these consts or coin their own URI-ish id (e.g. `"authority:pathology:acme-labs"`).
/// "Government" is deliberately just one of many — a pathology company, a university, a bank and an
/// NGO are equally authorities.
pub mod authority_type {
    /// A government body — the classic "government letter" case.
    pub const GOVERNMENT: &str = "authority:government";
    /// A pathology / medical laboratory or provider.
    pub const PATHOLOGY: &str = "authority:pathology";
    /// A university, school, or other educational institution.
    pub const EDUCATION: &str = "authority:education";
    /// A bank or other financial institution.
    pub const FINANCIAL: &str = "authority:financial";
    /// A non-governmental / charitable organisation.
    pub const NGO: &str = "authority:ngo";
}

// Re-export the well-known consts at module top level for ergonomic `authority_attestation::GOVERNMENT`.
pub use authority_type::{EDUCATION, FINANCIAL, GOVERNMENT, NGO, PATHOLOGY};

/// The authorizing body behind an attestation.
///
/// `type_id` is an extensible URI-ish string (see [`authority_type`]); `label` is the human-readable
/// name. `jurisdiction` locates the authority's remit (e.g. `"AU"`, `"AU-VIC"`), and `department` is
/// an optional branch / sub-unit within the authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Authority {
    /// Extensible type id, e.g. `authority_type::GOVERNMENT` or a caller-coined `"authority:…"`.
    pub type_id: String,
    /// Human-readable name of the authority, e.g. "Services Australia", "Acme Pathology".
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub jurisdiction: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub department: Option<String>,
}

impl Authority {
    /// A new authority of the given extensible `type_id` with a human `label`.
    pub fn new(type_id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            type_id: type_id.into(),
            label: label.into(),
            jurisdiction: None,
            department: None,
        }
    }

    /// True if this authority is a government (`authority_type::GOVERNMENT`).
    pub fn is_government(&self) -> bool {
        self.type_id == authority_type::GOVERNMENT
    }

    /// True if this authority is a pathology / medical body — its attestations carry more weight as
    /// evidence (see [`build_authority_attestation_envelope`]). Matches the exact `PATHOLOGY` const
    /// or any type id under the `authority:pathology`/`authority:medical` prefix so caller-coined
    /// ids like `"authority:pathology:acme-labs"` are still recognised as clinician-grade.
    pub fn is_medical(&self) -> bool {
        self.type_id == authority_type::PATHOLOGY
            || self.type_id.starts_with("authority:pathology")
            || self.type_id.starts_with("authority:medical")
    }
}

/// A natural person (or software agent) acting FOR the authority in a stated capacity/role —
/// e.g. a pathologist signing off a result, a case officer issuing a determination.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentInCapacity {
    /// Name of the person or software agent.
    pub name: String,
    /// The role/capacity they act in, e.g. "pathologist", "case officer", "registrar".
    pub capacity: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub did: Option<String>,
}

impl AgentInCapacity {
    pub fn new(name: impl Into<String>, capacity: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            capacity: capacity.into(),
            did: None,
        }
    }

    /// Attach a DID to the agent.
    pub fn with_did(mut self, did: impl Into<String>) -> Self {
        self.did = Some(did.into());
        self
    }
}

/// How an attestation is delivered — ORTHOGONAL to its content.
///
/// The same attestation (same authority, subject, statement) can arrive as a plain document blob,
/// as a verifiable credential, or as a PDF with a credential baked in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Representation {
    /// A document blob (e.g. a PDF letter).
    Pdf,
    /// A verifiable credential.
    Credential,
    /// A PDF document with an embedded verifiable credential.
    PdfWithEmbeddedCredential,
}

impl Default for Representation {
    fn default() -> Self {
        Self::Pdf
    }
}

/// An attestation issued by an authority (through an optional agent-in-capacity) about a subject.
///
/// This is the general model; [`government_letter`] is a preset over it. The attestation body
/// itself lives out-of-band as a blob (`blob_hash`); this struct is the indexable envelope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthorityAttestation {
    pub id: String,
    pub authority: Authority,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<AgentInCapacity>,
    #[serde(default)]
    pub representation: Representation,
    /// What / who is attested (the subject of the attestation).
    pub subject: String,
    /// Free-text statement / subject line describing the attestation.
    pub statement: String,
    #[serde(default)]
    pub action_required: bool,
    pub issued_unix: u32,
    /// High-resolution instant (T71 bridge). Preferred over `issued_unix`
    /// when present; the u32 field is kept for backward-compatible deserialization.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub issued_instant: Option<InstantBridge>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blob_hash: Option<String>,
}

impl AuthorityAttestation {
    /// A new attestation from `authority` about `subject`, with the given free-text `statement`,
    /// issued at `issued_unix`. Defaults: no agent, `Pdf` representation, no action required, no blob.
    pub fn new(
        authority: Authority,
        subject: impl Into<String>,
        statement: impl Into<String>,
        issued_unix: u32,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            authority,
            agent: None,
            representation: Representation::default(),
            subject: subject.into(),
            statement: statement.into(),
            action_required: false,
            issued_unix,
            issued_instant: Some(InstantBridge::from_coarse(issued_unix)),
            blob_hash: None,
        }
    }

    /// Resolve the issued instant, preferring the high-resolution
    /// `InstantBridge` field when present (T71 bridge).
    pub fn issued_at(&self) -> InstantBridge {
        self.issued_instant
            .unwrap_or_else(|| InstantBridge::from_coarse(self.issued_unix))
    }

    /// Attach the agent-in-capacity who issued the attestation.
    pub fn with_agent(mut self, agent: AgentInCapacity) -> Self {
        self.agent = Some(agent);
        self
    }

    /// Set the authority's department / branch.
    pub fn with_department(mut self, department: impl Into<String>) -> Self {
        self.authority.department = Some(department.into());
        self
    }

    /// Set the authority's jurisdiction (e.g. "AU", "AU-VIC").
    pub fn with_jurisdiction(mut self, jurisdiction: impl Into<String>) -> Self {
        self.authority.jurisdiction = Some(jurisdiction.into());
        self
    }

    /// Reference the content-addressed blob (PDF / credential bytes) for this attestation.
    pub fn with_blob(mut self, blob_hash: impl Into<String>) -> Self {
        self.blob_hash = Some(blob_hash.into());
        self
    }

    /// Set the delivery representation (Pdf / Credential / PdfWithEmbeddedCredential).
    pub fn with_representation(mut self, representation: Representation) -> Self {
        self.representation = representation;
        self
    }

    /// Mark whether the owner must act on this attestation.
    pub fn with_action_required(mut self, action_required: bool) -> Self {
        self.action_required = action_required;
        self
    }
}

/// The "government letter" specialization: an [`AuthorityAttestation`] whose authority is a
/// government (`authority_type::GOVERNMENT`), labelled from `sender`, delivered as a `Pdf`.
///
/// This makes the old `GovernmentLetter` just a preset of the general model. `subject` is used both
/// as the attestation subject and its statement/subject line (a government letter's "what it's
/// about" is its subject line).
pub fn government_letter(sender: &str, subject: &str, issued_unix: u32) -> AuthorityAttestation {
    let authority = Authority::new(authority_type::GOVERNMENT, sender);
    AuthorityAttestation::new(authority, subject, subject, issued_unix)
        .with_representation(Representation::Pdf)
}

pub fn authority_attestation_record_id(uuid: &str) -> String {
    format!("urn:wellfair:authority_attestation:{uuid}")
}

/// Build the Restricted envelope for an authority attestation.
///
/// Sensitivity is always `Restricted` (this is authority paperwork about the owner, not a public
/// disclosure). The epistemic status is `Asserted`. Evidence type is chosen by the authority: a
/// pathology / medical authority's attestation is `ClinicianObserved` (it is a clinician acting in
/// capacity), whereas any other authority is treated as `SelfReported` — the owner is asserting
/// "here is a letter I received", not making an independently-verified clinical claim. The
/// attestation's own `blob_hash` (the PDF / credential bytes) flows through to the envelope, and
/// validity anchors to when the attestation was issued. `proxy_did` is `None`.
pub fn build_authority_attestation_envelope(
    att: &AuthorityAttestation,
    owner_did: &str,
    author_did: &str,
    asserted_unix: u32,
) -> RecordEnvelope {
    let id = authority_attestation_record_id(&att.id);
    let evidence_type = if att.authority.is_medical() {
        EvidenceType::ClinicianObserved
    } else {
        EvidenceType::SelfReported
    };
    RecordEnvelope {
        id,
        owner_did: owner_did.to_string(),
        author_did: author_did.to_string(),
        proxy_did: None,
        epistemic_status: EpistemicStatus::Asserted,
        evidence_type,
        sensitivity: SensitivityClass::Restricted,
        asserted_time_unix: asserted_unix,
        asserted_instant: None,
        valid_time_start_unix: Some(att.issued_unix),
        valid_time_start_instant: att.issued_instant,
        valid_time_end_unix: None,
        valid_time_end_instant: None,
        predecessor_id: None,
        blob_hash: att.blob_hash.clone(),
        tombstone: false,
    }
}

/// A compact JSON projection of an attestation for journal display.
pub fn authority_attestation_summary(att: &AuthorityAttestation) -> String {
    serde_json::json!({
        "authority_label": att.authority.label,
        "authority_type": att.authority.type_id,
        "jurisdiction": att.authority.jurisdiction,
        "department": att.authority.department,
        "capacity": att.agent.as_ref().map(|a| a.capacity.clone()),
        "agent_name": att.agent.as_ref().map(|a| a.name.clone()),
        "representation": att.representation,
        "subject": att.subject,
        "statement": att.statement,
        "action_required": att.action_required,
        "issued_unix": att.issued_unix,
    })
    .to_string()
}

/// Parse a compact projection produced by [`authority_attestation_summary`] back into a value.
pub fn parse_authority_attestation_summary(
    summary: &str,
) -> Result<serde_json::Value, serde_json::Error> {
    serde_json::from_str(summary)
}

#[cfg(test)]
mod tests {
    use super::*;

    const OWNER: &str = "did:wf:owner";

    #[test]
    fn builder_round_trips_full_attestation() {
        let authority = Authority::new(authority_type::PATHOLOGY, "Acme Pathology");
        let agent = AgentInCapacity::new("Dr. Rao", "pathologist").with_did("did:wf:rao");
        let att = AuthorityAttestation::new(
            authority,
            "Blood panel result",
            "Full blood count within normal range",
            1_700_000_000,
        )
        .with_agent(agent.clone())
        .with_jurisdiction("AU-VIC")
        .with_department("Haematology")
        .with_representation(Representation::PdfWithEmbeddedCredential)
        .with_blob("blob-hash-1")
        .with_action_required(true);

        assert_eq!(att.authority.type_id, authority_type::PATHOLOGY);
        assert_eq!(att.authority.label, "Acme Pathology");
        assert_eq!(att.authority.jurisdiction.as_deref(), Some("AU-VIC"));
        assert_eq!(att.authority.department.as_deref(), Some("Haematology"));
        assert_eq!(att.agent, Some(agent));
        assert_eq!(
            att.representation,
            Representation::PdfWithEmbeddedCredential
        );
        assert_eq!(att.subject, "Blood panel result");
        assert!(att.action_required);
        assert_eq!(att.blob_hash.as_deref(), Some("blob-hash-1"));

        // Serde round-trip preserves every field.
        let json = serde_json::to_string(&att).unwrap();
        let back: AuthorityAttestation = serde_json::from_str(&json).unwrap();
        assert_eq!(att, back);
    }

    #[test]
    fn government_letter_is_a_preset_of_the_general_model() {
        let att = government_letter("Services Australia", "Claim outcome", 1_700_000_100);
        assert_eq!(att.authority.type_id, authority_type::GOVERNMENT);
        assert_eq!(att.authority.type_id, GOVERNMENT); // re-exported const
        assert!(att.authority.is_government());
        assert_eq!(att.authority.label, "Services Australia");
        assert_eq!(att.representation, Representation::Pdf);
        assert_eq!(att.subject, "Claim outcome");
        assert_eq!(att.statement, "Claim outcome");
        assert_eq!(att.issued_unix, 1_700_000_100);
        assert!(att.agent.is_none());
    }

    #[test]
    fn record_id_carries_authority_attestation_segment() {
        let att = government_letter("ATO", "Tax assessment", 1);
        let rid = authority_attestation_record_id(&att.id);
        assert!(rid.contains(":authority_attestation:"), "got {rid}");
    }

    #[test]
    fn envelope_is_restricted_and_threads_the_blob() {
        let att = government_letter("Centrelink", "Appointment notice", 1_700_002_000)
            .with_blob("letter-pdf-hash")
            .with_action_required(true);
        let env = build_authority_attestation_envelope(&att, OWNER, OWNER, 1_700_002_010);

        assert!(env.id.contains(":authority_attestation:"));
        assert_eq!(env.sensitivity, SensitivityClass::Restricted);
        assert_eq!(env.epistemic_status, EpistemicStatus::Asserted);
        // A government (non-medical) authority yields SelfReported evidence.
        assert_eq!(env.evidence_type, EvidenceType::SelfReported);
        assert_eq!(env.valid_time_start_unix, Some(1_700_002_000));
        assert_eq!(env.asserted_time_unix, 1_700_002_010);
        assert_eq!(env.proxy_did, None);
        // The attestation's blob flows through to the envelope.
        assert_eq!(env.blob_hash.as_deref(), Some("letter-pdf-hash"));
        assert!(!env.tombstone);
    }

    #[test]
    fn medical_authority_yields_clinician_observed_evidence() {
        // Exact PATHOLOGY const.
        let att = AuthorityAttestation::new(
            Authority::new(authority_type::PATHOLOGY, "Acme Pathology"),
            "Blood panel",
            "Result attached",
            1_700_003_000,
        );
        let env = build_authority_attestation_envelope(&att, OWNER, OWNER, 1_700_003_010);
        assert_eq!(env.evidence_type, EvidenceType::ClinicianObserved);
        assert_eq!(env.sensitivity, SensitivityClass::Restricted);

        // Caller-coined pathology sub-type is still recognised as clinician-grade.
        let att2 = AuthorityAttestation::new(
            Authority::new("authority:pathology:acme-labs", "Acme Labs"),
            "Culture result",
            "No growth",
            1_700_003_000,
        );
        assert!(att2.authority.is_medical());
        let env2 = build_authority_attestation_envelope(&att2, OWNER, OWNER, 1_700_003_010);
        assert_eq!(env2.evidence_type, EvidenceType::ClinicianObserved);
    }

    #[test]
    fn extensible_custom_authority_builds_with_named_agent_in_capacity() {
        // A pathology company (custom type id) with a named pathologist acting in capacity.
        let authority = Authority::new("authority:pathology:acme-labs", "Acme Labs Pty Ltd");
        let pathologist = AgentInCapacity::new("Dr. Elena Novak", "pathologist");
        let att = AuthorityAttestation::new(
            authority,
            "Histopathology report",
            "Specimen benign; no malignancy identified",
            1_700_004_000,
        )
        .with_agent(pathologist)
        .with_jurisdiction("AU")
        .with_representation(Representation::Credential)
        .with_blob("vc-blob-hash");

        assert_eq!(att.authority.type_id, "authority:pathology:acme-labs");
        assert!(att.authority.is_medical());
        assert!(!att.authority.is_government());
        let agent = att.agent.as_ref().unwrap();
        assert_eq!(agent.name, "Dr. Elena Novak");
        assert_eq!(agent.capacity, "pathologist");
        assert_eq!(att.representation, Representation::Credential);

        // Envelope reflects clinician evidence + the credential blob.
        let env = build_authority_attestation_envelope(&att, OWNER, OWNER, 1_700_004_010);
        assert_eq!(env.evidence_type, EvidenceType::ClinicianObserved);
        assert_eq!(env.blob_hash.as_deref(), Some("vc-blob-hash"));
    }

    #[test]
    fn summary_round_trips_through_the_parser() {
        let authority = Authority::new(authority_type::EDUCATION, "State University");
        let registrar = AgentInCapacity::new("J. Smith", "registrar");
        let att = AuthorityAttestation::new(
            authority,
            "Degree conferral",
            "Bachelor of Science conferred",
            1_700_005_000,
        )
        .with_agent(registrar)
        .with_jurisdiction("AU-NSW")
        .with_department("Registry")
        .with_representation(Representation::PdfWithEmbeddedCredential)
        .with_action_required(false);

        let summary = authority_attestation_summary(&att);
        // Representation serializes snake_case.
        assert!(
            summary.contains("pdf_with_embedded_credential"),
            "representation must be snake_case: {summary}"
        );

        let v = parse_authority_attestation_summary(&summary).unwrap();
        assert_eq!(
            v.get("authority_label").unwrap().as_str().unwrap(),
            "State University"
        );
        assert_eq!(
            v.get("authority_type").unwrap().as_str().unwrap(),
            authority_type::EDUCATION
        );
        assert_eq!(v.get("jurisdiction").unwrap().as_str().unwrap(), "AU-NSW");
        assert_eq!(v.get("department").unwrap().as_str().unwrap(), "Registry");
        assert_eq!(v.get("capacity").unwrap().as_str().unwrap(), "registrar");
        assert_eq!(v.get("agent_name").unwrap().as_str().unwrap(), "J. Smith");
        assert_eq!(
            v.get("representation").unwrap().as_str().unwrap(),
            "pdf_with_embedded_credential"
        );
        assert_eq!(
            v.get("subject").unwrap().as_str().unwrap(),
            "Degree conferral"
        );
        assert_eq!(v.get("action_required").unwrap().as_bool().unwrap(), false);
    }

    #[test]
    fn well_known_authority_type_consts_are_stable() {
        assert_eq!(authority_type::GOVERNMENT, "authority:government");
        assert_eq!(authority_type::PATHOLOGY, "authority:pathology");
        assert_eq!(authority_type::EDUCATION, "authority:education");
        assert_eq!(authority_type::FINANCIAL, "authority:financial");
        assert_eq!(authority_type::NGO, "authority:ngo");
    }
}
