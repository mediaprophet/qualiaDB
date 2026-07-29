//! Personal Core extensions — disputed diagnoses and housing/safety (PRO-01..08).

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::record::{EpistemicStatus, EvidenceType, RecordEnvelope, SensitivityClass};

/// Owner disputes or has not confirmed a diagnosis attributed to them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DisputedDiagnosisReport {
    pub id: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attributed_by: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dispute_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub supporting_notes: Option<String>,
}

impl DisputedDiagnosisReport {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            label: label.into(),
            attributed_by: None,
            dispute_reason: None,
            supporting_notes: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DwellingType {
    Fixed,
    Temporary,
    MobileShelter,
    Homeless,
    Unknown,
}

impl Default for DwellingType {
    fn default() -> Self {
        Self::Unknown
    }
}

/// Housing, safety hazards, and shelter context (Restricted — elevated fields in summary only).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HousingSafetyReport {
    pub id: String,
    #[serde(default)]
    pub dwelling_type: DwellingType,
    #[serde(default)]
    pub homelessness: bool,
    #[serde(default)]
    pub violence_concern: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hazards: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location_notes: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

impl HousingSafetyReport {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            dwelling_type: DwellingType::Unknown,
            homelessness: false,
            violence_concern: false,
            hazards: None,
            location_notes: None,
            notes: None,
        }
    }
}

pub fn disputed_diagnosis_record_id(uuid: &str) -> String {
    format!("urn:wellfair:disputed_diagnosis:{uuid}")
}

pub fn housing_safety_record_id(uuid: &str) -> String {
    format!("urn:wellfair:housing_safety:{uuid}")
}

pub fn journal_kind_for_personal_record_id(record_id: &str) -> Option<&'static str> {
    if record_id.contains(":disputed_diagnosis:") {
        Some("disputed_diagnosis")
    } else if record_id.contains(":housing_safety:") {
        Some("housing_safety")
    } else {
        None
    }
}

fn envelope_with_epistemic(
    id: &str,
    owner_did: &str,
    author_did: &str,
    asserted_unix: u32,
    blob_hash: Option<String>,
    epistemic: EpistemicStatus,
    sensitivity: SensitivityClass,
) -> RecordEnvelope {
    RecordEnvelope {
        id: id.to_string(),
        owner_did: owner_did.to_string(),
        author_did: author_did.to_string(),
        proxy_did: None,
        epistemic_status: epistemic,
        evidence_type: EvidenceType::SelfReported,
        sensitivity,
        asserted_time_unix: asserted_unix,
        valid_time_start_unix: Some(asserted_unix),
        valid_time_end_unix: None,
        predecessor_id: None,
        blob_hash,
        tombstone: false,
    }
}

pub fn build_disputed_diagnosis_envelope(
    report: &DisputedDiagnosisReport,
    owner_did: &str,
    author_did: &str,
    asserted_unix: u32,
    blob_hash: Option<String>,
) -> RecordEnvelope {
    let id = disputed_diagnosis_record_id(&report.id);
    envelope_with_epistemic(
        &id,
        owner_did,
        author_did,
        asserted_unix,
        blob_hash,
        EpistemicStatus::Disputed,
        SensitivityClass::Restricted,
    )
}

pub fn build_housing_safety_envelope(
    report: &HousingSafetyReport,
    owner_did: &str,
    author_did: &str,
    asserted_unix: u32,
    blob_hash: Option<String>,
) -> RecordEnvelope {
    let id = housing_safety_record_id(&report.id);
    envelope_with_epistemic(
        &id,
        owner_did,
        author_did,
        asserted_unix,
        blob_hash,
        EpistemicStatus::Asserted,
        SensitivityClass::Restricted,
    )
}

pub fn disputed_diagnosis_summary(report: &DisputedDiagnosisReport) -> String {
    serde_json::json!({
        "label": report.label,
        "epistemic": "disputed",
        "attributed_by": report.attributed_by,
        "dispute_reason": report.dispute_reason,
    })
    .to_string()
}

pub fn housing_safety_summary(report: &HousingSafetyReport) -> String {
    serde_json::json!({
        "dwelling_type": report.dwelling_type,
        "homelessness": report.homelessness,
        "violence_concern": report.violence_concern,
        "hazards": report.hazards,
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disputed_diagnosis_uses_disputed_epistemic() {
        let report = DisputedDiagnosisReport::new("Bipolar disorder");
        let env = build_disputed_diagnosis_envelope(
            &report,
            "did:wf:owner",
            "did:wf:owner",
            1_700_000_000,
            Some("abc".into()),
        );
        assert!(env.id.contains(":disputed_diagnosis:"));
        assert_eq!(env.epistemic_status, EpistemicStatus::Disputed);
        assert_eq!(
            journal_kind_for_personal_record_id(&env.id),
            Some("disputed_diagnosis")
        );
    }

    #[test]
    fn housing_safety_envelope_kind() {
        let report = HousingSafetyReport {
            id: "h1".into(),
            dwelling_type: DwellingType::MobileShelter,
            homelessness: true,
            violence_concern: false,
            hazards: Some("mould".into()),
            location_notes: None,
            notes: None,
        };
        let env = build_housing_safety_envelope(
            &report,
            "did:wf:owner",
            "did:wf:owner",
            1_700_000_000,
            None,
        );
        assert!(env.id.contains(":housing_safety:"));
        assert_eq!(env.sensitivity, SensitivityClass::Restricted);
        let summary = housing_safety_summary(&report);
        assert!(summary.contains("mobile_shelter") || summary.contains("MobileShelter"));
    }
}
