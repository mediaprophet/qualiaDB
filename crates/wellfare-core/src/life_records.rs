//! Life events, welfare cases, and case tasks (LIF-01..17 records-first).

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::record::{EpistemicStatus, EvidenceType, RecordEnvelope, SensitivityClass};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaseStatus {
    Open,
    Paused,
    Closed,
}

impl Default for CaseStatus {
    fn default() -> Self {
        Self::Open
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LifeEventReport {
    pub id: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wellbeing_impact: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub occurred_at_unix: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

impl LifeEventReport {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            title: title.into(),
            event_type: None,
            wellbeing_impact: None,
            occurred_at_unix: None,
            notes: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WelfareCaseReport {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub status: CaseStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assistance_needs: Option<String>,
}

impl WelfareCaseReport {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            title: title.into(),
            status: CaseStatus::Open,
            summary: None,
            assistance_needs: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaseTaskReport {
    pub id: String,
    pub case_id: String,
    pub title: String,
    #[serde(default)]
    pub completed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub due_at_unix: Option<u32>,
}

impl CaseTaskReport {
    pub fn new(case_id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            case_id: case_id.into(),
            title: title.into(),
            completed: false,
            due_at_unix: None,
        }
    }
}

pub fn life_event_record_id(uuid: &str) -> String {
    format!("urn:wellfair:life_event:{uuid}")
}

pub fn welfare_case_record_id(uuid: &str) -> String {
    format!("urn:wellfair:welfare_case:{uuid}")
}

pub fn case_task_record_id(uuid: &str) -> String {
    format!("urn:wellfair:case_task:{uuid}")
}

fn life_envelope(
    id: &str,
    owner_did: &str,
    author_did: &str,
    asserted_unix: u32,
    blob_hash: Option<String>,
    sensitivity: SensitivityClass,
) -> RecordEnvelope {
    RecordEnvelope {
        id: id.to_string(),
        owner_did: owner_did.to_string(),
        author_did: author_did.to_string(),
        proxy_did: None,
        epistemic_status: EpistemicStatus::Asserted,
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

pub fn build_life_event_envelope(
    report: &LifeEventReport,
    owner_did: &str,
    author_did: &str,
    asserted_unix: u32,
    blob_hash: Option<String>,
) -> RecordEnvelope {
    life_envelope(
        &life_event_record_id(&report.id),
        owner_did,
        author_did,
        asserted_unix,
        blob_hash,
        SensitivityClass::Restricted,
    )
}

pub fn build_welfare_case_envelope(
    report: &WelfareCaseReport,
    owner_did: &str,
    author_did: &str,
    asserted_unix: u32,
    blob_hash: Option<String>,
) -> RecordEnvelope {
    life_envelope(
        &welfare_case_record_id(&report.id),
        owner_did,
        author_did,
        asserted_unix,
        blob_hash,
        SensitivityClass::Restricted,
    )
}

pub fn build_case_task_envelope(
    report: &CaseTaskReport,
    owner_did: &str,
    author_did: &str,
    asserted_unix: u32,
    blob_hash: Option<String>,
) -> RecordEnvelope {
    life_envelope(
        &case_task_record_id(&report.id),
        owner_did,
        author_did,
        asserted_unix,
        blob_hash,
        SensitivityClass::Restricted,
    )
}

pub fn life_event_summary(report: &LifeEventReport) -> String {
    serde_json::json!({
        "title": report.title,
        "event_type": report.event_type,
        "occurred_at_unix": report.occurred_at_unix,
    })
    .to_string()
}

pub fn welfare_case_summary(report: &WelfareCaseReport) -> String {
    serde_json::json!({
        "title": report.title,
        "status": report.status,
    })
    .to_string()
}

pub fn case_task_summary(report: &CaseTaskReport) -> String {
    serde_json::json!({
        "case_id": report.case_id,
        "title": report.title,
        "completed": report.completed,
        "due_at_unix": report.due_at_unix,
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn life_event_envelope_kind() {
        let report = LifeEventReport::new("Moved residence");
        let env = build_life_event_envelope(&report, "did:o", "did:a", 100, None);
        assert!(env.id.contains(":life_event:"));
        assert_eq!(env.sensitivity, SensitivityClass::Restricted);
    }

    #[test]
    fn welfare_case_is_restricted_sanctuary_kind() {
        let report = WelfareCaseReport::new("Housing assistance");
        let env = build_welfare_case_envelope(&report, "did:o", "did:a", 100, None);
        assert_eq!(env.sensitivity, SensitivityClass::Restricted);
    }
}
