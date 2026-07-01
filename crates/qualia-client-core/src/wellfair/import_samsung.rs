//! Samsung Health import — companion bundle (primary) and folder scan (dev fallback).

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use wellfare_core::companion_sync::CompanionHealthBundle;
use wellfare_core::models::{HeartRateRecord, SleepRecord, StepRecord, WeightRecord};
use wellfare_core::parser::{
    parse_heart_rate_csv, parse_sleep_csv, parse_steps_csv, parse_weight_csv,
};
use wellfare_core::record::{
    EpistemicStatus, EvidenceType, RecordEnvelope, SensitivityClass,
};

use super::api::WebizenHostApi;

const QAPP_HEALTH: &str = "wellfair-health";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SamsungFileReport {
    pub path: String,
    pub kind: String,
    pub records: u32,
    pub rejected: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SamsungImportReport {
    pub source: String,
    pub files: Vec<SamsungFileReport>,
    pub records_committed: usize,
    pub records_skipped: usize,
    pub errors: Vec<String>,
}

fn content_hash_hex(payload: &str) -> String {
    hex::encode(Sha256::digest(payload.as_bytes()).as_slice())
}

fn envelope_from_parts(
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
        evidence_type: EvidenceType::DeviceMeasured,
        sensitivity: SensitivityClass::Restricted,
        asserted_time_unix: asserted_unix,
        valid_time_start_unix: Some(asserted_unix),
        valid_time_end_unix: None,
        predecessor_id: None,
        blob_hash: Some(content_hash_hex(payload_json)),
        tombstone: false,
    }
}

fn weight_envelopes(records: &[WeightRecord], owner: &str, author: &str) -> Vec<RecordEnvelope> {
    records
        .iter()
        .filter_map(|r| {
            let payload = serde_json::to_string(r).ok()?;
            let id = format!("urn:wellfair:weight:{}", r.uuid);
            let unix = r.start_datetime.timestamp().max(0) as u32;
            Some(envelope_from_parts(&id, owner, author, unix, &payload))
        })
        .collect()
}

fn sleep_envelopes(records: &[SleepRecord], owner: &str, author: &str) -> Vec<RecordEnvelope> {
    records
        .iter()
        .filter_map(|r| {
            let payload = serde_json::to_string(r).ok()?;
            let id = format!("urn:wellfair:sleep:{}", r.uuid);
            let unix = r.start_datetime.timestamp().max(0) as u32;
            Some(envelope_from_parts(&id, owner, author, unix, &payload))
        })
        .collect()
}

fn heart_rate_envelopes(records: &[HeartRateRecord], owner: &str, author: &str) -> Vec<RecordEnvelope> {
    records
        .iter()
        .filter_map(|r| {
            let payload = serde_json::to_string(r).ok()?;
            let id = format!("urn:wellfair:heart_rate:{}", r.uuid);
            let unix = r.start_datetime.timestamp().max(0) as u32;
            Some(envelope_from_parts(&id, owner, author, unix, &payload))
        })
        .collect()
}

fn steps_envelopes(records: &[StepRecord], owner: &str, author: &str) -> Vec<RecordEnvelope> {
    records
        .iter()
        .filter_map(|r| {
            let payload = serde_json::to_string(r).ok()?;
            let id = format!("urn:wellfair:steps:{}", r.uuid);
            let unix = r.start_datetime.timestamp().max(0) as u32;
            Some(envelope_from_parts(&id, owner, author, unix, &payload))
        })
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SamsungCsvKind {
    Weight,
    Sleep,
    HeartRate,
    Steps,
    Unknown,
}

fn classify_samsung_csv(name: &str) -> SamsungCsvKind {
    let lower = name.to_ascii_lowercase();
    if lower.contains("weight") || lower.contains("body_composition") {
        SamsungCsvKind::Weight
    } else if lower.contains("sleep") {
        SamsungCsvKind::Sleep
    } else if lower.contains("heart") {
        SamsungCsvKind::HeartRate
    } else if lower.contains("step") || lower.contains("walk") {
        SamsungCsvKind::Steps
    } else {
        SamsungCsvKind::Unknown
    }
}

fn kind_label(kind: SamsungCsvKind) -> &'static str {
    match kind {
        SamsungCsvKind::Weight => "weight",
        SamsungCsvKind::Sleep => "sleep",
        SamsungCsvKind::HeartRate => "heart_rate",
        SamsungCsvKind::Steps => "steps",
        SamsungCsvKind::Unknown => "unknown",
    }
}

