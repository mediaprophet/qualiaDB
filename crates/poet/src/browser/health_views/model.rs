//! Pure projections for the person-controlled health workspace.

use chrono::{DateTime, NaiveDate, NaiveDateTime};
use serde_json::{Map, Value};

#[derive(Debug, Clone, PartialEq)]
pub struct HealthRecord {
    pub id: String,
    pub family: String,
    pub title: String,
    pub fields: Map<String, Value>,
    pub created_at: i64,
    pub updated_at: i64,
}

impl HealthRecord {
    pub fn field_text(&self, key: &str) -> Option<String> {
        self.fields.get(key).and_then(|value| match value {
            Value::String(text) if !text.trim().is_empty() => Some(text.clone()),
            Value::Number(number) => Some(number.to_string()),
            Value::Bool(flag) => Some(flag.to_string()),
            _ => None,
        })
    }

    pub fn field_number(&self, key: &str) -> Option<f64> {
        self.fields.get(key).and_then(|value| match value {
            Value::Number(number) => number.as_f64(),
            Value::String(text) => text.trim().parse::<f64>().ok(),
            _ => None,
        })
    }

    pub fn occurred_label(&self) -> String {
        for key in ["measured_at", "occurred_at", "date"] {
            if let Some(value) = self.field_text(key) {
                return value.replace('T', " · ");
            }
        }
        if self.updated_at > 0 {
            if let Some(time) = DateTime::from_timestamp(self.updated_at, 0) {
                return time.format("%d %b %Y · %H:%M UTC").to_string();
            }
        }
        "Date not recorded".into()
    }

    pub fn sort_key(&self) -> i64 {
        for key in ["measured_at", "occurred_at", "date"] {
            let Some(raw) = self.field_text(key) else {
                continue;
            };
            if let Ok(parsed) = DateTime::parse_from_rfc3339(&raw) {
                return parsed.timestamp();
            }
            if let Ok(parsed) = NaiveDateTime::parse_from_str(&raw, "%Y-%m-%dT%H:%M") {
                return parsed.and_utc().timestamp();
            }
            if let Ok(parsed) = NaiveDate::parse_from_str(&raw, "%Y-%m-%d") {
                return parsed
                    .and_hms_opt(0, 0, 0)
                    .map(|time| time.and_utc().timestamp())
                    .unwrap_or(self.updated_at);
            }
        }
        self.updated_at
    }

