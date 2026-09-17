//! Domain models and zero-heap projection for Health Documents & Clinical Reports.
//!
//! Provides typed structures for:
//! - Health documents (`health_document`): Text-extract ingestion, document metadata,
//!   provenance, sensitivity classification (Restricted, Classified, Secret), and
//!   honest binary PDF / scan limitations.
//! - Clinical reports (`health_report`): Formal diagnostic summaries, consultation
//!   notes, pathology findings, and recommendations.

use super::model::HealthRecord;
use serde::{Deserialize, Serialize};

/// Supported clinical document types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DocumentType {
    DischargeSummary,
    PathologyReport,
    ClinicalNote,
    ConsultationLetter,
    ImagingReport,
    Other,
}

impl DocumentType {
    pub fn parse(s: &str) -> Self {
        match s.trim().to_lowercase().as_str() {
            "discharge_summary" | "discharge" => Self::DischargeSummary,
            "pathology_report" | "pathology" => Self::PathologyReport,
            "clinical_note" | "note" => Self::ClinicalNote,
            "consult_letter" | "consultation" | "consult" => Self::ConsultationLetter,
            "imaging_report" | "imaging" | "radiology" => Self::ImagingReport,
            _ => Self::Other,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::DischargeSummary => "discharge_summary",
            Self::PathologyReport => "pathology_report",
            Self::ClinicalNote => "clinical_note",
            Self::ConsultationLetter => "consult_letter",
            Self::ImagingReport => "imaging_report",
            Self::Other => "other",
        }
    }

    pub fn display_label(&self) -> &'static str {
        match self {
            Self::DischargeSummary => "Discharge Summary",
            Self::PathologyReport => "Pathology Report",
            Self::ClinicalNote => "Clinical Note",
            Self::ConsultationLetter => "Consultation Letter",
            Self::ImagingReport => "Imaging Report",
            Self::Other => "General Document",
        }
    }
}

/// Projected health document item.
#[derive(Debug, Clone, PartialEq)]
pub struct DocumentItem {
    pub record: HealthRecord,
    pub title: String,
    pub doc_type: DocumentType,
    pub author: Option<String>,
    pub facility: Option<String>,
    pub date: String,
    pub text: String,
    pub snippet: String,
    pub sensitivity: String,
    pub pipeline_status: String,
    pub source_uri: String,
}

/// Projects raw HealthRecords into structured DocumentItems.
pub fn project_documents(records: &[HealthRecord]) -> Vec<DocumentItem> {
    records
        .iter()
        .filter(|r| r.family == "health_document")
        .map(|r| {
            let title = r
                .field_text("title")
                .or_else(|| r.field_text("name"))
                .unwrap_or_else(|| r.title.clone());
            let doc_type = DocumentType::parse(
                &r.field_text("doc_type")
                    .or_else(|| r.field_text("type"))
                    .unwrap_or_default(),
            );
            let author = r.field_text("author");
            let facility = r.field_text("facility");
            let date = r
                .field_text("date")
                .or_else(|| r.field_text("occurred_at"))
                .unwrap_or_else(|| "Undated".into());
            let text = r.field_text("text").unwrap_or_default();
            let snippet = if text.chars().count() > 140 {
                format!("{}…", text.chars().take(137).collect::<String>())
            } else {
                text.clone()
            };
            let sensitivity = r
                .field_text("sensitivity")
                .unwrap_or_else(|| "classified".into());
            let pipeline_status = r
                .field_text("pipeline_status")
                .unwrap_or_else(|| "extracted_text_only".into());
            let source_uri = r.field_text("source_uri").unwrap_or_default();

            DocumentItem {
                record: r.clone(),
                title,
                doc_type,
                author,
                facility,
                date,
                text,
                snippet,
                sensitivity,
                pipeline_status,
                source_uri,
            }
        })
        .collect()
}

