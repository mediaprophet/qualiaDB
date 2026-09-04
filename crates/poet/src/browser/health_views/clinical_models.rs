//! Data models, projection, and validation logic for conditions and medications.

use serde::{Deserialize, Serialize};
use super::model::HealthRecord;

/// Status of a diagnosed condition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConditionStatus {
    Active,
    Recurrence,
    Relapse,
    Remission,
    Resolved,
}

impl ConditionStatus {
    pub fn parse(s: &str) -> Self {
        match s.trim().to_lowercase().as_str() {
            "resolved" => Self::Resolved,
            "remission" => Self::Remission,
            "recurrence" => Self::Recurrence,
            "relapse" => Self::Relapse,
            _ => Self::Active,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Recurrence => "recurrence",
            Self::Relapse => "relapse",
            Self::Remission => "remission",
            Self::Resolved => "resolved",
        }
    }

    pub fn display_label(&self) -> &'static str {
        match self {
            Self::Active => "Active",
            Self::Recurrence => "Recurrence",
            Self::Relapse => "Relapse",
            Self::Remission => "In Remission",
            Self::Resolved => "Resolved",
        }
    }

    pub fn is_active(&self) -> bool {
        matches!(self, Self::Active | Self::Recurrence | Self::Relapse)
    }

    pub fn is_history(&self) -> bool {
        matches!(self, Self::Remission | Self::Resolved)
    }
}

/// Projected condition item.
#[derive(Debug, Clone, PartialEq)]
pub struct ConditionItem {
    pub record: HealthRecord,
    pub name: String,
    pub status: ConditionStatus,
    pub onset_date: Option<String>,
    pub resolved_date: Option<String>,
    pub clinical_code: Option<String>,
    pub notes: Option<String>,
    pub sensitivity: String,
}

/// Projects raw HealthRecords into structured ConditionItems.
pub fn project_conditions(records: &[HealthRecord]) -> Vec<ConditionItem> {
    records
        .iter()
        .filter(|r| r.family == "health_condition")
        .map(|r| {
            let name = r.field_text("name").or_else(|| r.field_text("code")).unwrap_or_else(|| r.title.clone());
            let status = ConditionStatus::parse(&r.field_text("status").unwrap_or_default());
            let onset_date = r.field_text("onset_date").or_else(|| r.field_text("occurred_at"));
            let resolved_date = r.field_text("resolved_date");
            let clinical_code = r.field_text("clinical_code");
            let notes = r.field_text("notes");
            let sensitivity = r.field_text("sensitivity").unwrap_or_else(|| "classified".into());
            ConditionItem {
                record: r.clone(),
                name,
                status,
                onset_date,
                resolved_date,
                clinical_code,
                notes,
                sensitivity,
            }
        })
        .collect()
}

/// Builds the field payload for creating a condition record (`health_condition`).
pub fn build_condition_payload(
    name: &str,
    status: &str,
    onset_date: &str,
    resolved_date: Option<&str>,
    clinical_code: Option<&str>,
    notes: Option<&str>,
    sensitivity: &str,
) -> (String, String, serde_json::Map<String, serde_json::Value>) {
    let title = name.trim().to_string();
    let mut fields = serde_json::Map::new();
    fields.insert("name".into(), serde_json::Value::String(title.clone()));
    fields.insert("code".into(), serde_json::Value::String(clinical_code.unwrap_or(name).trim().to_string()));
    fields.insert("status".into(), serde_json::Value::String(status.trim().to_lowercase()));
    fields.insert("onset_date".into(), serde_json::Value::String(onset_date.trim().to_string()));
    fields.insert("occurred_at".into(), serde_json::Value::String(onset_date.trim().to_string()));
    if let Some(res) = resolved_date.filter(|s| !s.trim().is_empty()) {
        fields.insert("resolved_date".into(), serde_json::Value::String(res.trim().to_string()));
    }
    if let Some(code) = clinical_code.filter(|s| !s.trim().is_empty()) {
        fields.insert("clinical_code".into(), serde_json::Value::String(code.trim().to_string()));
    }
    if let Some(n) = notes.filter(|s| !s.trim().is_empty()) {
        fields.insert("notes".into(), serde_json::Value::String(n.trim().to_string()));
    }
    fields.insert("sensitivity".into(), serde_json::Value::String(sensitivity.trim().to_string()));

    ("health_condition".into(), title, fields)
}

/// Status of a medication.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MedicationStatus {
    Active,
    OnHold,
    Completed,
    Stopped,
}

