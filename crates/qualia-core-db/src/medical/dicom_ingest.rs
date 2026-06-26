//! Core 3 DICOM split-ingestion pipeline.
//!
//! Pixel payloads stream to a memory-mapped blob store; semantic metadata becomes
//! 48-byte Quins appended to the conduct WAL. Core 1 never blocks on I/O — jobs are
//! handed to a dedicated Swarm worker via a lock-free channel.

use crate::dicom::{
    decode_blob_pointer, encode_blob_pointer, pack_volume_metadata, split_dicom_payload,
    DicomMetadata, DicomSplitPayload,
};
use crate::q_hash;
use crate::wal::WriteAheadLog;
use crate::NQuin;
use crossbeam_channel::{bounded, Receiver, Sender};
use memmap2::Mmap;
use std::cell::UnsafeCell;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, AtomicU64, AtomicU8, AtomicUsize, Ordering};
use std::sync::OnceLock;
use std::thread;

pub const BLOB_FILE_NAME: &str = "dicom_pixels.blob";
pub const MAX_SERIES_RECORDS: usize = 64;
pub const JOB_PENDING: u8 = 0;
pub const JOB_COMPLETE: u8 = 1;
pub const JOB_FAILED: u8 = 2;

/// Published series record — single-writer (Core 3), lock-free readers (Core 1).
#[derive(Debug, Clone, Copy, Default)]
pub struct DicomSeriesRecord {
    pub patient_did_hash: u64,
    pub series_hash: u64,
    pub blob_offset: u64,
    pub blob_length: u32,
    pub rows: u16,
    pub cols: u16,
    pub organ_hash: u64,
    pub pixel_pointer_quin_parity: u64,
}

struct IngestJob {
    job_id: u64,
    source_path: PathBuf,
    patient_did_hash: u64,
    storage_root: PathBuf,
}

struct JobSlot {
    status: AtomicU8,
    series_hash: AtomicU64,
    blob_offset: AtomicU64,
    blob_length: AtomicU32,
}

impl JobSlot {
    const fn new() -> Self {
        Self {
            status: AtomicU8::new(JOB_PENDING),
            series_hash: AtomicU64::new(0),
            blob_offset: AtomicU64::new(0),
            blob_length: AtomicU32::new(0),
        }
    }
}

static IO_TX: OnceLock<Sender<IngestJob>> = OnceLock::new();
static NEXT_JOB_ID: AtomicU64 = AtomicU64::new(1);
static JOB_SLOTS: [JobSlot; MAX_SERIES_RECORDS] = [const { JobSlot::new() }; MAX_SERIES_RECORDS];
const EMPTY_SERIES_RECORD: DicomSeriesRecord = DicomSeriesRecord {
    patient_did_hash: 0,
    series_hash: 0,
    blob_offset: 0,
    blob_length: 0,
    rows: 0,
    cols: 0,
    organ_hash: 0,
    pixel_pointer_quin_parity: 0,
};
struct SyncSeriesRegistry([UnsafeCell<DicomSeriesRecord>; MAX_SERIES_RECORDS]);
// SAFETY: Core 3 is the sole writer; readers bound indices via SERIES_COUNT Acquire.
unsafe impl Sync for SyncSeriesRegistry {}

static SERIES_RECORDS: SyncSeriesRegistry =
    SyncSeriesRegistry([const { UnsafeCell::new(EMPTY_SERIES_RECORD) }; MAX_SERIES_RECORDS]);
static SERIES_COUNT: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, PartialEq, Eq)]
pub enum DicomIngestError {
    WorkerUnavailable,
    Io(String),
    Parse(String),
    RegistryFull,
    JobNotFound,
    BlobRead(String),
}

impl std::fmt::Display for DicomIngestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WorkerUnavailable => write!(f, "Core 3 DICOM worker not initialized"),
            Self::Io(msg) => write!(f, "DICOM ingest IO: {msg}"),
            Self::Parse(msg) => write!(f, "DICOM ingest parse: {msg}"),
            Self::RegistryFull => write!(f, "DICOM series registry full"),
            Self::JobNotFound => write!(f, "DICOM ingest job not found"),
            Self::BlobRead(msg) => write!(f, "DICOM blob read: {msg}"),
        }
    }
}

impl std::error::Error for DicomIngestError {}

/// Append-only pixel blob store (Core 3 writer).
pub struct DicomBlobStore {
    file: File,
    write_offset: u64,
}

