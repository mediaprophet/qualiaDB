//! Welfare support — assistance needs, welfare streams, and government letters.
//!
//! Cold-path Restricted profile records for the WellFair vault (audit LIF-08..14).
//! These capture the practical machinery of receiving support: the *needs* a person has
//! identified, the government/benefit *streams* they are applying for or receiving, and the
//! *letters* they get back. All three are owner self-reported, epistemically Asserted, and
//! held at Restricted sensitivity — this is welfare paperwork, not a public disclosure.
//!
//! This module is deliberately SEPARATE from `life_records.rs`; it owns its own record kinds
//! (`assistance_need`, `welfare_stream`, `government_letter`).

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::record::{EpistemicStatus, EvidenceType, RecordEnvelope, SensitivityClass};

/// How pressing an identified assistance need is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Urgency {
    Low,
    Moderate,
    High,
    Critical,
}

impl Default for Urgency {
    fn default() -> Self {
        Self::Moderate
    }
}

/// Lifecycle of a welfare / benefit stream from the owner's point of view.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StreamStatus {
    Applied,
    Active,
    Suspended,
    Ceased,
    Rejected,
}

impl Default for StreamStatus {
    fn default() -> Self {
        Self::Applied
    }
}

/// An assistance need the owner (or their proxy) has identified — e.g. "emergency housing",
/// "food relief", "help completing a form". This is the demand side of welfare support.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssistanceNeed {
    pub id: String,
    /// Broad classification of the need, e.g. "housing", "food", "legal", "medical".
    pub category: String,
    pub description: String,
    #[serde(default)]
    pub urgency: Urgency,
    pub identified_at_unix: u32,
}

impl AssistanceNeed {
    pub fn new(
        category: impl Into<String>,
        description: impl Into<String>,
        identified_at_unix: u32,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            category: category.into(),
            description: description.into(),
            urgency: Urgency::default(),
            identified_at_unix,
        }
    }
}

/// A welfare / benefit / assistance stream the owner is engaged with — a payment program, a
/// support service, a subsidy. `reference` is the program's own case/claim reference where one
/// exists. This is the supply side of welfare support.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WelfareStream {
    pub id: String,
    pub program_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    #[serde(default)]
    pub status: StreamStatus,
    pub started_at_unix: u32,
}

impl WelfareStream {
    pub fn new(program_name: impl Into<String>, started_at_unix: u32) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            program_name: program_name.into(),
            reference: None,
            status: StreamStatus::default(),
            started_at_unix,
        }
    }
}

/// A letter (or equivalent formal communication) received from a government body or agency.
/// The letter body itself lives out-of-band as a blob (`attachment_blob_hash`); this record is
/// the indexable envelope over it, flagging whether the owner must act on it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GovernmentLetter {
    pub id: String,
    pub sender: String,
    pub subject: String,
    pub received_at_unix: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attachment_blob_hash: Option<String>,
    #[serde(default)]
    pub action_required: bool,
}

impl GovernmentLetter {
    pub fn new(
        sender: impl Into<String>,
        subject: impl Into<String>,
        received_at_unix: u32,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            sender: sender.into(),
            subject: subject.into(),
            received_at_unix,
            attachment_blob_hash: None,
            action_required: false,
        }
    }
}

pub fn assistance_need_record_id(uuid: &str) -> String {
    format!("urn:wellfair:assistance_need:{uuid}")
}

pub fn welfare_stream_record_id(uuid: &str) -> String {
    format!("urn:wellfair:welfare_stream:{uuid}")
}

pub fn government_letter_record_id(uuid: &str) -> String {
    format!("urn:wellfair:government_letter:{uuid}")
}

