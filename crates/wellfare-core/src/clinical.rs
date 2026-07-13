//! Clinical documents / pathology — Restricted, records-first clinical entries.
//!
//! Implements the master plan's clinical-document rules (audit CLI-01..13, plan §5):
//!
//! - **Records-first, manual entry.** A `ClinicalReport` captures what the owner (or a
//!   labelled author) actually typed. There is **NO parsing of pathology/imaging PDFs
//!   here** — a mock extractor must never be presented as real parsing. Structured
//!   extraction, when it exists, is a separate audited stage that raises its own records;
//!   this module only stores hand-entered documents and their attachment metadata.
//! - **Content-addressed attachments (plan §5).** The attachment *content* is a
//!   content-addressed blob stored elsewhere. `AttachmentMeta` holds only metadata plus
//!   the content hash — never the bytes.
//! - **Honest epistemics.** An unconfirmed report is *asserted by its author*, not
//!   observed by a clinician. Only a `ClinicianConfirmed` report maps to
//!   `EvidenceType::ClinicianObserved`; a `Disputed` report is surfaced as
//!   `EpistemicStatus::Disputed`. The envelope never launders a self-reported claim into
//!   clinician-grade evidence.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::record::{EpistemicStatus, EvidenceType, RecordEnvelope, SensitivityClass};

/// Category of a manually-entered clinical document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClinicalReportType {
    Pathology,
    Imaging,
    Discharge,
    Referral,
    Other,
}

impl Default for ClinicalReportType {
    fn default() -> Self {
        Self::Other
    }
}

/// Claim-approval lifecycle for a clinical document.
///
/// A report starts life as an owner/author claim and only becomes clinician-grade
/// evidence once a clinician confirms it. The transitions are deliberately narrow so the
/// state can never silently upgrade its own trustworthiness (see [`advance_claim`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaimStatus {
    /// Being written; not yet put forward for confirmation.
    Draft,
    /// Put forward by the author, awaiting clinician review.
    Submitted,
    /// A clinician has confirmed the document as accurate.
    ClinicianConfirmed,
    /// A clinician (or the owner) disputes the document's accuracy.
    Disputed,
    /// Replaced by a newer report (a correction / re-issue).
    Superseded,
}

impl Default for ClaimStatus {
    fn default() -> Self {
        Self::Draft
    }
}

/// Events that can move a [`ClaimStatus`] along its lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaimEvent {
    /// Author submits a draft for clinician review.
    Submit,
    /// A clinician confirms a submitted report.
    Confirm,
    /// A clinician / owner disputes the report.
    Dispute,
    /// A newer report replaces this one.
    Supersede,
    /// A disputed report is sent back to the author for revision.
    Revise,
}

/// Advance the claim lifecycle for a given event, returning the resulting status.
///
/// Transition table (any pair not listed is a **no-op**, i.e. the current status is
/// returned unchanged — invalid moves never silently succeed):
///
/// | From                 | Event      | To                   |
/// |----------------------|------------|----------------------|
/// | `Draft`              | `Submit`   | `Submitted`          |
/// | `Submitted`          | `Confirm`  | `ClinicianConfirmed` |
/// | `Submitted`          | `Dispute`  | `Disputed`           |
/// | `ClinicianConfirmed` | `Dispute`  | `Disputed`           |
/// | `ClinicianConfirmed` | `Supersede`| `Superseded`         |
/// | `Disputed`           | `Confirm`  | `ClinicianConfirmed` |
/// | `Disputed`           | `Revise`   | `Draft`              |
/// | `Disputed`           | `Supersede`| `Superseded`         |
/// | `Submitted`          | `Supersede`| `Superseded`         |
///
/// In particular a `Draft` can **never** jump straight to `ClinicianConfirmed`: a
/// `Confirm` event on a `Draft` is a no-op, so a document must always pass through
/// `Submitted` (author submission) before a clinician can confirm it.
pub fn advance_claim(status: ClaimStatus, event: ClaimEvent) -> ClaimStatus {
    use ClaimEvent::*;
    use ClaimStatus::*;
    match (status, event) {
        (Draft, Submit) => Submitted,
        (Submitted, Confirm) => ClinicianConfirmed,
        (Submitted, Dispute) => Disputed,
        (Submitted, Supersede) => Superseded,
        (ClinicianConfirmed, Dispute) => Disputed,
        (ClinicianConfirmed, Supersede) => Superseded,
        (Disputed, Confirm) => ClinicianConfirmed,
        (Disputed, Revise) => Draft,
        (Disputed, Supersede) => Superseded,
        // Any other (status, event) pair is not a legal transition: no-op.
        (other, _) => other,
    }
}