impl DicomBlobStore {
    pub fn open(path: &Path) -> Result<Self, std::io::Error> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)?;
        let len = file.metadata()?.len();
        Ok(Self {
            file,
            write_offset: len,
        })
    }

    pub fn append_pixels(&mut self, pixels: &[u8]) -> Result<u64, std::io::Error> {
        let offset = self.write_offset;
        self.file.seek(SeekFrom::Start(offset))?;
        self.file.write_all(pixels)?;
        self.file.sync_data()?;
        self.write_offset = offset.saturating_add(pixels.len() as u64);
        Ok(offset)
    }

    pub fn path_mmap(&self) -> Result<Mmap, std::io::Error> {
        unsafe { memmap2::MmapOptions::new().map(&self.file) }
    }
}

/// Zero-copy slice reader over the blob file (Core 1 / FRB boundary).
pub struct DicomBlobReader {
    mmap: Mmap,
}

impl DicomBlobReader {
    pub fn open(path: &Path) -> Result<Self, DicomIngestError> {
        let file = File::open(path).map_err(|e| DicomIngestError::Io(e.to_string()))?;
        let mmap = unsafe { memmap2::MmapOptions::new().map(&file) }
            .map_err(|e| DicomIngestError::Io(e.to_string()))?;
        Ok(Self { mmap })
    }

    #[inline]
    pub fn slice(&self, offset: usize, length: usize) -> Result<&[u8], DicomIngestError> {
        let end = offset
            .checked_add(length)
            .ok_or_else(|| DicomIngestError::BlobRead("overflow".into()))?;
        if end > self.mmap.len() {
            return Err(DicomIngestError::BlobRead("range past blob end".into()));
        }
        Ok(&self.mmap[offset..end])
    }
}

fn imaging_dir(storage_root: &Path) -> PathBuf {
    storage_root.join("Imaging")
}

fn blob_path(storage_root: &Path) -> PathBuf {
    imaging_dir(storage_root).join(BLOB_FILE_NAME)
}

fn wal_path(storage_root: &Path) -> PathBuf {
    storage_root.join("qualia_global.wal")
}

fn hash_series_uid(uid: &str) -> u64 {
    if uid.is_empty() {
        return 0;
    }
    q_hash(uid)
}

fn infer_organ_hash(meta: &DicomMetadata) -> u64 {
    if let Some(organ) =
        crate::dicom::infer_organ_from_metadata(meta, &crate::dicom::default_organ_matchers())
    {
        q_hash(&organ)
    } else {
        0
    }
}

fn compile_semantic_quins(
    meta: &DicomMetadata,
    patient_did_hash: u64,
    series_hash: u64,
    blob_offset: u64,
    blob_length: u32,
    out: &mut [NQuin; 6],
) -> usize {
    let study_ctx = q_hash("q42:imagingStudy");
    let modality_hash = q_hash(&meta.modality);
    let pred_has_series = q_hash("q42:hasDicomSeries");
    let pred_modality = q_hash("q42:hasModality");
    let pred_pixel_ptr = q_hash("q42:pixelBlobPointer");
    let pred_body_part = q_hash("q42:bodyPartExamined");

    let mut count = 0usize;

    let mut q0 = NQuin::default();
    q0.subject = patient_did_hash;
    q0.predicate = pred_has_series;
    q0.object = series_hash;
    q0.context = study_ctx;
    q0.metadata = pack_volume_metadata(meta.rows, meta.columns, blob_length);
    q0.parity = q0.subject ^ q0.predicate ^ q0.object ^ q0.context;
    out[count] = q0;
    count += 1;

    let mut q1 = NQuin::default();
    q1.subject = series_hash;
    q1.predicate = pred_modality;
    q1.object = modality_hash;
    q1.context = study_ctx;
    q1.parity = q1.subject ^ q1.predicate ^ q1.object ^ q1.context;
    out[count] = q1;
    count += 1;

    if !meta.body_part_examined.is_empty() {
        let mut q2 = NQuin::default();
        q2.subject = series_hash;
        q2.predicate = pred_body_part;
        q2.object = q_hash(&meta.body_part_examined);
        q2.context = study_ctx;
        q2.parity = q2.subject ^ q2.predicate ^ q2.object ^ q2.context;
        out[count] = q2;
        count += 1;
    }

    let mut q3 = NQuin::default();
    q3.subject = series_hash;
    q3.predicate = pred_pixel_ptr;
    q3.object = encode_blob_pointer(blob_offset);
    q3.context = study_ctx;
    q3.metadata = pack_volume_metadata(meta.rows, meta.columns, blob_length);
    q3.parity = q3.subject ^ q3.predicate ^ q3.object ^ q3.context;
    out[count] = q3;
    count += 1;

    let _ = decode_blob_pointer(q3.object);
    count
}