/// Parse Samsung CSV content using filename heuristics (transport-neutral).
pub fn parse_csv_named_content(
    filename: &str,
    content: &str,
    owner_did: &str,
    author_did: &str,
) -> Result<(SamsungCsvKind, Vec<RecordEnvelope>, u32), String> {
    let kind = classify_samsung_csv(filename);
    match kind {
        SamsungCsvKind::Weight => {
            let records = parse_weight_csv(content).map_err(|e| e.to_string())?;
            Ok((kind, weight_envelopes(&records, owner_did, author_did), 0))
        }
        SamsungCsvKind::Sleep => {
            let records = parse_sleep_csv(content).map_err(|e| e.to_string())?;
            Ok((kind, sleep_envelopes(&records, owner_did, author_did), 0))
        }
        SamsungCsvKind::HeartRate => {
            let records = parse_heart_rate_csv(content).map_err(|e| e.to_string())?;
            Ok((kind, heart_rate_envelopes(&records, owner_did, author_did), 0))
        }
        SamsungCsvKind::Steps => {
            let records = parse_steps_csv(content).map_err(|e| e.to_string())?;
            Ok((kind, steps_envelopes(&records, owner_did, author_did), 0))
        }
        SamsungCsvKind::Unknown => Ok((kind, Vec::new(), 0)),
    }
}

fn parse_csv_file(
    path: &Path,
    owner_did: &str,
    author_did: &str,
) -> Result<(SamsungCsvKind, Vec<RecordEnvelope>, u32), String> {
    let content = fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");
    parse_csv_named_content(name, &content, owner_did, author_did)
}

fn commit_envelopes(
    host: &mut WebizenHostApi,
    report: &mut SamsungImportReport,
    path_label: &str,
    kind: SamsungCsvKind,
    envelopes: Vec<RecordEnvelope>,
    rejected: u32,
) {
    let mut committed = 0u32;
    for envelope in envelopes {
        match host.submit_record(QAPP_HEALTH, envelope) {
            Ok(n) => {
                report.records_committed += n;
                committed += 1;
            }
            Err(e) => {
                report.records_skipped += 1;
                report.errors.push(e);
            }
        }
    }
    report.files.push(SamsungFileReport {
        path: path_label.to_string(),
        kind: kind_label(kind).to_string(),
        records: committed,
        rejected,
    });
}

/// Primary path: ingest a companion bundle produced on the user's phone.
pub fn ingest_companion_health_bundle(
    host: &mut WebizenHostApi,
    bundle: &CompanionHealthBundle,
    owner_did: &str,
    author_did: &str,
) -> SamsungImportReport {
    let mut report = SamsungImportReport {
        source: format!("companion:{}", bundle.device_id),
        files: Vec::new(),
        records_committed: 0,
        records_skipped: 0,
        errors: Vec::new(),
    };

    if let Err(e) = bundle.validate() {
        report.errors.push(e);
        return report;
    }

    for file in &bundle.files {
        match parse_csv_named_content(&file.filename, &file.csv_content, owner_did, author_did) {
            Ok((kind, envelopes, rejected)) => {
                commit_envelopes(host, &mut report, &file.filename, kind, envelopes, rejected);
            }
            Err(e) => {
                report.errors.push(e);
                report.files.push(SamsungFileReport {
                    path: file.filename.clone(),
                    kind: "error".into(),
                    records: 0,
                    rejected: 1,
                });
            }
        }
    }

    report
}