/// Builds the payload for saving a health document (`health_document`).
pub fn build_document_payload(
    title: &str,
    doc_type: &str,
    date: &str,
    author: Option<&str>,
    facility: Option<&str>,
    text: &str,
    sensitivity: &str,
    source_uri: &str,
    nlp_analyzed: bool,
) -> (String, String, serde_json::Map<String, serde_json::Value>) {
    let clean_title = title.trim().to_string();
    let mut fields = serde_json::Map::new();
    fields.insert(
        "title".into(),
        serde_json::Value::String(clean_title.clone()),
    );
    fields.insert(
        "name".into(),
        serde_json::Value::String(clean_title.clone()),
    );
    fields.insert(
        "doc_type".into(),
        serde_json::Value::String(doc_type.trim().to_string()),
    );
    fields.insert(
        "type".into(),
        serde_json::Value::String(doc_type.trim().to_string()),
    );
    fields.insert(
        "date".into(),
        serde_json::Value::String(date.trim().to_string()),
    );
    fields.insert(
        "occurred_at".into(),
        serde_json::Value::String(date.trim().to_string()),
    );

    if let Some(auth) = author.filter(|s| !s.trim().is_empty()) {
        fields.insert(
            "author".into(),
            serde_json::Value::String(auth.trim().to_string()),
        );
    }
    if let Some(fac) = facility.filter(|s| !s.trim().is_empty()) {
        fields.insert(
            "facility".into(),
            serde_json::Value::String(fac.trim().to_string()),
        );
    }

    let clean_text = text.trim().to_string();
    let snippet = if clean_text.chars().count() > 140 {
        format!("{}…", clean_text.chars().take(137).collect::<String>())
    } else {
        clean_text.clone()
    };
    fields.insert("text".into(), serde_json::Value::String(clean_text));
    fields.insert("snippet".into(), serde_json::Value::String(snippet));
    fields.insert(
        "sensitivity".into(),
        serde_json::Value::String(sensitivity.trim().to_string()),
    );

    let pipeline_status = if nlp_analyzed {
        "analyzed_and_ingested"
    } else {
        "extracted_text_only"
    };
    fields.insert(
        "pipeline_status".into(),
        serde_json::Value::String(pipeline_status.into()),
    );
    fields.insert(
        "source_uri".into(),
        serde_json::Value::String(source_uri.trim().to_string()),
    );

    ("health_document".into(), clean_title, fields)
}

/// Supported clinical report types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReportType {
    Consultation,
    Diagnostic,
    LabPanel,
    Operative,
    Pathology,
    Other,
}

impl ReportType {
    pub fn parse(s: &str) -> Self {
        match s.trim().to_lowercase().as_str() {
            "consultation" | "consult" => Self::Consultation,
            "diagnostic" | "summary" => Self::Diagnostic,
            "lab_panel" | "lab" => Self::LabPanel,
            "operative" | "surgery" => Self::Operative,
            "pathology" | "biopsy" => Self::Pathology,
            _ => Self::Other,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Consultation => "consultation",
            Self::Diagnostic => "diagnostic",
            Self::LabPanel => "lab_panel",
            Self::Operative => "operative",
            Self::Pathology => "pathology",
            Self::Other => "other",
        }
    }

    pub fn display_label(&self) -> &'static str {
        match self {
            Self::Consultation => "Consultation",
            Self::Diagnostic => "Diagnostic Summary",
            Self::LabPanel => "Lab Panel Report",
            Self::Operative => "Operative Report",
            Self::Pathology => "Pathology Report",
            Self::Other => "Clinical Report",
        }
    }
}

/// Projected clinical report item.
#[derive(Debug, Clone, PartialEq)]
pub struct ReportItem {
    pub record: HealthRecord,
    pub title: String,
    pub report_type: ReportType,
    pub author: Option<String>,
    pub facility: Option<String>,
    pub date: String,
    pub findings: Option<String>,
    pub recommendations: Option<String>,
    pub sensitivity: String,
}

/// Projects raw HealthRecords into structured ReportItems.
pub fn project_reports(records: &[HealthRecord]) -> Vec<ReportItem> {
    records
        .iter()
        .filter(|r| r.family == "health_report")
        .map(|r| {
            let title = r
                .field_text("title")
                .or_else(|| r.field_text("name"))
                .unwrap_or_else(|| r.title.clone());
            let report_type = ReportType::parse(
                &r.field_text("report_type")
                    .or_else(|| r.field_text("type"))
                    .unwrap_or_default(),
            );
            let author = r.field_text("author");
            let facility = r.field_text("facility");
            let date = r
                .field_text("date")
                .or_else(|| r.field_text("occurred_at"))
                .unwrap_or_else(|| "Undated".into());
            let findings = r.field_text("findings");
            let recommendations = r.field_text("recommendations");
            let sensitivity = r
                .field_text("sensitivity")
                .unwrap_or_else(|| "restricted".into());

            ReportItem {
                record: r.clone(),
                title,
                report_type,
                author,
                facility,
                date,
                findings,
                recommendations,
                sensitivity,
            }
        })
        .collect()
}