fn publish_series_record(record: DicomSeriesRecord) -> Result<usize, DicomIngestError> {
    let idx = SERIES_COUNT.load(Ordering::Acquire);
    if idx >= MAX_SERIES_RECORDS {
        return Err(DicomIngestError::RegistryFull);
    }
    // SAFETY: single Core-3 writer; readers use Acquire on SERIES_COUNT before indexing.
    unsafe {
        *SERIES_RECORDS.0[idx].get() = record;
    }
    SERIES_COUNT.store(idx + 1, Ordering::Release);
    Ok(idx)
}

fn run_split_ingest(job: IngestJob) {
    let slot_idx = ((job.job_id as usize) - 1) % MAX_SERIES_RECORDS;
    let slot = &JOB_SLOTS[slot_idx];

    let result = (|| -> Result<DicomSeriesRecord, DicomIngestError> {
        std::fs::create_dir_all(imaging_dir(&job.storage_root))
            .map_err(|e| DicomIngestError::Io(e.to_string()))?;

        let mut file_bytes = Vec::new();
        File::open(&job.source_path)
            .and_then(|mut f| f.read_to_end(&mut file_bytes))
            .map_err(|e| DicomIngestError::Io(e.to_string()))?;

        let DicomSplitPayload { meta, pixels } =
            split_dicom_payload(&file_bytes).map_err(|e| DicomIngestError::Parse(e.to_string()))?;

        let pixel_bytes = &file_bytes[pixels.offset..pixels.offset + pixels.length];
        let mut store = DicomBlobStore::open(&blob_path(&job.storage_root))
            .map_err(|e| DicomIngestError::Io(e.to_string()))?;
        let blob_offset = store
            .append_pixels(pixel_bytes)
            .map_err(|e| DicomIngestError::Io(e.to_string()))?;

        let series_hash = hash_series_uid(&meta.series_instance_uid);
        let patient = if job.patient_did_hash != 0 {
            job.patient_did_hash
        } else if !meta.patient_id.is_empty() {
            q_hash(&meta.patient_id)
        } else {
            q_hash("q42:anonymousPatient")
        };

        let mut quins = [NQuin::default(); 6];
        let quin_count = compile_semantic_quins(
            &meta,
            patient,
            series_hash,
            blob_offset,
            pixels.length as u32,
            &mut quins,
        );

        let wal_file = wal_path(&job.storage_root);
        if let Ok(mut wal) = WriteAheadLog::open(&wal_file) {
            for quin in &quins[..quin_count] {
                let _ = wal.append_mutation(quin);
            }
        }

        let pixel_ptr_parity = quins[quin_count.saturating_sub(1)].parity;
        Ok(DicomSeriesRecord {
            patient_did_hash: patient,
            series_hash,
            blob_offset,
            blob_length: pixels.length as u32,
            rows: meta.rows,
            cols: meta.columns,
            organ_hash: infer_organ_hash(&meta),
            pixel_pointer_quin_parity: pixel_ptr_parity,
        })
    })();

    match result {
        Ok(record) => {
            slot.series_hash
                .store(record.series_hash, Ordering::Release);
            slot.blob_offset
                .store(record.blob_offset, Ordering::Release);
            slot.blob_length
                .store(record.blob_length, Ordering::Release);
            let _ = publish_series_record(record);
            slot.status.store(JOB_COMPLETE, Ordering::Release);
        }
        Err(_) => {
            slot.status.store(JOB_FAILED, Ordering::Release);
        }
    }
}

/// Pin the Core 3 DICOM Swarm worker (idempotent).
pub fn init_core3_dicom_worker(storage_root: PathBuf) {
    if IO_TX.get().is_some() {
        return;
    }

    let (tx, rx): (Sender<IngestJob>, Receiver<IngestJob>) = bounded(32);
    let _ = IO_TX.set(tx);

    thread::Builder::new()
        .name("qualia-core3-dicom".into())
        .spawn(move || {
            while let Ok(job) = rx.recv() {
                run_split_ingest(job);
            }
        })
        .expect("spawn Core 3 DICOM worker");

    let _ = storage_root;
}

