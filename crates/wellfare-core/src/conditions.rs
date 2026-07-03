//! Self-reported conditions and allergies — Restricted profile records for the WellFair vault.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::record::{
    EpistemicStatus, EvidenceType, RecordEnvelope, SensitivityClass,
};

/// Lifecycle of a self-reported condition entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConditionStatus {
    Active,
    Resolved,
    InRemission,
}

impl Default for ConditionStatus {
    fn default() -> Self {
        Self::Active
    }
}

/// Owner-reported diagnosis or chronic condition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConditionReport {
    pub id: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icd10_code: Option<String>,
    #[serde(default)]
    pub status: ConditionStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

impl ConditionReport {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            label: label.into(),
            icd10_code: None,
            status: ConditionStatus::Active,
            notes: None,
        }
    }
}

/// Owner-reported allergy or intolerance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AllergyReport {
    pub id: String,
    pub substance: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reaction: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub severity: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

impl AllergyReport {
    pub fn new(substance: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            substance: substance.into(),
            reaction: None,
            severity: None,
            notes: None,
        }
    }
}

pub fn condition_record_id(uuid: &str) -> String {
    format!("urn:wellfair:condition:{uuid}")
}

pub fn allergy_record_id(uuid: &str) -> String {
    format!("urn:wellfair:allergy:{uuid}")
}

/// Infer journal projection kind from a WellFair record id.
pub fn journal_kind_for_record_id(record_id: &str) -> &'static str {
    if record_id.contains(":condition:") {
        "condition"
    } else if record_id.contains(":allergy:") {
        "allergy"
    } else if record_id.contains(":disputed_diagnosis:") {
        "disputed_diagnosis"
    } else if record_id.contains(":housing_safety:") {
        "housing_safety"
    } else if record_id.contains(":life_event:") {
        "life_event"
    } else if record_id.contains(":welfare_case:") {
        "welfare_case"
    } else if record_id.contains(":case_task:") {
        "case_task"
    } else if record_id.contains(":wellbeing_observation:") {
        "wellbeing_observation"
    } else if record_id.contains(":therapy_note:") {
        "therapy_note"
    } else if record_id.contains(":sanctuary_note:") {
        "sanctuary_note"
    } else if record_id.contains(":medication:") {
        "medication"
    } else if record_id.contains(":med_admin:") {
        "med_administration"
    } else if record_id.contains(":diet:") {
        "diet"
    } else if record_id.contains(":ledger_entry:") {
        "ledger_entry"
    } else if record_id.contains(":credential:") {
        "credential"
    } else if record_id.contains(":project_membership:") {
        "project_membership"
    } else if record_id.contains(":project:") {
        "project"
    } else if record_id.contains(":contribution:") {
        "contribution"
    } else if record_id.contains(":clinical_report:") {
        "clinical_report"
    } else if record_id.contains(":clinical_attachment:") {
        "clinical_attachment"
    } else if record_id.contains(":assistance_need:") {
        "assistance_need"
    } else if record_id.contains(":welfare_stream:") {
        "welfare_stream"
    } else if record_id.contains(":government_letter:") {
        "government_letter"
    } else if record_id.contains(":work_item_status:") {
        "work_item_status"
    } else if record_id.contains(":work_item:") {
        "work_item"
    } else if record_id.contains(":weight:") {
        "weight"
    } else if record_id.contains(":sleep:") {
        "sleep"
    } else if record_id.contains(":steps:") {
        "steps"
    } else if record_id.contains(":heart_rate:") {
        "heart_rate"
    } else {
        "record"
    }
}

fn self_reported_envelope(
    id: &str,
    owner_did: &str,
    author_did: &str,
    asserted_unix: u32,
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
        valid_time_start_unix: Some(asserted_unix),
        valid_time_end_unix: None,
        predecessor_id: None,
        blob_hash,
        tombstone: false,
    }
}

pub fn build_condition_envelope(
    report: &ConditionReport,
    owner_did: &str,
    author_did: &str,
    asserted_unix: u32,
    blob_hash: Option<String>,
) -> RecordEnvelope {
    let id = condition_record_id(&report.id);
    self_reported_envelope(&id, owner_did, author_did, asserted_unix, blob_hash)
}

pub fn build_allergy_envelope(
    report: &AllergyReport,
    owner_did: &str,
    author_did: &str,
    asserted_unix: u32,
    blob_hash: Option<String>,
) -> RecordEnvelope {
    let id = allergy_record_id(&report.id);
    self_reported_envelope(&id, owner_did, author_did, asserted_unix, blob_hash)
}

pub fn condition_summary(report: &ConditionReport) -> String {
    serde_json::json!({
        "label": report.label,
        "status": report.status,
        "icd10": report.icd10_code,
    })
    .to_string()
}

pub fn allergy_summary(report: &AllergyReport) -> String {
    serde_json::json!({
        "substance": report.substance,
        "reaction": report.reaction,
        "severity": report.severity,
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn condition_envelope_is_self_reported_restricted() {
        let report = ConditionReport::new("Type 2 diabetes");
        let env = build_condition_envelope(
            &report,
            "did:wf:owner",
            "did:wf:owner",
            1_700_000_000,
            Some("abc123".into()),
        );
        assert!(env.id.contains(":condition:"));
        assert_eq!(env.evidence_type, EvidenceType::SelfReported);
        assert_eq!(env.sensitivity, SensitivityClass::Restricted);
        assert_eq!(journal_kind_for_record_id(&env.id), "condition");
    }

    #[test]
    fn allergy_envelope_kind() {
        let report = AllergyReport::new("Peanuts");
        let env = build_allergy_envelope(
            &report,
            "did:wf:owner",
            "did:wf:owner",
            1_700_000_000,
            None,
        );
        assert!(env.id.contains(":allergy:"));
        assert_eq!(journal_kind_for_record_id(&env.id), "allergy");
    }

    #[test]
    fn summaries_include_primary_fields() {
        let cond = ConditionReport {
            id: "c1".into(),
            label: "Asthma".into(),
            icd10_code: Some("J45".into()),
            status: ConditionStatus::Active,
            notes: None,
        };
        let summary = condition_summary(&cond);
        assert!(summary.contains("Asthma"));
        assert!(summary.contains("J45"));

        let allergy = AllergyReport {
            id: "a1".into(),
            substance: "Latex".into(),
            reaction: Some("Rash".into()),
            severity: Some("moderate".into()),
            notes: None,
        };
        let allergy_summary = allergy_summary(&allergy);
        assert!(allergy_summary.contains("Latex"));
        assert!(allergy_summary.contains("Rash"));
    }
}