/// A manually-entered clinical document (pathology / imaging / discharge / referral).
///
/// `body` is exactly what the author typed. `attachment_blob_hash` optionally links to a
/// content-addressed blob (the original PDF/image) whose metadata lives in
/// [`AttachmentMeta`]; the bytes are never stored in this struct.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClinicalReport {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub report_type: ClinicalReportType,
    /// Free-text label for the author (e.g. "Dr Smith, Pathology") — NOT a verified DID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author_label: Option<String>,
    /// When the clinical event / observation occurred (unix seconds).
    pub observed_at_unix: u32,
    /// The document body as typed by the author. No parsing is performed on it.
    pub body: String,
    /// Optional content hash of the attached source document blob.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attachment_blob_hash: Option<String>,
    #[serde(default)]
    pub claim_status: ClaimStatus,
}

impl ClinicalReport {
    pub fn new(
        title: impl Into<String>,
        report_type: ClinicalReportType,
        observed_at_unix: u32,
        body: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            title: title.into(),
            report_type,
            author_label: None,
            observed_at_unix,
            body: body.into(),
            attachment_blob_hash: None,
            claim_status: ClaimStatus::Draft,
        }
    }
}

/// Metadata for a content-addressed attachment blob (plan §5).
///
/// The blob content itself is stored out-of-band and referenced by `content_hash`; this
/// struct records only the descriptive metadata needed to present and verify it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttachmentMeta {
    pub id: String,
    pub filename: String,
    /// MIME/media type, e.g. "application/pdf" or "image/png".
    pub media_type: String,
    pub byte_len: u64,
    /// Content-address of the blob (e.g. a hex digest). This is the load-bearing anchor.
    pub content_hash: String,
}

impl AttachmentMeta {
    pub fn new(
        filename: impl Into<String>,
        media_type: impl Into<String>,
        byte_len: u64,
        content_hash: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            filename: filename.into(),
            media_type: media_type.into(),
            byte_len,
            content_hash: content_hash.into(),
        }
    }
}

pub fn clinical_report_record_id(uuid: &str) -> String {
    format!("urn:wellfair:clinical_report:{uuid}")
}

pub fn clinical_attachment_record_id(uuid: &str) -> String {
    format!("urn:wellfair:clinical_attachment:{uuid}")
}

/// Map a claim status to the epistemic status the envelope should honestly carry.
///
/// Only a clinician-confirmed report is anything other than an author assertion, and a
/// disputed report is surfaced as `Disputed`. A superseded document keeps whatever it was
/// (a superseded correction is still, at heart, an assertion) — its replacement is tracked
/// via the envelope `predecessor_id`, not by re-labelling epistemics.
fn epistemic_for_claim(status: ClaimStatus) -> EpistemicStatus {
    match status {
        ClaimStatus::Disputed => EpistemicStatus::Disputed,
        // Confirmation is about *evidence grade* (clinician-observed), not about the
        // proposition being a hypothesis; a confirmed report is still an assertion.
        ClaimStatus::Draft
        | ClaimStatus::Submitted
        | ClaimStatus::ClinicianConfirmed
        | ClaimStatus::Superseded => EpistemicStatus::Asserted,
    }
}

/// Map a claim status to the evidence type. Crucially, a report only counts as
/// `ClinicianObserved` once it is `ClinicianConfirmed`; everything else is
/// `SelfReported`, because it is an author's claim until a clinician confirms it.
fn evidence_for_claim(status: ClaimStatus) -> EvidenceType {
    match status {
        ClaimStatus::ClinicianConfirmed => EvidenceType::ClinicianObserved,
        _ => EvidenceType::SelfReported,
    }
}

/// Build the record envelope for a clinical report, deriving epistemic/evidence honesty
/// from its claim status. Always `Restricted`.
pub fn build_clinical_report_envelope(
    report: &ClinicalReport,
    owner_did: &str,
    author_did: &str,
    asserted_unix: u32,
    blob_hash: Option<String>,
) -> RecordEnvelope {
    RecordEnvelope {
        id: clinical_report_record_id(&report.id),
        owner_did: owner_did.to_string(),
        author_did: author_did.to_string(),
        proxy_did: None,
        epistemic_status: epistemic_for_claim(report.claim_status),
        evidence_type: evidence_for_claim(report.claim_status),
        sensitivity: SensitivityClass::Restricted,
        asserted_time_unix: asserted_unix,
        valid_time_start_unix: Some(report.observed_at_unix),
        valid_time_end_unix: None,
        predecessor_id: None,
        blob_hash,
        tombstone: false,
    }
}

