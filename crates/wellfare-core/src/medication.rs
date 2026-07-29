//! Medication, administration, and diet record builders for WellFair Phase 2 (MED-01..13).
//!
//! Compiles self-reported entries into canonical `RecordEnvelope` + JSON summary projections.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::record::{EpistemicStatus, EvidenceType, RecordEnvelope, SensitivityClass};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdministrationStatus {
    Taken,
    Skipped,
    Overdue,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MedicationCatalogEntry {
    pub id: String,
    pub name: String,
    pub dose: String,
    pub route: String,
    /// HH:MM local schedule slots (e.g. ["08:00", "20:00"]).
    pub schedule_times: Vec<String>,
    pub prescriber: Option<String>,
    pub ceased_at_unix: Option<u32>,
    pub created_at_unix: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MedicationAdministration {
    pub id: String,
    pub medication_id: String,
    pub medication_name: String,
    pub status: AdministrationStatus,
    pub administered_at_unix: u32,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DietEntry {
    pub id: String,
    pub description: String,
    pub meal_type: String,
    pub calories_kcal: Option<u32>,
    pub logged_at_unix: u32,
}

fn content_hash_hex(payload: &str) -> String {
    hex::encode(Sha256::digest(payload.as_bytes()).as_slice())
}

fn stable_suffix(parts: &[&str]) -> String {
    let joined = parts.join(":");
    let digest = Sha256::digest(joined.as_bytes());
    hex::encode(&digest[..4])
}

pub fn new_medication_id(name: &str, created_unix: u32) -> String {
    format!(
        "urn:wellfair:medication:{}",
        stable_suffix(&[name, &created_unix.to_string()])
    )
}

pub fn new_administration_id(med_id: &str, unix: u32) -> String {
    format!(
        "urn:wellfair:med_admin:{}",
        stable_suffix(&[med_id, &unix.to_string()])
    )
}

pub fn new_diet_id(description: &str, unix: u32) -> String {
    format!(
        "urn:wellfair:diet:{}",
        stable_suffix(&[description, &unix.to_string()])
    )
}

fn self_reported_envelope(
    id: &str,
    owner_did: &str,
    author_did: &str,
    asserted_unix: u32,
    payload_json: &str,
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
        blob_hash: Some(content_hash_hex(payload_json)),
        tombstone: false,
    }
}

pub struct EnvelopeWithSummary {
    pub envelope: RecordEnvelope,
    pub summary: String,
}

pub fn medication_envelope(
    entry: &MedicationCatalogEntry,
    owner_did: &str,
    author_did: &str,
) -> EnvelopeWithSummary {
    let payload = serde_json::to_string(entry).unwrap_or_default();
    let summary = serde_json::json!({
        "name": entry.name,
        "dose": entry.dose,
        "route": entry.route,
        "schedule_times": entry.schedule_times,
        "ceased": entry.ceased_at_unix.is_some(),
    })
    .to_string();
    EnvelopeWithSummary {
        envelope: self_reported_envelope(
            &entry.id,
            owner_did,
            author_did,
            entry.created_at_unix,
            &payload,
        ),
        summary,
    }
}

pub fn administration_envelope(
    admin: &MedicationAdministration,
    owner_did: &str,
    author_did: &str,
) -> EnvelopeWithSummary {
    let payload = serde_json::to_string(admin).unwrap_or_default();
    let summary = serde_json::json!({
        "medication_id": admin.medication_id,
        "medication_name": admin.medication_name,
        "status": admin.status,
        "notes": admin.notes,
    })
    .to_string();
    EnvelopeWithSummary {
        envelope: self_reported_envelope(
            &admin.id,
            owner_did,
            author_did,
            admin.administered_at_unix,
            &payload,
        ),
        summary,
    }
}

pub fn diet_envelope(entry: &DietEntry, owner_did: &str, author_did: &str) -> EnvelopeWithSummary {
    let payload = serde_json::to_string(entry).unwrap_or_default();
    let summary = serde_json::json!({
        "description": entry.description,
        "meal_type": entry.meal_type,
        "calories_kcal": entry.calories_kcal,
    })
    .to_string();
    EnvelopeWithSummary {
        envelope: self_reported_envelope(
            &entry.id,
            owner_did,
            author_did,
            entry.logged_at_unix,
            &payload,
        ),
        summary,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn medication_envelope_has_stable_id_prefix() {
        let entry = MedicationCatalogEntry {
            id: new_medication_id("Metformin", 1_700_000_000),
            name: "Metformin".into(),
            dose: "500mg".into(),
            route: "oral".into(),
            schedule_times: vec!["08:00".into()],
            prescriber: None,
            ceased_at_unix: None,
            created_at_unix: 1_700_000_000,
        };
        let packed = medication_envelope(&entry, "did:wf:owner", "did:wf:owner");
        assert!(packed.envelope.id.contains(":medication:"));
        assert!(packed.summary.contains("Metformin"));
    }
}