    pub fn summary(&self) -> String {
        let keys: &[&str] = match self.family.as_str() {
            "health_vital" => &["sys_bp", "dia_bp", "hr", "glucose", "unit"],
            "health_condition" => &["code", "status"],
            "health_medication" => &["dose", "status"],
            "health_lab" => &["analyte", "value", "unit"],
            "health_share" => &["share_to", "purpose", "status"],
            "health_document" => &["uri", "sensitivity"],
            _ => &["kind", "status"],
        };
        let mut parts = Vec::new();
        for key in keys {
            if let Some(value) = self.field_text(key) {
                parts.push(format!("{} {}", friendly_key(key), value));
            }
        }
        if parts.is_empty() {
            "No additional details".into()
        } else {
            parts.join(" · ")
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VitalPoint {
    pub sort_key: i64,
    pub systolic: Option<f64>,
    pub diastolic: Option<f64>,
    pub heart_rate: Option<f64>,
}

pub fn records_from_payload(family: &str, data: &Value) -> Vec<HealthRecord> {
    data.get("records")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|record| {
            Some(HealthRecord {
                id: record.get("id")?.as_str()?.to_string(),
                family: record
                    .get("family")
                    .and_then(Value::as_str)
                    .unwrap_or(family)
                    .to_string(),
                title: record
                    .get("title")
                    .and_then(Value::as_str)
                    .unwrap_or("Untitled record")
                    .to_string(),
                fields: record
                    .get("fields")
                    .and_then(Value::as_object)
                    .cloned()
                    .unwrap_or_default(),
                created_at: record
                    .get("created_at")
                    .and_then(Value::as_i64)
                    .unwrap_or_default(),
                updated_at: record
                    .get("updated_at")
                    .and_then(Value::as_i64)
                    .unwrap_or_default(),
            })
        })
        .collect()
}

pub fn sort_recent(records: &mut [HealthRecord]) {
    records.sort_by_key(|record| std::cmp::Reverse(record.sort_key()));
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MetricKind {
    BloodPressure,
    HeartRate,
    Glucose,
    Lab(String),
}

impl MetricKind {
    pub fn id(&self) -> String {
        match self {
            MetricKind::BloodPressure => "bp".into(),
            MetricKind::HeartRate => "hr".into(),
            MetricKind::Glucose => "glucose".into(),
            MetricKind::Lab(analyte) => format!("lab:{}", analyte.to_lowercase().replace(' ', "_")),
        }
    }

    pub fn label(&self) -> String {
        match self {
            MetricKind::BloodPressure => "Blood pressure".into(),
            MetricKind::HeartRate => "Heart rate".into(),
            MetricKind::Glucose => "Glucose".into(),
            MetricKind::Lab(analyte) => analyte.clone(),
        }
    }

    pub fn default_unit(&self) -> &str {
        match self {
            MetricKind::BloodPressure => "mmHg",
            MetricKind::HeartRate => "bpm",
            MetricKind::Glucose => "mg/dL",
            MetricKind::Lab(_) => "",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MetricPoint {
    pub sort_key: i64,
    pub timestamp_label: String,
    pub primary: f64,
    pub secondary: Option<f64>,
    pub unit: String,
    pub record_id: String,
    pub sensitivity: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MetricSeries {
    pub kind: MetricKind,
    pub unit: String,
    pub points: Vec<MetricPoint>,
}

/// Discover all metric kinds present in health records, prioritizing standard vitals.
pub fn available_metric_kinds(records: &[HealthRecord]) -> Vec<MetricKind> {
    let mut kinds = vec![
        MetricKind::BloodPressure,
        MetricKind::HeartRate,
        MetricKind::Glucose,
    ];
    let mut seen_labs = std::collections::HashSet::new();
    for record in records {
        if record.family == "health_lab" {
            if let Some(analyte) = record.field_text("analyte") {
                let trimmed = analyte.trim().to_string();
                if !trimmed.is_empty() && seen_labs.insert(trimmed.to_lowercase()) {
                    kinds.push(MetricKind::Lab(trimmed));
                }
            }
        }
    }
    kinds
}

/// Extract points matching a specific metric kind and group them strictly by unit.
///
/// Under no circumstances are different units co-mingled or converted without explicit license.
/// Each distinct unit produces its own `MetricSeries`.
pub fn extract_metric_series(records: &[HealthRecord], kind: &MetricKind) -> Vec<MetricSeries> {
    let mut by_unit: std::collections::BTreeMap<String, Vec<MetricPoint>> =
        std::collections::BTreeMap::new();

    for record in records {
        match kind {
            MetricKind::BloodPressure => {
                if record.family == "health_vital" {
                    if let Some(sys) = record.field_number("sys_bp") {
                        let dia = record.field_number("dia_bp");
                        let unit = record.field_text("unit").unwrap_or_else(|| "mmHg".into());
                        let point = MetricPoint {
                            sort_key: record.sort_key(),
                            timestamp_label: record.occurred_label(),
                            primary: sys,
                            secondary: dia,
                            unit: unit.clone(),
                            record_id: record.id.clone(),
                            sensitivity: record
                                .field_text("sensitivity")
                                .unwrap_or_else(|| "classified".into()),
                        };
                        by_unit.entry(unit).or_default().push(point);
                    }
                }
            }
            MetricKind::HeartRate => {
                if record.family == "health_vital" {
                    if let Some(hr) = record.field_number("hr") {
                        let unit = record.field_text("unit").unwrap_or_else(|| "bpm".into());
                        let point = MetricPoint {
                            sort_key: record.sort_key(),
                            timestamp_label: record.occurred_label(),
                            primary: hr,
                            secondary: None,
                            unit: unit.clone(),
                            record_id: record.id.clone(),
                            sensitivity: record
                                .field_text("sensitivity")
                                .unwrap_or_else(|| "classified".into()),
                        };
                        by_unit.entry(unit).or_default().push(point);
                    }
                }
            }
            MetricKind::Glucose => {
                let mut found = false;
                if record.family == "health_vital" {
                    if let Some(val) = record.field_number("glucose") {
                        let unit = record.field_text("unit").unwrap_or_else(|| "mg/dL".into());
                        let point = MetricPoint {
                            sort_key: record.sort_key(),
                            timestamp_label: record.occurred_label(),
                            primary: val,
                            secondary: None,
                            unit: unit.clone(),
                            record_id: record.id.clone(),
                            sensitivity: record
                                .field_text("sensitivity")
                                .unwrap_or_else(|| "classified".into()),
                        };
                        by_unit.entry(unit).or_default().push(point);
                        found = true;
                    }
                }
                if !found && record.family == "health_lab" {
                    if let Some(analyte) = record.field_text("analyte") {
                        if analyte.trim().eq_ignore_ascii_case("glucose") {
                            if let Some(val) = record.field_number("value") {
                                let unit =
                                    record.field_text("unit").unwrap_or_else(|| "mg/dL".into());
                                let point = MetricPoint {
                                    sort_key: record.sort_key(),
                                    timestamp_label: record.occurred_label(),
                                    primary: val,
                                    secondary: None,
                                    unit: unit.clone(),
                                    record_id: record.id.clone(),
                                    sensitivity: record
                                        .field_text("sensitivity")
                                        .unwrap_or_else(|| "classified".into()),
                                };
                                by_unit.entry(unit).or_default().push(point);
                            }
                        }
                    }
                }
            }
            MetricKind::Lab(analyte_name) => {
                if record.family == "health_lab" {
                    if let Some(analyte) = record.field_text("analyte") {
                        if analyte.trim().eq_ignore_ascii_case(analyte_name.trim()) {
                            if let Some(val) = record.field_number("value") {
                                let unit = record
                                    .field_text("unit")
                                    .unwrap_or_else(|| "unspecified unit".into());
                                let point = MetricPoint {
                                    sort_key: record.sort_key(),
                                    timestamp_label: record.occurred_label(),
                                    primary: val,
                                    secondary: None,
                                    unit: unit.clone(),
                                    record_id: record.id.clone(),
                                    sensitivity: record
                                        .field_text("sensitivity")
                                        .unwrap_or_else(|| "classified".into()),
                                };
                                by_unit.entry(unit).or_default().push(point);
                            }
                        }
                    }
                }
            }
        }
    }

    let mut result = Vec::new();
    for (unit, mut points) in by_unit {
        points.sort_by_key(|p| p.sort_key);
        result.push(MetricSeries {
            kind: kind.clone(),
            unit,
            points,
        });
    }
    result
}

pub fn vital_points(records: &[HealthRecord]) -> Vec<VitalPoint> {
    let mut points = records
        .iter()
        .filter(|record| record.family == "health_vital")
        .filter_map(|record| {
            let point = VitalPoint {
                sort_key: record.sort_key(),
                systolic: record.field_number("sys_bp"),
                diastolic: record.field_number("dia_bp"),
                heart_rate: record.field_number("hr"),
            };
            (point.systolic.is_some() || point.diastolic.is_some() || point.heart_rate.is_some())
                .then_some(point)
        })
        .collect::<Vec<_>>();
    points.sort_by_key(|point| point.sort_key);
    points
}

fn friendly_key(key: &str) -> &str {
    match key {
        "sys_bp" => "Systolic",
        "dia_bp" => "Diastolic",
        "hr" => "Heart rate",
        "share_to" => "Shared with",
        "corrects_id" => "Corrects record",
        "reason" => "Correction reason",
        _ => key,
    }
}

/// Status of a record in the person-controlled timeline regarding corrections.
#[derive(Debug, Clone, PartialEq)]
pub enum CorrectionStatus {
    /// Active record that has not been superseded or corrected.
    Current,
    /// Record that has been corrected by an append-only receipt.
    Corrected {
        receipt_id: String,
        reason: String,
        corrected_at: String,
    },
    /// Immutable correction receipt linked to an earlier record.
    CorrectionReceipt { targets_id: String, reason: String },
}

/// A projected timeline item pairing the underlying health record with its correction status.
#[derive(Debug, Clone, PartialEq)]
pub struct TimelineItem {
    pub record: HealthRecord,
    pub status: CorrectionStatus,
}

/// Project health records into timeline items, identifying correction relationships.
///
/// Ensures original records remain intact and queryable while explicitly distinguishing
/// between current entries, corrected entries, and immutable correction receipts.
pub fn project_timeline(records: &[HealthRecord]) -> Vec<TimelineItem> {
    let mut corrections: std::collections::HashMap<String, &HealthRecord> =
        std::collections::HashMap::new();

    for record in records {
        if record.family == "health_correction" {
            if let Some(target) = record.field_text("corrects_id") {
                corrections.insert(target, record);
            }
        } else if let Some(target) = record.field_text("corrects_id") {
            corrections.insert(target, record);
        }
    }

    records
        .iter()
        .map(|record| {
            let status = if record.family == "health_correction" {
                let targets_id = record.field_text("corrects_id").unwrap_or_default();
                let reason = record
                    .field_text("reason")
                    .unwrap_or_else(|| "Correction receipt".into());
                CorrectionStatus::CorrectionReceipt { targets_id, reason }
            } else if let Some(receipt) = corrections.get(&record.id) {
                let receipt_id = receipt.id.clone();
                let reason = receipt
                    .field_text("reason")
                    .unwrap_or_else(|| "Recorded correction".into());
                let corrected_at = receipt.occurred_label();
                CorrectionStatus::Corrected {
                    receipt_id,
                    reason,
                    corrected_at,
                }
            } else {
                CorrectionStatus::Current
            };
            TimelineItem {
                record: record.clone(),
                status,
            }
        })
        .collect()
}

/// Construct the field payload for an append-only correction receipt.
///
/// Preserves the original record ID, links the reason and corrected values,
/// and assigns an explicit timestamp and sensitivity.
pub fn build_correction_receipt_payload(
    original: &HealthRecord,
    reason: &str,
    correction_notes: &str,
    sensitivity: &str,
) -> (String, String, serde_json::Map<String, serde_json::Value>) {
    let now = chrono::Utc::now();
    let receipt_title = format!("Correction receipt · {}", original.title);
    let mut fields = serde_json::Map::new();
    fields.insert(
        "corrects_id".into(),
        serde_json::Value::String(original.id.clone()),
    );
    fields.insert(
        "target_family".into(),
        serde_json::Value::String(original.family.clone()),
    );
    fields.insert(
        "reason".into(),
        serde_json::Value::String(reason.trim().to_string()),
    );
    if !correction_notes.trim().is_empty() {
        fields.insert(
            "notes".into(),
            serde_json::Value::String(correction_notes.trim().to_string()),
        );
    }
    fields.insert(
        "original_title".into(),
        serde_json::Value::String(original.title.clone()),
    );
    fields.insert(
        "occurred_at".into(),
        serde_json::Value::String(now.to_rfc3339()),
    );
    fields.insert(
        "sensitivity".into(),
        serde_json::Value::String(sensitivity.to_string()),
    );
    ("health_correction".into(), receipt_title, fields)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_numeric_and_string_vital_values() {
        let records = records_from_payload(
            "health_vital",
            &json!({"records": [{
                "id": "v-1", "family": "health_vital", "title": "Morning",
                "fields": {"sys_bp": "121", "dia_bp": 79, "hr": "64"},
                "created_at": 10, "updated_at": 11
            }]}),
        );
        let points = vital_points(&records);
        assert_eq!(points.len(), 1);
        assert_eq!(points[0].systolic, Some(121.0));
        assert_eq!(points[0].diastolic, Some(79.0));
        assert_eq!(points[0].heart_rate, Some(64.0));
    }

    #[test]
    fn sorts_timeline_by_recorded_occurrence() {
        let mut records = records_from_payload(
            "health_note",
            &json!({"records": [
                {"id":"old", "title":"Old", "fields":{"date":"2025-01-01"}, "updated_at":99},
                {"id":"new", "title":"New", "fields":{"date":"2026-01-01"}, "updated_at":1}
            ]}),
        );
        sort_recent(&mut records);
        assert_eq!(records[0].id, "new");
    }

    #[test]
    fn empty_payload_creates_no_demo_records() {
        assert!(records_from_payload("health_vital", &json!({})).is_empty());
    }

    #[test]
    fn project_timeline_distinguishes_current_and_corrected_records() {
        let records = records_from_payload(
            "health_vital",
            &json!({"records": [
                {
                    "id": "bp-original",
                    "family": "health_vital",
                    "title": "Blood pressure reading",
                    "fields": {"sys_bp": 145, "dia_bp": 92},
                    "created_at": 100,
                    "updated_at": 100
                },
                {
                    "id": "receipt-1",
                    "family": "health_correction",
                    "title": "Correction receipt · Blood pressure reading",
                    "fields": {
                        "corrects_id": "bp-original",
                        "reason": "Cuff movement detected during measurement",
                        "occurred_at": "2026-09-04T12:00:00Z"
                    },
                    "created_at": 110,
                    "updated_at": 110
                },
                {
                    "id": "hr-reading",
                    "family": "health_vital",
                    "title": "Heart rate reading",
                    "fields": {"hr": 68},
                    "created_at": 120,
                    "updated_at": 120
                }
            ]}),
        );

        let timeline = project_timeline(&records);
        assert_eq!(timeline.len(), 3);

        // Original record remains queryable and is flagged as Corrected with receipt details
        let original_item = timeline
            .iter()
            .find(|t| t.record.id == "bp-original")
            .unwrap();
        match &original_item.status {
            CorrectionStatus::Corrected {
                receipt_id, reason, ..
            } => {
                assert_eq!(receipt_id, "receipt-1");
                assert_eq!(reason, "Cuff movement detected during measurement");
            }
            other => panic!("Expected Corrected status, got {:?}", other),
        }

        // The receipt itself is flagged as CorrectionReceipt
        let receipt_item = timeline
            .iter()
            .find(|t| t.record.id == "receipt-1")
            .unwrap();
        match &receipt_item.status {
            CorrectionStatus::CorrectionReceipt { targets_id, reason } => {
                assert_eq!(targets_id, "bp-original");
                assert_eq!(reason, "Cuff movement detected during measurement");
            }
            other => panic!("Expected CorrectionReceipt status, got {:?}", other),
        }

        // Uncorrected record remains Current
        let current_item = timeline
            .iter()
            .find(|t| t.record.id == "hr-reading")
            .unwrap();
        assert_eq!(current_item.status, CorrectionStatus::Current);
    }

    #[test]
    fn build_correction_receipt_payload_stores_provenance() {
        let original = HealthRecord {
            id: "vital-42".into(),
            family: "health_vital".into(),
            title: "Morning vitals".into(),
            fields: serde_json::Map::new(),
            created_at: 100,
            updated_at: 100,
        };
        let (family, title, fields) = build_correction_receipt_payload(
            &original,
            "Typo in diastolic field",
            "Actual reading was 78, entered as 98",
            "classified",
        );
        assert_eq!(family, "health_correction");
        assert_eq!(title, "Correction receipt · Morning vitals");
        assert_eq!(
            fields.get("corrects_id").unwrap().as_str().unwrap(),
            "vital-42"
        );
        assert_eq!(
            fields.get("target_family").unwrap().as_str().unwrap(),
            "health_vital"
        );
        assert_eq!(
            fields.get("reason").unwrap().as_str().unwrap(),
            "Typo in diastolic field"
        );
        assert_eq!(
            fields.get("notes").unwrap().as_str().unwrap(),
            "Actual reading was 78, entered as 98"
        );
        assert_eq!(
            fields.get("sensitivity").unwrap().as_str().unwrap(),
            "classified"
        );
        assert!(fields.contains_key("occurred_at"));
    }

    #[test]
    fn metric_series_partitions_differing_units_without_mixing() {
        let records = records_from_payload(
            "health_vital",
            &json!({"records": [
                {
                    "id": "g-mgdl",
                    "family": "health_vital",
                    "title": "Fasting glucose",
                    "fields": {"glucose": 95, "unit": "mg/dL", "date": "2026-09-01"},
                    "created_at": 10, "updated_at": 10
                },
                {
                    "id": "g-mmol",
                    "family": "health_vital",
                    "title": "Postprandial glucose",
                    "fields": {"glucose": 5.4, "unit": "mmol/L", "date": "2026-09-02"},
                    "created_at": 20, "updated_at": 20
                }
            ]}),
        );

        let series = extract_metric_series(&records, &MetricKind::Glucose);
        // CRITICAL: Units must NEVER mix silently into a single series
        assert_eq!(series.len(), 2, "mg/dL and mmol/L must be separate series");

        let mgdl_series = series.iter().find(|s| s.unit == "mg/dL").unwrap();
        assert_eq!(mgdl_series.points.len(), 1);
        assert_eq!(mgdl_series.points[0].primary, 95.0);

        let mmol_series = series.iter().find(|s| s.unit == "mmol/L").unwrap();
        assert_eq!(mmol_series.points.len(), 1);
        assert_eq!(mmol_series.points[0].primary, 5.4);
    }

    #[test]
    fn metric_series_orders_points_chronologically() {
        let records = records_from_payload(
            "health_vital",
            &json!({"records": [
                {
                    "id": "bp-later",
                    "family": "health_vital",
                    "title": "Afternoon",
                    "fields": {"sys_bp": 125, "dia_bp": 82, "date": "2026-09-03T16:00:00Z"},
                    "created_at": 30, "updated_at": 30
                },
                {
                    "id": "bp-earlier",
                    "family": "health_vital",
                    "title": "Morning",
                    "fields": {"sys_bp": 118, "dia_bp": 78, "date": "2026-09-01T08:00:00Z"},
                    "created_at": 10, "updated_at": 10
                }
            ]}),
        );

        let series = extract_metric_series(&records, &MetricKind::BloodPressure);
        assert_eq!(series.len(), 1);
        assert_eq!(series[0].points.len(), 2);
        assert_eq!(series[0].points[0].record_id, "bp-earlier");
        assert_eq!(series[0].points[1].record_id, "bp-later");
    }

    #[test]
    fn available_metric_kinds_discovers_labs_and_vitals() {
        let records = records_from_payload(
            "health_lab",
            &json!({"records": [
                {
                    "id": "lab-ferritin",
                    "family": "health_lab",
                    "title": "Serum Ferritin",
                    "fields": {"analyte": "Ferritin", "value": 45, "unit": "µg/L"},
                    "created_at": 10, "updated_at": 10
                },
                {
                    "id": "lab-crp",
                    "family": "health_lab",
                    "title": "C-Reactive Protein",
                    "fields": {"analyte": "CRP", "value": 1.2, "unit": "mg/L"},
                    "created_at": 12, "updated_at": 12
                }
            ]}),
        );

        let kinds = available_metric_kinds(&records);
        assert!(kinds.contains(&MetricKind::BloodPressure));
        assert!(kinds.contains(&MetricKind::HeartRate));
        assert!(kinds.contains(&MetricKind::Glucose));
        assert!(kinds.contains(&MetricKind::Lab("Ferritin".into())));
        assert!(kinds.contains(&MetricKind::Lab("CRP".into())));
    }
}
