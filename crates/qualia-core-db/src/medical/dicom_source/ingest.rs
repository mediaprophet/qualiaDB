use super::association::associate;
use super::storage::{preserve, read_source};
use super::{DicomSourceArtifact, DicomSourceBudget};
use crate::dicom::{split_dicom_payload, DicomSplitPayload};
use crate::dicom_ingest::{self, DicomBlobStore, DicomIngestError, DicomSeriesRecord};
use crate::wal::WriteAheadLog;
use crate::{q_hash, NQuin};
use std::path::Path;

/// Additive result; the existing public series-record layout stays unchanged.
#[derive(Debug, Clone)]
pub struct DicomIngestWithSource {
    pub record: DicomSeriesRecord,
    pub source: DicomSourceArtifact,
}

/// Ingest admitted original bytes unchanged, preserving full source custody before
/// reporting success. Pixel bytes retain the existing append-only representation.
/// Metadata/encapsulated payloads are not decoded, deidentified or reserialized.
/// On later pixel/WAL/registry failure an admitted immutable source may remain;
/// this is not a transaction or aggregate retention/garbage-collection policy.
pub fn split_ingest_sync_with_source(
    storage_root: &Path,
    source_path: &Path,
    patient_did_hash: u64,
    budget: DicomSourceBudget,
) -> Result<DicomIngestWithSource, DicomIngestError> {
    let map_source = |e: super::DicomSourceError| DicomIngestError::Io(e.to_string());
    let file_bytes = read_source(source_path, budget).map_err(map_source)?;
    let DicomSplitPayload { meta, pixels } =
        split_dicom_payload(&file_bytes).map_err(|e| DicomIngestError::Parse(e.to_string()))?;
    let pixel_end = pixels
        .offset
        .checked_add(pixels.length)
        .ok_or_else(|| DicomIngestError::Parse("pixel extent overflow".into()))?;
    let pixel_bytes = file_bytes
        .get(pixels.offset..pixel_end)
        .ok_or_else(|| DicomIngestError::Parse("pixel extent outside original".into()))?;
    let blob_length = u32::try_from(pixels.length)
        .map_err(|_| DicomIngestError::Parse("pixel length exceeds blob record".into()))?;
    let source = preserve(storage_root, &file_bytes, budget).map_err(map_source)?;
    let mut store = DicomBlobStore::open(&dicom_ingest::blob_path(storage_root))
        .map_err(|e| DicomIngestError::Io(e.to_string()))?;
    let blob_offset = store
        .append_pixels(pixel_bytes)
        .map_err(|e| DicomIngestError::Io(e.to_string()))?;
    let series_hash = dicom_ingest::hash_series_uid(&meta.series_instance_uid);
    let patient = if patient_did_hash != 0 {
        patient_did_hash
    } else if !meta.patient_id.is_empty() {
        q_hash(&meta.patient_id)
    } else {
        q_hash("q42:anonymousPatient")
    };
    let mut quins = [NQuin::default(); 6];
    let quin_count = dicom_ingest::compile_semantic_quins(
        &meta,
        patient,
        series_hash,
        blob_offset,
        blob_length,
        &mut quins,
    );
    // Preserve existing best-effort WAL semantics; source publication does not
    // assert WAL durability or repair the preexisting multi-store transaction gap.
    if let Ok(mut wal) = WriteAheadLog::open(&dicom_ingest::wal_path(storage_root)) {
        for quin in &quins[..quin_count] {
            let _ = wal.append_mutation(quin);
        }
    }
    let record = DicomSeriesRecord {
        patient_did_hash: patient,
        series_hash,
        blob_offset,
        blob_length,
        rows: meta.rows,
        cols: meta.columns,
        organ_hash: dicom_ingest::infer_organ_hash(&meta),
        pixel_pointer_quin_parity: quins[quin_count.saturating_sub(1)].parity,
    };
    associate(storage_root, &record, &source).map_err(map_source)?;
    dicom_ingest::publish_series_record(record)?;
    Ok(DicomIngestWithSource { record, source })
}