/// Build the record envelope for an attachment's metadata. The blob hash is carried both
/// as the content anchor and (redundantly for the graph) as the envelope `blob_hash`.
pub fn build_clinical_attachment_envelope(
    meta: &AttachmentMeta,
    owner_did: &str,
    author_did: &str,
    asserted_unix: u32,
) -> RecordEnvelope {
    RecordEnvelope {
        id: clinical_attachment_record_id(&meta.id),
        owner_did: owner_did.to_string(),
        author_did: author_did.to_string(),
        proxy_did: None,
        // Attachment metadata describes a stored file; it is an asserted, self-reported fact.
        epistemic_status: EpistemicStatus::Asserted,
        evidence_type: EvidenceType::SelfReported,
        sensitivity: SensitivityClass::Restricted,
        asserted_time_unix: asserted_unix,
        valid_time_start_unix: Some(asserted_unix),
        valid_time_end_unix: None,
        predecessor_id: None,
        blob_hash: Some(meta.content_hash.clone()),
        tombstone: false,
    }
}

pub fn clinical_report_summary(report: &ClinicalReport) -> String {
    serde_json::json!({
        "title": report.title,
        "report_type": report.report_type,
        "author_label": report.author_label,
        "observed_at_unix": report.observed_at_unix,
        "claim_status": report.claim_status,
        "has_attachment": report.attachment_blob_hash.is_some(),
    })
    .to_string()
}