/// Dev/testing fallback: scan a desktop folder for Samsung Health CSV exports.
pub fn import_samsung_folder(
    host: &mut WebizenHostApi,
    folder: &Path,
    owner_did: &str,
    author_did: &str,
) -> SamsungImportReport {
    let mut report = SamsungImportReport {
        source: format!("folder:{}", folder.display()),
        files: Vec::new(),
        records_committed: 0,
        records_skipped: 0,
        errors: Vec::new(),
    };

    if !folder.is_dir() {
        report.errors.push(format!("Not a directory: {}", folder.display()));
        return report;
    }

    let mut csv_paths: Vec<PathBuf> = Vec::new();
    let entries = match fs::read_dir(folder) {
        Ok(e) => e,
        Err(e) => {
            report.errors.push(format!("read_dir: {e}"));
            return report;
        }
    };

    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("csv") {
            csv_paths.push(path);
        }
    }
    csv_paths.sort();

    for path in csv_paths {
        let label = path.display().to_string();
        match parse_csv_file(&path, owner_did, author_did) {
            Ok((kind, envelopes, rejected)) => {
                commit_envelopes(host, &mut report, &label, kind, envelopes, rejected);
            }
            Err(e) => {
                report.errors.push(e);
                report.files.push(SamsungFileReport {
                    path: label,
                    kind: "error".into(),
                    records: 0,
                    rejected: 1,
                });
            }
        }
    }

    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use wellfare_core::companion_sync::{CompanionCsvFile, CompanionHealthBundle};

    #[test]
    fn classifies_samsung_csv_names() {
        assert_eq!(
            classify_samsung_csv("com.samsung.health.weight.20260101.csv"),
            SamsungCsvKind::Weight
        );
        assert_eq!(classify_samsung_csv("sleep.csv"), SamsungCsvKind::Sleep);
    }

    #[test]
    fn weight_csv_produces_envelopes() {
        let dir = tempfile::tempdir().unwrap();
        let csv = dir.path().join("weight.csv");
        let mut f = fs::File::create(&csv).unwrap();
        writeln!(
            f,
            "uuid,start_time,end_time,time_offset,weight,body_fat,muscle_mass,body_water,skeletal_muscle,bmi"
        )
        .unwrap();
        writeln!(
            f,
            "a1000001-0000-4000-8000-000000000001,1777632000000,1777632060000,60,72.0,18.5,32.1,55.2,30.5,23.1"
        )
        .unwrap();

        let (kind, envelopes, _) =
            parse_csv_file(&csv, "did:wf:owner", "did:wf:owner").unwrap();
        assert_eq!(kind, SamsungCsvKind::Weight);
        assert_eq!(envelopes.len(), 1);
        assert_eq!(envelopes[0].evidence_type, EvidenceType::DeviceMeasured);
    }

    #[test]
    fn companion_bundle_parses_named_content() {
        let csv = "uuid,start_time,end_time,time_offset,weight,body_fat,muscle_mass,body_water,skeletal_muscle,bmi\n\
                   a1000001-0000-4000-8000-000000000001,1777632000000,1777632060000,60,72.0,18.5,32.1,55.2,30.5,23.1\n";
        let (kind, envelopes, _) =
            parse_csv_named_content("weight.csv", csv, "did:wf:owner", "did:wf:owner").unwrap();
        assert_eq!(kind, SamsungCsvKind::Weight);
        assert_eq!(envelopes.len(), 1);

        let bundle = CompanionHealthBundle::new(
            "pixel-test",
            1_700_000_000,
            vec![CompanionCsvFile {
                filename: "weight.csv".into(),
                csv_content: csv.into(),
            }],
        );
        assert!(bundle.validate().is_ok());
    }
}