impl MedicationStatus {
    pub fn parse(s: &str) -> Self {
        match s.trim().to_lowercase().as_str() {
            "stopped" => Self::Stopped,
            "completed" => Self::Completed,
            "on_hold" | "onhold" | "paused" => Self::OnHold,
            _ => Self::Active,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::OnHold => "on_hold",
            Self::Completed => "completed",
            Self::Stopped => "stopped",
        }
    }

    pub fn display_label(&self) -> &'static str {
        match self {
            Self::Active => "Active",
            Self::OnHold => "On Hold",
            Self::Completed => "Completed",
            Self::Stopped => "Stopped",
        }
    }

    pub fn is_active(&self) -> bool {
        matches!(self, Self::Active | Self::OnHold)
    }

    pub fn is_history(&self) -> bool {
        matches!(self, Self::Completed | Self::Stopped)
    }
}

/// Projected medication item.
#[derive(Debug, Clone, PartialEq)]
pub struct MedicationItem {
    pub record: HealthRecord,
    pub name: String,
    pub dose: String,
    pub unit: String,
    pub schedule: String,
    pub status: MedicationStatus,
    pub started_at: Option<String>,
    pub stopped_at: Option<String>,
    pub indication: Option<String>,
    pub sensitivity: String,
}

/// Projects raw HealthRecords into structured MedicationItems.
pub fn project_medications(records: &[HealthRecord]) -> Vec<MedicationItem> {
    records
        .iter()
        .filter(|r| r.family == "health_medication")
        .map(|r| {
            let name = r.title.clone();
            let dose = r.field_text("dose").unwrap_or_default();
            let unit = r.field_text("unit").unwrap_or_else(|| "mg".into());
            let schedule = r.field_text("schedule").or_else(|| r.field_text("frequency")).unwrap_or_else(|| "daily".into());
            let status = MedicationStatus::parse(&r.field_text("status").unwrap_or_default());
            let started_at = r.field_text("started_at").or_else(|| r.field_text("occurred_at"));
            let stopped_at = r.field_text("stopped_at");
            let indication = r.field_text("indication");
            let sensitivity = r.field_text("sensitivity").unwrap_or_else(|| "classified".into());
            MedicationItem {
                record: r.clone(),
                name,
                dose,
                unit,
                schedule,
                status,
                started_at,
                stopped_at,
                indication,
                sensitivity,
            }
        })
        .collect()
}

