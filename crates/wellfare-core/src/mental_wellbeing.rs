//! Mental wellbeing observations and therapy notes (MHT — no licensed instruments).

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::record::{
    EpistemicStatus, EvidenceType, RecordEnvelope, SensitivityClass,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WellbeingObservation {
    pub id: String,
    pub mood_label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intensity_1_10: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

impl WellbeingObservation {
    pub fn new(mood_label: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            mood_label: mood_label.into(),
            intensity_1_10: None,
            notes: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TherapyNote {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_date_unix: Option<u32>,
    pub notes: String,
}

impl TherapyNote {
    pub fn new(notes: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            provider_label: None,
            session_date_unix: None,
            notes: notes.into(),
        }
    }
}

pub fn wellbeing_observation_record_id(uuid: &str) -> String {
    format!("urn:wellfair:wellbeing_observation:{uuid}")
}

pub fn therapy_note_record_id(uuid: &str) -> String {
    format!("urn:wellfair:therapy_note:{uuid}")
}

fn wellbeing_envelope(
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

pub fn build_wellbeing_observation_envelope(
    report: &WellbeingObservation,
    owner_did: &str,
    author_did: &str,
    asserted_unix: u32,
    blob_hash: Option<String>,
) -> RecordEnvelope {
    wellbeing_envelope(
        &wellbeing_observation_record_id(&report.id),
        owner_did,
        author_did,
        asserted_unix,
        blob_hash,
        SensitivityClass::Restricted,
    )
}

pub fn build_therapy_note_envelope(
    report: &TherapyNote,
    owner_did: &str,
    author_did: &str,
    asserted_unix: u32,
    blob_hash: Option<String>,
) -> RecordEnvelope {
    wellbeing_envelope(
        &therapy_note_record_id(&report.id),
        owner_did,
        author_did,
        asserted_unix,
        blob_hash,
        SensitivityClass::Classified,
    )
}

pub fn wellbeing_observation_summary(report: &WellbeingObservation) -> String {
    serde_json::json!({
        "mood": report.mood_label,
        "intensity": report.intensity_1_10,
    })
    .to_string()
}

pub fn therapy_note_summary(report: &TherapyNote) -> String {
    serde_json::json!({
        "provider": report.provider_label,
        "session_date_unix": report.session_date_unix,
        "note_len": report.notes.len(),
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn therapy_note_classified_sanctuary_kind() {
        let note = TherapyNote::new("session reflection");
        let env = build_therapy_note_envelope(&note, "did:o", "did:a", 1, None);
        assert_eq!(env.sensitivity, SensitivityClass::Classified);
        assert!(env.id.contains(":therapy_note:"));
    }
}