fn job_sender() -> Result<&'static Sender<IngestJob>, DicomIngestError> {
    IO_TX.get().ok_or(DicomIngestError::WorkerUnavailable)
}

/// Submit a `.dcm` path to Core 3; returns a lock-free job id immediately.
pub fn submit_dicom_ingest(
    storage_root: &Path,
    source_path: &Path,
    patient_did_hash: u64,
) -> Result<u64, DicomIngestError> {
    init_core3_dicom_worker(storage_root.to_path_buf());
    let job_id = NEXT_JOB_ID.fetch_add(1, Ordering::Relaxed);
    let slot_idx = ((job_id as usize) - 1) % MAX_SERIES_RECORDS;
    JOB_SLOTS[slot_idx]
        .status
        .store(JOB_PENDING, Ordering::Release);

    job_sender()?
        .send(IngestJob {
            job_id,
            source_path: source_path.to_path_buf(),
            patient_did_hash,
            storage_root: storage_root.to_path_buf(),
        })
        .map_err(|e| DicomIngestError::Io(e.to_string()))?;

    Ok(job_id)
}

/// Poll job completion without blocking Core 1.
pub fn dicom_ingest_status(job_id: u64) -> u8 {
    let slot_idx = ((job_id as usize) - 1) % MAX_SERIES_RECORDS;
    JOB_SLOTS[slot_idx].status.load(Ordering::Acquire)
}

/// Synchronous split-ingest for tests and CLI (runs on caller thread).
pub fn split_ingest_sync(
    storage_root: &Path,
    source_path: &Path,
    patient_did_hash: u64,
) -> Result<DicomSeriesRecord, DicomIngestError> {
    let mut file_bytes = Vec::new();
    File::open(source_path)
        .and_then(|mut f| f.read_to_end(&mut file_bytes))
        .map_err(|e| DicomIngestError::Io(e.to_string()))?;

    let DicomSplitPayload { meta, pixels } =
        split_dicom_payload(&file_bytes).map_err(|e| DicomIngestError::Parse(e.to_string()))?;

    std::fs::create_dir_all(imaging_dir(storage_root))
        .map_err(|e| DicomIngestError::Io(e.to_string()))?;

    let pixel_bytes = &file_bytes[pixels.offset..pixels.offset + pixels.length];
    let mut store = DicomBlobStore::open(&blob_path(storage_root))
        .map_err(|e| DicomIngestError::Io(e.to_string()))?;
    let blob_offset = store
        .append_pixels(pixel_bytes)
        .map_err(|e| DicomIngestError::Io(e.to_string()))?;

    let series_hash = hash_series_uid(&meta.series_instance_uid);
    let patient = if patient_did_hash != 0 {
        patient_did_hash
    } else if !meta.patient_id.is_empty() {
        q_hash(&meta.patient_id)
    } else {
        q_hash("q42:anonymousPatient")
    };

    let mut quins = [NQuin::default(); 6];
    let quin_count = compile_semantic_quins(
        &meta,
        patient,
        series_hash,
        blob_offset,
        pixels.length as u32,
        &mut quins,
    );

    let wal_file = wal_path(storage_root);
    if let Ok(mut wal) = WriteAheadLog::open(&wal_file) {
        for quin in &quins[..quin_count] {
            let _ = wal.append_mutation(quin);
        }
    }

    let record = DicomSeriesRecord {
        patient_did_hash: patient,
        series_hash,
        blob_offset,
        blob_length: pixels.length as u32,
        rows: meta.rows,
        cols: meta.columns,
        organ_hash: infer_organ_hash(&meta),
        pixel_pointer_quin_parity: quins[quin_count.saturating_sub(1)].parity,
    };
    publish_series_record(record)?;
    Ok(record)
}

#[cfg(test)]
pub fn reset_ingest_registry_for_tests() {
    SERIES_COUNT.store(0, Ordering::Release);
}