pub fn clinical_attachment_summary(meta: &AttachmentMeta) -> String {
    serde_json::json!({
        "filename": meta.filename,
        "media_type": meta.media_type,
        "byte_len": meta.byte_len,
        "content_hash": meta.content_hash,
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_envelope_kind_and_sensitivity() {
        let report = ClinicalReport::new(
            "Full blood count",
            ClinicalReportType::Pathology,
            1_700_000_000,
            "Hb 140 g/L; WCC 6.2",
        );
        let env = build_clinical_report_envelope(
            &report,
            "did:wf:owner",
            "did:wf:owner",
            1_700_000_500,
            Some("blobhash".into()),
        );
        assert!(env.id.contains(":clinical_report:"));
        assert_eq!(env.sensitivity, SensitivityClass::Restricted);
        // valid_time_start tracks the clinical observation time, not assertion time.
        assert_eq!(env.valid_time_start_unix, Some(1_700_000_000));
    }

    #[test]
    fn attachment_envelope_kind_and_hash() {
        let meta = AttachmentMeta::new(
            "path_report.pdf",
            "application/pdf",
            2048,
            "deadbeef",
        );
        let env = build_clinical_attachment_envelope(
            &meta,
            "did:wf:owner",
            "did:wf:owner",
            1_700_000_000,
        );
        assert!(env.id.contains(":clinical_attachment:"));
        assert_eq!(env.sensitivity, SensitivityClass::Restricted);
        assert_eq!(env.blob_hash.as_deref(), Some("deadbeef"));
    }

    #[test]
    fn unconfirmed_report_is_self_reported_not_clinician_observed() {
        // A draft / submitted report is the author's claim, NOT clinician evidence.
        let mut report = ClinicalReport::new(
            "MRI brain",
            ClinicalReportType::Imaging,
            10,
            "No acute findings.",
        );
        for status in [ClaimStatus::Draft, ClaimStatus::Submitted, ClaimStatus::Superseded] {
            report.claim_status = status;
            let env = build_clinical_report_envelope(&report, "did:wf:o", "did:wf:o", 20, None);
            assert_eq!(
                env.evidence_type,
                EvidenceType::SelfReported,
                "status {status:?} must not be clinician-observed"
            );
            assert_eq!(env.epistemic_status, EpistemicStatus::Asserted);
        }
    }

    #[test]
    fn confirmed_report_maps_to_clinician_observed() {
        let mut report = ClinicalReport::new("Biopsy", ClinicalReportType::Pathology, 10, "benign");
        report.claim_status = ClaimStatus::ClinicianConfirmed;
        let env = build_clinical_report_envelope(&report, "did:wf:o", "did:wf:o", 20, None);
        assert_eq!(env.evidence_type, EvidenceType::ClinicianObserved);
        assert_eq!(env.epistemic_status, EpistemicStatus::Asserted);
    }

    #[test]
    fn disputed_report_is_surfaced_as_disputed() {
        let mut report = ClinicalReport::new("Referral", ClinicalReportType::Referral, 10, "…");
        report.claim_status = ClaimStatus::Disputed;
        let env = build_clinical_report_envelope(&report, "did:wf:o", "did:wf:o", 20, None);
        assert_eq!(env.epistemic_status, EpistemicStatus::Disputed);
        // Disputed is still the author's claim in terms of evidence grade.
        assert_eq!(env.evidence_type, EvidenceType::SelfReported);
    }

    #[test]
    fn draft_cannot_jump_to_clinician_confirmed_without_submission() {
        // Confirm on a Draft is a no-op — the document must be Submitted first.
        assert_eq!(advance_claim(ClaimStatus::Draft, ClaimEvent::Confirm), ClaimStatus::Draft);
        // The legal path: Draft --submit--> Submitted --confirm--> ClinicianConfirmed.
        let submitted = advance_claim(ClaimStatus::Draft, ClaimEvent::Submit);
        assert_eq!(submitted, ClaimStatus::Submitted);
        let confirmed = advance_claim(submitted, ClaimEvent::Confirm);
        assert_eq!(confirmed, ClaimStatus::ClinicianConfirmed);
    }

    #[test]
    fn claim_lifecycle_transitions() {
        // Dispute from confirmed, then revise back to draft, then round-trip again.
        let confirmed = ClaimStatus::ClinicianConfirmed;
        let disputed = advance_claim(confirmed, ClaimEvent::Dispute);
        assert_eq!(disputed, ClaimStatus::Disputed);
        let redraft = advance_claim(disputed, ClaimEvent::Revise);
        assert_eq!(redraft, ClaimStatus::Draft);

        // Supersede a confirmed report.
        assert_eq!(
            advance_claim(ClaimStatus::ClinicianConfirmed, ClaimEvent::Supersede),
            ClaimStatus::Superseded
        );
        // A disputed report can be directly confirmed by a clinician.
        assert_eq!(
            advance_claim(ClaimStatus::Disputed, ClaimEvent::Confirm),
            ClaimStatus::ClinicianConfirmed
        );
        // Superseded is terminal: any event leaves it unchanged.
        assert_eq!(
            advance_claim(ClaimStatus::Superseded, ClaimEvent::Confirm),
            ClaimStatus::Superseded
        );
        assert_eq!(
            advance_claim(ClaimStatus::Superseded, ClaimEvent::Submit),
            ClaimStatus::Superseded
        );
    }

    #[test]
    fn report_summary_round_trips() {
        let mut report = ClinicalReport::new(
            "Discharge summary",
            ClinicalReportType::Discharge,
            1_700_000_000,
            "Discharged home.",
        );
        report.author_label = Some("Ward 3".into());
        report.attachment_blob_hash = Some("hash123".into());
        report.claim_status = ClaimStatus::Submitted;

        let summary = clinical_report_summary(&report);
        let v: serde_json::Value = serde_json::from_str(&summary).unwrap();
        assert_eq!(v["title"], "Discharge summary");
        assert_eq!(v["report_type"], "discharge");
        assert_eq!(v["claim_status"], "submitted");
        assert_eq!(v["author_label"], "Ward 3");
        assert_eq!(v["has_attachment"], true);
        assert_eq!(v["observed_at_unix"], 1_700_000_000u32);
    }

    #[test]
    fn attachment_summary_round_trips() {
        let meta = AttachmentMeta::new("scan.png", "image/png", 4096, "cafebabe");
        let summary = clinical_attachment_summary(&meta);
        let v: serde_json::Value = serde_json::from_str(&summary).unwrap();
        assert_eq!(v["filename"], "scan.png");
        assert_eq!(v["media_type"], "image/png");
        assert_eq!(v["byte_len"], 4096u64);
        assert_eq!(v["content_hash"], "cafebabe");
    }

    #[test]
    fn report_type_defaults_to_other() {
        assert_eq!(ClinicalReportType::default(), ClinicalReportType::Other);
        assert_eq!(ClaimStatus::default(), ClaimStatus::Draft);
    }
}
