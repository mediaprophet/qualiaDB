use super::storage::{directory, object_path, publish_exact, read_association, verify};
use super::{DicomSourceArtifact, DicomSourceBudget, DicomSourceError};
use crate::dicom_ingest::DicomSeriesRecord;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Association {
    version: u8,
    source_blake3: [u8; 32],
    source_bytes: u64,
    patient: u64,
    series: u64,
    blob_offset: u64,
    blob_bytes: u32,
}
fn receipt_path(root: &Path, record: &DicomSeriesRecord) -> Result<PathBuf, DicomSourceError> {
    Ok(directory(root, "dicom_source_refs")?.join(format!(
        "{:016x}-{:08x}.json",
        record.blob_offset, record.blob_length
    )))
}
impl Association {
    fn matches(&self, record: &DicomSeriesRecord) -> bool {
        self.version == 1
            && self.patient == record.patient_did_hash
            && self.series == record.series_hash
            && self.blob_offset == record.blob_offset
            && self.blob_bytes == record.blob_length
    }
}

pub(super) fn associate(
    root: &Path,
    record: &DicomSeriesRecord,
    source: &DicomSourceArtifact,
) -> Result<(), DicomSourceError> {
    // Reconstruct paths from digest/record values, never from serialized path strings.
    if source.path != object_path(root, &source.blake3)? {
        return Err(DicomSourceError::InvalidPath);
    }
    let receipt = Association {
        version: 1,
        source_blake3: source.blake3,
        source_bytes: source.byte_length,
        patient: record.patient_did_hash,
        series: record.series_hash,
        blob_offset: record.blob_offset,
        blob_bytes: record.blob_length,
    };
    let bytes = serde_json::to_vec(&receipt).map_err(|_| DicomSourceError::InvalidAssociation)?;
    if bytes.len() > 2048 {
        return Err(DicomSourceError::InvalidAssociation);
    }
    publish_exact(&receipt_path(root, record)?, &bytes)
}

/// Resolve a successful ingest's durable association and verify the whole original
/// object's final-byte BLAKE3 before returning its canonical path. This also works
/// for records obtained from the existing asynchronous worker snapshot/status APIs.
pub fn source_for_record(
    root: &Path,
    record: &DicomSeriesRecord,
    budget: DicomSourceBudget,
) -> Result<DicomSourceArtifact, DicomSourceError> {
    let bytes = read_association(&receipt_path(root, record)?)?;
    let receipt: Association =
        serde_json::from_slice(&bytes).map_err(|_| DicomSourceError::InvalidAssociation)?;
    if !receipt.matches(record) {
        return Err(DicomSourceError::InvalidAssociation);
    }
    verify(root, receipt.source_blake3, receipt.source_bytes, budget)
}