#[allow(dead_code)]
fn split_ingest_sync_via_worker(
    storage_root: &Path,
    source_path: &Path,
    patient_did_hash: u64,
) -> Result<DicomSeriesRecord, DicomIngestError> {
    init_core3_dicom_worker(storage_root.to_path_buf());
    let job_id = NEXT_JOB_ID.fetch_add(1, Ordering::Relaxed);
    run_split_ingest(IngestJob {
        job_id,
        source_path: source_path.to_path_buf(),
        patient_did_hash,
        storage_root: storage_root.to_path_buf(),
    });
    if dicom_ingest_status(job_id) != JOB_COMPLETE {
        return Err(DicomIngestError::Parse("split ingest worker failed".into()));
    }
    let idx = SERIES_COUNT.load(Ordering::Acquire);
    if idx == 0 {
        return Err(DicomIngestError::JobNotFound);
    }
    Ok(unsafe { *SERIES_RECORDS.0[idx - 1].get() })
}

pub fn series_records_snapshot(out: &mut [DicomSeriesRecord]) -> usize {
    let count = SERIES_COUNT.load(Ordering::Acquire).min(MAX_SERIES_RECORDS);
    for (i, slot) in out.iter_mut().enumerate().take(count) {
        // SAFETY: reader synchronizes via Acquire load on SERIES_COUNT.
        *slot = unsafe { *SERIES_RECORDS.0[i].get() };
    }
    count
}

pub fn find_series_record(patient_did_hash: u64, series_hash: u64) -> Option<DicomSeriesRecord> {
    let count = SERIES_COUNT.load(Ordering::Acquire);
    for i in 0..count.min(MAX_SERIES_RECORDS) {
        let record = unsafe { *SERIES_RECORDS.0[i].get() };
        if record.patient_did_hash == patient_did_hash && record.series_hash == series_hash {
            return Some(record);
        }
    }
    None
}

/// Read a pixel payload slice from the blob store (single copy at FRB boundary).
pub fn read_volume_bytes(
    storage_root: &Path,
    record: &DicomSeriesRecord,
) -> Result<Vec<u8>, DicomIngestError> {
    let reader = DicomBlobReader::open(&blob_path(storage_root))?;
    Ok(reader
        .slice(record.blob_offset as usize, record.blob_length as usize)?
        .to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dicom::{encode_blob_pointer, INLINE_TAG_BLOB_POINTER};
    use tempfile::TempDir;

    #[test]
    fn blob_pointer_tag_encoding() {
        let ptr = encode_blob_pointer(4096);
        assert_eq!(ptr & INLINE_TAG_BLOB_POINTER, INLINE_TAG_BLOB_POINTER);
        assert_eq!(crate::dicom::decode_blob_pointer(ptr), Some(4096));
    }

    /// End-to-end split ingest against gitignored private DICOM (skips if folder absent).
    #[test]
    fn local_private_dicom_split_ingest() {
        let Some(root) = crate::dicom::resolve_local_dicom_dir() else {
            eprintln!("skip: private DICOM fixtures not present");
            return;
        };
        let paths = crate::dicom::collect_dicom_image_paths_under(&root, 2);
        assert!(
            !paths.is_empty(),
            "no image slices under {}",
            root.display()
        );

        reset_ingest_registry_for_tests();
        let tmp = TempDir::new().unwrap();
        let storage = tmp.path().to_path_buf();

        for path in &paths {
            let record =
                split_ingest_sync(&storage, path, q_hash("did:patient:local-dicom-test")).unwrap();
            assert!(record.blob_length > 0);
            assert!(record.rows > 0);
            assert!(record.cols > 0);
            let blob = read_volume_bytes(&storage, &record).unwrap();
            assert_eq!(blob.len(), record.blob_length as usize);
        }
    }

    #[test]
    fn split_ingest_writes_blob_and_registry() {
        reset_ingest_registry_for_tests();
        let tmp = TempDir::new().unwrap();
        let storage = tmp.path().to_path_buf();
        let bytes = crate::dicom::test_fixture_split_bytes();
        let dcm_path = storage.join("slice.dcm");
        std::fs::write(&dcm_path, &bytes).unwrap();

        let split = crate::dicom::split_dicom_payload(&bytes).expect("fixture must parse");
        assert_eq!(split.pixels.length, 4);

        let record = split_ingest_sync(&storage, &dcm_path, q_hash("did:patient:test")).unwrap();
        assert!(record.blob_length > 0);
        assert_eq!(record.rows, 2);
        assert_eq!(record.cols, 2);

        let blob = read_volume_bytes(&storage, &record).unwrap();
        assert_eq!(blob.len(), 4);
        assert_eq!(blob[0], 10);
    }
}