/// Builds the field payload for creating a medication record (`health_medication`).
pub fn build_medication_payload(
    name: &str,
    dose: &str,
    unit: &str,
    schedule: &str,
    status: &str,
    started_at: &str,
    stopped_at: Option<&str>,
    indication: Option<&str>,
    sensitivity: &str,
) -> (String, String, serde_json::Map<String, serde_json::Value>) {
    let title = name.trim().to_string();
    let mut fields = serde_json::Map::new();
    fields.insert("name".into(), serde_json::Value::String(title.clone()));
    fields.insert("dose".into(), serde_json::Value::String(dose.trim().to_string()));
    fields.insert("unit".into(), serde_json::Value::String(unit.trim().to_string()));
    fields.insert("schedule".into(), serde_json::Value::String(schedule.trim().to_string()));
    fields.insert("status".into(), serde_json::Value::String(status.trim().to_lowercase()));
    fields.insert("started_at".into(), serde_json::Value::String(started_at.trim().to_string()));
    fields.insert("occurred_at".into(), serde_json::Value::String(started_at.trim().to_string()));
    if let Some(stop) = stopped_at.filter(|s| !s.trim().is_empty()) {
        fields.insert("stopped_at".into(), serde_json::Value::String(stop.trim().to_string()));
    }
    if let Some(ind) = indication.filter(|s| !s.trim().is_empty()) {
        fields.insert("indication".into(), serde_json::Value::String(ind.trim().to_string()));
    }
    fields.insert("sensitivity".into(), serde_json::Value::String(sensitivity.trim().to_string()));

    ("health_medication".into(), title, fields)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::browser::health_views::model::records_from_payload;
    use serde_json::json;

    #[test]
    fn condition_status_separates_active_and_historical() {
        assert!(ConditionStatus::Active.is_active());
        assert!(ConditionStatus::Recurrence.is_active());
        assert!(ConditionStatus::Relapse.is_active());
        assert!(!ConditionStatus::Active.is_history());

        assert!(ConditionStatus::Resolved.is_history());
        assert!(ConditionStatus::Remission.is_history());
        assert!(!ConditionStatus::Resolved.is_active());
    }

    #[test]
    fn medication_status_separates_active_and_historical() {
        assert!(MedicationStatus::Active.is_active());
        assert!(MedicationStatus::OnHold.is_active());
        assert!(!MedicationStatus::Active.is_history());

        assert!(MedicationStatus::Stopped.is_history());
        assert!(MedicationStatus::Completed.is_history());
        assert!(!MedicationStatus::Stopped.is_active());
    }

    #[test]
    fn projects_conditions_with_active_and_resolved_distinction() {
        let payload = json!({
            "records": [
                {
                    "id": "cond-1",
                    "family": "health_condition",
                    "title": "Essential Hypertension",
                    "fields": {
                        "name": "Essential Hypertension",
                        "status": "active",
                        "onset_date": "2024-01-15",
                        "clinical_code": "SNOMED: 59621000",
                        "sensitivity": "restricted"
                    }
                },
                {
                    "id": "cond-2",
                    "family": "health_condition",
                    "title": "Acute Bronchitis",
                    "fields": {
                        "name": "Acute Bronchitis",
                        "status": "resolved",
                        "onset_date": "2023-11-01",
                        "resolved_date": "2023-11-20",
                        "sensitivity": "classified"
                    }
                }
            ]
        });

        let records = records_from_payload("health_condition", &payload);
        let items = project_conditions(&records);
        assert_eq!(items.len(), 2);

        let active = items.iter().find(|c| c.name == "Essential Hypertension").unwrap();
        assert_eq!(active.status, ConditionStatus::Active);
        assert!(active.status.is_active());
        assert_eq!(active.clinical_code.as_deref(), Some("SNOMED: 59621000"));

        let resolved = items.iter().find(|c| c.name == "Acute Bronchitis").unwrap();
        assert_eq!(resolved.status, ConditionStatus::Resolved);
        assert!(resolved.status.is_history());
        assert_eq!(resolved.resolved_date.as_deref(), Some("2023-11-20"));
    }

    #[test]
    fn projects_medications_with_dose_unit_schedule() {
        let payload = json!({
            "records": [
                {
                    "id": "med-1",
                    "family": "health_medication",
                    "title": "Lisinopril",
                    "fields": {
                        "dose": "10",
                        "unit": "mg",
                        "schedule": "Once daily in the morning",
                        "status": "active",
                        "started_at": "2024-01-20",
                        "indication": "Hypertension"
                    }
                },
                {
                    "id": "med-2",
                    "family": "health_medication",
                    "title": "Amoxicillin",
                    "fields": {
                        "dose": "500",
                        "unit": "mg",
                        "schedule": "Three times daily",
                        "status": "stopped",
                        "started_at": "2023-11-02",
                        "stopped_at": "2023-11-12",
                        "indication": "Bronchitis"
                    }
                }
            ]
        });

        let records = records_from_payload("health_medication", &payload);
        let items = project_medications(&records);
        assert_eq!(items.len(), 2);

        let active = items.iter().find(|m| m.name == "Lisinopril").unwrap();
        assert_eq!(active.dose, "10");
        assert_eq!(active.unit, "mg");
        assert_eq!(active.schedule, "Once daily in the morning");
        assert_eq!(active.status, MedicationStatus::Active);
        assert!(active.status.is_active());

        let stopped = items.iter().find(|m| m.name == "Amoxicillin").unwrap();
        assert_eq!(stopped.status, MedicationStatus::Stopped);
        assert!(stopped.status.is_history());
        assert_eq!(stopped.stopped_at.as_deref(), Some("2023-11-12"));
    }

    #[test]
    fn build_payloads_generate_correct_fields() {
        let (c_family, c_title, c_fields) = build_condition_payload(
            "Asthma",
            "active",
            "2022-05-10",
            None,
            Some("SNOMED: 195967001"),
            Some("Mild intermittent"),
            "restricted",
        );
        assert_eq!(c_family, "health_condition");
        assert_eq!(c_title, "Asthma");
        assert_eq!(c_fields.get("status").unwrap(), "active");
        assert_eq!(c_fields.get("clinical_code").unwrap(), "SNOMED: 195967001");

        let (m_family, m_title, m_fields) = build_medication_payload(
            "Salbutamol Inhaler",
            "100",
            "mcg",
            "1-2 puffs as needed",
            "active",
            "2022-05-10",
            None,
            Some("Asthma relief"),
            "restricted",
        );
        assert_eq!(m_family, "health_medication");
        assert_eq!(m_title, "Salbutamol Inhaler");
        assert_eq!(m_fields.get("dose").unwrap(), "100");
        assert_eq!(m_fields.get("unit").unwrap(), "mcg");
        assert_eq!(m_fields.get("schedule").unwrap(), "1-2 puffs as needed");
    }
}