/// Shared Restricted / SelfReported / Asserted envelope for welfare-support records.
fn self_reported_envelope(
    id: &str,
    owner_did: &str,
    author_did: &str,
    asserted_unix: u32,
    valid_time_start_unix: u32,
    blob_hash: Option<String>,
) -> RecordEnvelope {
    RecordEnvelope {
        id: id.to_string(),
        owner_did: owner_did.to_string(),
        author_did: author_did.to_string(),
        proxy_did: None,
        epistemic_status: EpistemicStatus::Asserted,
        evidence_type: EvidenceType::SelfReported,
        sensitivity: SensitivityClass::Restricted,
        asserted_time_unix: asserted_unix,
        valid_time_start_unix: Some(valid_time_start_unix),
        valid_time_end_unix: None,
        predecessor_id: None,
        blob_hash,
        tombstone: false,
    }
}

pub fn build_assistance_need_envelope(
    need: &AssistanceNeed,
    owner_did: &str,
    author_did: &str,
    asserted_unix: u32,
    blob_hash: Option<String>,
) -> RecordEnvelope {
    let id = assistance_need_record_id(&need.id);
    self_reported_envelope(
        &id,
        owner_did,
        author_did,
        asserted_unix,
        need.identified_at_unix,
        blob_hash,
    )
}

pub fn build_welfare_stream_envelope(
    stream: &WelfareStream,
    owner_did: &str,
    author_did: &str,
    asserted_unix: u32,
    blob_hash: Option<String>,
) -> RecordEnvelope {
    let id = welfare_stream_record_id(&stream.id);
    self_reported_envelope(
        &id,
        owner_did,
        author_did,
        asserted_unix,
        stream.started_at_unix,
        blob_hash,
    )
}

pub fn build_government_letter_envelope(
    letter: &GovernmentLetter,
    owner_did: &str,
    author_did: &str,
    asserted_unix: u32,
) -> RecordEnvelope {
    let id = government_letter_record_id(&letter.id);
    // The letter's own attachment is the blob; validity starts when it was received.
    self_reported_envelope(
        &id,
        owner_did,
        author_did,
        asserted_unix,
        letter.received_at_unix,
        letter.attachment_blob_hash.clone(),
    )
}

pub fn assistance_need_summary(need: &AssistanceNeed) -> String {
    serde_json::json!({
        "category": need.category,
        "description": need.description,
        "urgency": need.urgency,
        "identified_at_unix": need.identified_at_unix,
    })
    .to_string()
}

pub fn welfare_stream_summary(stream: &WelfareStream) -> String {
    serde_json::json!({
        "program_name": stream.program_name,
        "reference": stream.reference,
        "status": stream.status,
        "started_at_unix": stream.started_at_unix,
    })
    .to_string()
}

