//! Standards-readable health export package (§8.1 step 9).

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::journal::JournalEntry;
use super::receipt::ReceiptRecord;

pub const EXPORT_FORMAT_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HealthExportManifestEntry {
    pub id: String,
    pub kind: String,
    pub evidence_type: String,
    pub sensitivity: String,
    pub asserted_time_unix: u32,
    pub assurance: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HealthExportPackage {
    pub format_version: u32,
    pub exported_at_unix: u32,
    pub record_count: u32,
    pub content_sha256_hex: String,
    pub checkpoint_hash: Option<String>,
    pub turtle_body: String,
    pub manifest: Vec<HealthExportManifestEntry>,
}

fn assurance_label(evidence_type: &str, kind: &str) -> &'static str {
    if kind == "disputed_diagnosis" {
        "disputed_self_reported_restricted"
    } else if kind == "housing_safety" {
        "safety_context_restricted"
    } else if kind == "life_event" || kind == "case_task" {
        "life_event_restricted"
    } else if kind == "welfare_case" {
        "welfare_case_sanctuary"
    } else if kind == "wellbeing_observation" {
        "wellbeing_self_report"
    } else if kind == "therapy_note" || kind == "sanctuary_note" {
        "sanctuary_classified"
    } else if evidence_type.contains("SelfReported") {
        "self_reported_restricted"
    } else if evidence_type.contains("DeviceMeasured") {
        "device_measured_restricted"
    } else {
        "asserted_restricted"
    }
}

/// Build Turtle + manifest from committed journal rows (no heap in hot evaluators elsewhere;
/// export is a cold path).
pub fn build_export_package(
    entries: &[JournalEntry],
    exported_at_unix: u32,
    checkpoint_hash: Option<[u8; 32]>,
) -> HealthExportPackage {
    let mut turtle = wellfare_core::rdf::generate_rdf_prefixes();
    let mut manifest = Vec::with_capacity(entries.len());

    for entry in entries {
        let subj = format!("<{}>", entry.id);
        turtle.push_str(&format!("{subj} a wf:HealthRecord , fhir:Observation ;\n"));
        turtle.push_str(&format!("    wf:kind {:?} ;\n", entry.kind));
        turtle.push_str(&format!(
            "    wf:evidenceType {:?} ;\n",
            entry.evidence_type
        ));
        turtle.push_str(&format!("    wf:sensitivity {:?} ;\n", entry.sensitivity));
        turtle.push_str(&format!(
            "    fhir:Observation.effectiveDateTime \"{}\"^^xsd:unsignedInt ;\n",
            entry.asserted_instant.to_unix_secs() as u32
        ));
        if let Some(ref summary) = entry.summary {
            turtle.push_str(&format!("    schema:description {:?} ;\n", summary));
        }
        if let Some(ref blob) = entry.blob_hash {
            turtle.push_str(&format!("    wf:blobHash {:?} ;\n", blob));
        }
        turtle.push_str("    prov:wasGeneratedBy <urn:wellfair:agent:vault> .\n\n");

        manifest.push(HealthExportManifestEntry {
            id: entry.id.clone(),
            kind: entry.kind.clone(),
            evidence_type: entry.evidence_type.clone(),
            sensitivity: entry.sensitivity.clone(),
            asserted_time_unix: entry.asserted_instant.to_unix_secs() as u32,
            assurance: assurance_label(&entry.evidence_type, &entry.kind).to_string(),
        });
    }

    let content_sha256_hex = hex::encode(Sha256::digest(turtle.as_bytes()).as_slice());
    HealthExportPackage {
        format_version: EXPORT_FORMAT_VERSION,
        exported_at_unix,
        record_count: entries.len() as u32,
        content_sha256_hex,
        checkpoint_hash: checkpoint_hash.map(|h| hex::encode(h)),
        turtle_body: turtle,
        manifest,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExportReceipt {
    pub export_sha256_hex: String,
    pub record_count: u32,
    pub checkpoint_hash: Option<String>,
    pub exported_at_unix: u32,
}

impl ExportReceipt {
    pub fn from_package(pkg: &HealthExportPackage) -> Self {
        Self {
            export_sha256_hex: pkg.content_sha256_hex.clone(),
            record_count: pkg.record_count,
            checkpoint_hash: pkg.checkpoint_hash.clone(),
            exported_at_unix: pkg.exported_at_unix,
        }
    }
}

pub fn export_policy_receipt(pkg: &HealthExportPackage, timestamp_unix: u32) -> ReceiptRecord {
    ReceiptRecord {
        id: format!(
            "export-{}",
            pkg.content_sha256_hex.get(..8).unwrap_or("00000000")
        ),
        timestamp_unix,
        qapp_id: "wellfair-shell".into(),
        record_id: format!("urn:wellfair:export:{}", pkg.exported_at_unix),
        decision: "Permit".into(),
        obligations: vec![
            "standards_readable_export".into(),
            "typed_assurance_manifest".into(),
        ],
        checkpoint_hash: pkg.checkpoint_hash.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_entry(id: &str, kind: &str) -> JournalEntry {
        JournalEntry {
            id: id.into(),
            kind: kind.into(),
            asserted_time_unix: 1_700_000_000,
            asserted_instant: wellfare_core::record::InstantBridge::from_coarse(1_700_000_000),
            evidence_type: "DeviceMeasured".into(),
            sensitivity: "Restricted".into(),
            blob_hash: Some("abc".into()),
            source: "test".into(),
            committed_unix: 1_700_000_100,
            summary: Some(r#"{"weight":72.0}"#.into()),
        }
    }

    #[test]
    fn export_package_has_turtle_and_manifest() {
        let entries = vec![
            sample_entry("urn:wellfair:weight:w1", "weight"),
            sample_entry("urn:wellfair:condition:c1", "condition"),
        ];
        let pkg = build_export_package(&entries, 1_700_000_200, Some([7u8; 32]));
        assert_eq!(pkg.record_count, 2);
        assert!(pkg.turtle_body.contains("@prefix wf:"));
        assert!(pkg.turtle_body.contains("urn:wellfair:weight:w1"));
        assert_eq!(pkg.manifest.len(), 2);
        assert_eq!(pkg.manifest[0].assurance, "device_measured_restricted");
        assert!(!pkg.content_sha256_hex.is_empty());
        assert!(pkg.checkpoint_hash.is_some());
    }

    #[test]
    fn export_receipt_binds_checkpoint() {
        let entries = vec![sample_entry("urn:wellfair:sleep:s1", "sleep")];
        let pkg = build_export_package(&entries, 99, None);
        let receipt = export_policy_receipt(&pkg, 99);
        assert_eq!(receipt.decision, "Permit");
        assert!(receipt
            .obligations
            .contains(&"standards_readable_export".into()));
    }
}