/// Builds the payload for saving a clinical report (`health_report`).
pub fn build_report_payload(
    title: &str,
    report_type: &str,
    date: &str,
    author: Option<&str>,
    facility: Option<&str>,
    findings: Option<&str>,
    recommendations: Option<&str>,
    sensitivity: &str,
) -> (String, String, serde_json::Map<String, serde_json::Value>) {
    let clean_title = title.trim().to_string();
    let mut fields = serde_json::Map::new();
    fields.insert(
        "title".into(),
        serde_json::Value::String(clean_title.clone()),
    );
    fields.insert(
        "name".into(),
        serde_json::Value::String(clean_title.clone()),
    );
    fields.insert(
        "report_type".into(),
        serde_json::Value::String(report_type.trim().to_string()),
    );
    fields.insert(
        "type".into(),
        serde_json::Value::String(report_type.trim().to_string()),
    );
    fields.insert(
        "date".into(),
        serde_json::Value::String(date.trim().to_string()),
    );
    fields.insert(
        "occurred_at".into(),
        serde_json::Value::String(date.trim().to_string()),
    );

    if let Some(auth) = author.filter(|s| !s.trim().is_empty()) {
        fields.insert(
            "author".into(),
            serde_json::Value::String(auth.trim().to_string()),
        );
    }
    if let Some(fac) = facility.filter(|s| !s.trim().is_empty()) {
        fields.insert(
            "facility".into(),
            serde_json::Value::String(fac.trim().to_string()),
        );
    }
    if let Some(f) = findings.filter(|s| !s.trim().is_empty()) {
        fields.insert(
            "findings".into(),
            serde_json::Value::String(f.trim().to_string()),
        );
    }
    if let Some(rec) = recommendations.filter(|s| !s.trim().is_empty()) {
        fields.insert(
            "recommendations".into(),
            serde_json::Value::String(rec.trim().to_string()),
        );
    }
    fields.insert(
        "sensitivity".into(),
        serde_json::Value::String(sensitivity.trim().to_string()),
    );

    ("health_report".into(), clean_title, fields)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::browser::health_views::model::records_from_payload;
    use serde_json::json;

    #[test]
    fn document_type_parsing_and_labels() {
        assert_eq!(
            DocumentType::parse("discharge_summary"),
            DocumentType::DischargeSummary
        );
        assert_eq!(
            DocumentType::parse("pathology"),
            DocumentType::PathologyReport
        );
        assert_eq!(
            DocumentType::parse("consult_letter"),
            DocumentType::ConsultationLetter
        );
        assert_eq!(DocumentType::parse("imaging"), DocumentType::ImagingReport);
        assert_eq!(
            DocumentType::parse("clinical_note"),
            DocumentType::ClinicalNote
        );
        assert_eq!(DocumentType::parse("random"), DocumentType::Other);

        assert_eq!(
            DocumentType::DischargeSummary.display_label(),
            "Discharge Summary"
        );
        assert_eq!(
            DocumentType::PathologyReport.display_label(),
            "Pathology Report"
        );
    }

    #[test]
    fn report_type_parsing_and_labels() {
        assert_eq!(ReportType::parse("consultation"), ReportType::Consultation);
        assert_eq!(ReportType::parse("diagnostic"), ReportType::Diagnostic);
        assert_eq!(ReportType::parse("lab_panel"), ReportType::LabPanel);
        assert_eq!(ReportType::parse("operative"), ReportType::Operative);
        assert_eq!(ReportType::parse("pathology"), ReportType::Pathology);
        assert_eq!(ReportType::parse("unknown"), ReportType::Other);

        assert_eq!(ReportType::Consultation.display_label(), "Consultation");
        assert_eq!(ReportType::LabPanel.display_label(), "Lab Panel Report");
    }

    #[test]
    fn project_documents_extracts_typed_metadata_and_snippet() {
        let long_text = "Patient was admitted for acute evaluation following progressive dyspnea on exertion. Echocardiogram revealed preserved ejection fraction of 55% with mild diastolic dysfunction.";
        let payload = json!({
            "records": [
                {
                    "id": "doc-1",
                    "family": "health_document",
                    "title": "Hospital Discharge Summary",
                    "fields": {
                        "doc_type": "discharge_summary",
                        "author": "Dr. Sarah Chen",
                        "facility": "Metro Health",
                        "date": "2024-02-14",
                        "text": long_text,
                        "sensitivity": "classified",
                        "source_uri": "urn:poet:health:doc:12345"
                    }
                },
                {
                    "id": "doc-2",
                    "family": "health_document",
                    "title": "Histopathology Biopsy",
                    "fields": {
                        "doc_type": "pathology_report",
                        "author": "Dr. Marcus Vance",
                        "facility": "St. Jude Pathology",
                        "date": "2024-01-10",
                        "text": "Benign tissue specimen.",
                        "sensitivity": "secret",
                        "source_uri": "urn:poet:health:doc:67890"
                    }
                }
            ]
        });

        let records = records_from_payload("health_document", &payload);
        let items = project_documents(&records);
        assert_eq!(items.len(), 2);

        let doc1 = items
            .iter()
            .find(|d| d.title == "Hospital Discharge Summary")
            .unwrap();
        assert_eq!(doc1.doc_type, DocumentType::DischargeSummary);
        assert_eq!(doc1.author.as_deref(), Some("Dr. Sarah Chen"));
        assert_eq!(doc1.facility.as_deref(), Some("Metro Health"));
        assert_eq!(doc1.sensitivity, "classified");
        assert!(doc1.snippet.ends_with('…'));

        let doc2 = items
            .iter()
            .find(|d| d.title == "Histopathology Biopsy")
            .unwrap();
        assert_eq!(doc2.doc_type, DocumentType::PathologyReport);
        assert_eq!(doc2.sensitivity, "secret");
        assert_eq!(doc2.snippet, "Benign tissue specimen.");
    }

    #[test]
    fn project_reports_extracts_findings_and_recommendations() {
        let payload = json!({
            "records": [
                {
                    "id": "rep-1",
                    "family": "health_report",
                    "title": "Cardiology Consultation",
                    "fields": {
                        "report_type": "consultation",
                        "author": "Dr. Elena Rostova",
                        "facility": "Cardiology Associates",
                        "date": "2024-02-01",
                        "findings": "Sinus rhythm with rare premature ventricular contractions.",
                        "recommendations": "Continue current antihypertensive therapy. Follow up in 6 months.",
                        "sensitivity": "restricted"
                    }
                }
            ]
        });

        let records = records_from_payload("health_report", &payload);
        let items = project_reports(&records);
        assert_eq!(items.len(), 1);

        let rep = &items[0];
        assert_eq!(rep.report_type, ReportType::Consultation);
        assert_eq!(rep.author.as_deref(), Some("Dr. Elena Rostova"));
        assert_eq!(
            rep.findings.as_deref(),
            Some("Sinus rhythm with rare premature ventricular contractions.")
        );
        assert_eq!(
            rep.recommendations.as_deref(),
            Some("Continue current antihypertensive therapy. Follow up in 6 months.")
        );
    }

    #[test]
    fn build_payloads_generate_correct_fields() {
        let (d_fam, d_title, d_fields) = build_document_payload(
            "Pathology Result",
            "pathology_report",
            "2024-03-01",
            Some("Dr. Chen"),
            Some("Metro Lab"),
            "Specimen examined: negative for malignancy.",
            "secret",
            "urn:poet:health:doc:999",
            true,
        );
        assert_eq!(d_fam, "health_document");
        assert_eq!(d_title, "Pathology Result");
        assert_eq!(d_fields.get("doc_type").unwrap(), "pathology_report");
        assert_eq!(d_fields.get("sensitivity").unwrap(), "secret");
        assert_eq!(
            d_fields.get("pipeline_status").unwrap(),
            "analyzed_and_ingested"
        );

        let (r_fam, r_title, r_fields) = build_report_payload(
            "Surgical Note",
            "operative",
            "2024-02-15",
            Some("Dr. Surgeon"),
            None,
            Some("Uncomplicated procedure."),
            Some("Wound care daily."),
            "restricted",
        );
        assert_eq!(r_fam, "health_report");
        assert_eq!(r_title, "Surgical Note");
        assert_eq!(r_fields.get("report_type").unwrap(), "operative");
        assert_eq!(
            r_fields.get("findings").unwrap(),
            "Uncomplicated procedure."
        );
    }
}