pub fn government_letter_summary(letter: &GovernmentLetter) -> String {
    serde_json::json!({
        "sender": letter.sender,
        "subject": letter.subject,
        "received_at_unix": letter.received_at_unix,
        "attachment_blob_hash": letter.attachment_blob_hash,
        "action_required": letter.action_required,
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    const OWNER: &str = "did:wf:owner";

    #[test]
    fn urgency_and_stream_status_defaults() {
        assert_eq!(Urgency::default(), Urgency::Moderate);
        assert_eq!(StreamStatus::default(), StreamStatus::Applied);

        // Builders adopt the defaults.
        let need = AssistanceNeed::new("housing", "Emergency accommodation", 1_700_000_000);
        assert_eq!(need.urgency, Urgency::Moderate);

        let stream = WelfareStream::new("JobSeeker Payment", 1_700_000_000);
        assert_eq!(stream.status, StreamStatus::Applied);
        assert_eq!(stream.reference, None);

        let letter = GovernmentLetter::new("Services Australia", "Claim outcome", 1_700_000_000);
        assert!(!letter.action_required);
        assert_eq!(letter.attachment_blob_hash, None);
    }

    #[test]
    fn assistance_need_envelope_kind_and_class() {
        let need = AssistanceNeed::new("food", "Weekly food relief", 1_700_000_100);
        let env = build_assistance_need_envelope(&need, OWNER, OWNER, 1_700_000_200, None);
        assert!(env.id.contains(":assistance_need:"));
        assert_eq!(env.evidence_type, EvidenceType::SelfReported);
        assert_eq!(env.epistemic_status, EpistemicStatus::Asserted);
        assert_eq!(env.sensitivity, SensitivityClass::Restricted);
        // Validity anchors to when the need was identified.
        assert_eq!(env.valid_time_start_unix, Some(1_700_000_100));
        assert_eq!(env.asserted_time_unix, 1_700_000_200);
        assert!(!env.tombstone);
    }

    #[test]
    fn welfare_stream_envelope_kind_and_class() {
        let stream = WelfareStream::new("Disability Support Pension", 1_700_001_000);
        let env = build_welfare_stream_envelope(
            &stream,
            OWNER,
            OWNER,
            1_700_001_050,
            Some("blobref".into()),
        );
        assert!(env.id.contains(":welfare_stream:"));
        assert_eq!(env.evidence_type, EvidenceType::SelfReported);
        assert_eq!(env.sensitivity, SensitivityClass::Restricted);
        assert_eq!(env.valid_time_start_unix, Some(1_700_001_000));
        assert_eq!(env.blob_hash.as_deref(), Some("blobref"));
    }

    #[test]
    fn government_letter_envelope_uses_attachment_as_blob() {
        let mut letter = GovernmentLetter::new("Centrelink", "Appointment notice", 1_700_002_000);
        letter.attachment_blob_hash = Some("letter-pdf-hash".into());
        letter.action_required = true;
        let env = build_government_letter_envelope(&letter, OWNER, OWNER, 1_700_002_010);
        assert!(env.id.contains(":government_letter:"));
        assert_eq!(env.evidence_type, EvidenceType::SelfReported);
        assert_eq!(env.sensitivity, SensitivityClass::Restricted);
        assert_eq!(env.valid_time_start_unix, Some(1_700_002_000));
        // The letter's attachment flows through to the envelope blob reference.
        assert_eq!(env.blob_hash.as_deref(), Some("letter-pdf-hash"));
    }

    #[test]
    fn summaries_include_primary_fields() {
        let mut need = AssistanceNeed::new("legal", "Tenancy dispute advice", 1_700_000_000);
        need.urgency = Urgency::Critical;
        let s = assistance_need_summary(&need);
        assert!(s.contains("legal"));
        assert!(s.contains("Tenancy dispute advice"));
        assert!(
            s.contains("critical"),
            "urgency must serialize snake_case: {s}"
        );

        let mut stream = WelfareStream::new("Rent Assistance", 1_700_000_000);
        stream.reference = Some("CRN-12345".into());
        stream.status = StreamStatus::Active;
        let s = welfare_stream_summary(&stream);
        assert!(s.contains("Rent Assistance"));
        assert!(s.contains("CRN-12345"));
        assert!(
            s.contains("active"),
            "status must serialize snake_case: {s}"
        );

        let mut letter = GovernmentLetter::new("ATO", "Tax assessment", 1_700_000_000);
        letter.action_required = true;
        let s = government_letter_summary(&letter);
        assert!(s.contains("ATO"));
        assert!(s.contains("Tax assessment"));
        assert!(s.contains("true"), "action_required must be present: {s}");
    }

    #[test]
    fn summaries_round_trip_through_serde() {
        let need = AssistanceNeed::new("medical", "GP appointment help", 1_700_000_000);
        let json = assistance_need_summary(&need);
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v.get("category").unwrap().as_str().unwrap(), "medical");
        assert_eq!(v.get("urgency").unwrap().as_str().unwrap(), "moderate");
    }

    #[test]
    fn record_ids_carry_unique_kind_segments() {
        let need = AssistanceNeed::new("housing", "d", 1);
        let stream = WelfareStream::new("p", 1);
        let letter = GovernmentLetter::new("s", "subj", 1);
        assert!(assistance_need_record_id(&need.id).contains(":assistance_need:"));
        assert!(welfare_stream_record_id(&stream.id).contains(":welfare_stream:"));
        assert!(government_letter_record_id(&letter.id).contains(":government_letter:"));
    }
}
