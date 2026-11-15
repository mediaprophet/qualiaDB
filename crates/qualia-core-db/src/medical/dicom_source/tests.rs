use super::storage::{preserve, publish_exact, read_source};
use super::*;
use crate::dicom_ingest::INGEST_TEST_LOCK;
use std::{fs, path::Path};
use tempfile::TempDir;
fn budget(n: usize) -> DicomSourceBudget {
    DicomSourceBudget {
        max_source_bytes: n,
    }
}
fn fixture() -> Vec<u8> {
    let mut bytes = crate::dicom::test_fixture_split_bytes();
    bytes[..128].fill(0xa5);
    let at = crate::dicom::split_dicom_payload(&bytes)
        .unwrap()
        .pixels
        .offset
        - 12;
    // Unknown private metadata, never represented by DicomMetadata.
    let private = [
        0x11, 0x00, 0x10, 0x10, b'L', b'O', 4, 0, b'K', b'E', b'E', b'P',
    ];
    bytes.splice(at..at, private);
    bytes.extend_from_slice(&[0xde, 0xad, 0xbe, 0xef]);
    bytes
}
fn names(path: &Path) -> Vec<String> {
    let mut names: Vec<_> = fs::read_dir(path)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

#[test]
fn ingestion_preserves_full_original_metadata_pixels_and_final_byte_identity() {
    let _guard = INGEST_TEST_LOCK.lock().unwrap();
    crate::dicom_ingest::reset_ingest_registry_for_tests();
    let tmp = TempDir::new().unwrap();
    let storage = tmp.path().join("store");
    let path = tmp.path().join("synthetic.dcm");
    let bytes = fixture();
    fs::write(&path, &bytes).unwrap();
    let result = split_ingest_sync_with_source(&storage, &path, 42, budget(bytes.len())).unwrap();
    assert_eq!(fs::read(&path).unwrap(), bytes);
    assert_eq!(fs::read(&result.source.path).unwrap(), bytes);
    assert_eq!(result.source.blake3, *blake3::hash(&bytes).as_bytes());
    assert_eq!(result.source.byte_length, bytes.len() as u64);
    assert_eq!(
        crate::dicom_ingest::read_volume_bytes(&storage, &result.record).unwrap(),
        [10, 20, 30, 40]
    );
    assert_eq!(
        source_for_record(&storage, &result.record, budget(bytes.len())).unwrap(),
        result.source
    );
    assert!(matches!(
        source_for_record(&storage, &result.record, budget(bytes.len() - 1)),
        Err(DicomSourceError::BudgetExceeded)
    ));
    assert!(result
        .source
        .path
        .starts_with(storage.canonicalize().unwrap()));
    assert_eq!(names(result.source.path.parent().unwrap()).len(), 1);
    let mut altered = bytes.clone();
    *altered.last_mut().unwrap() ^= 1;
    assert_ne!(
        blake3::hash(&altered),
        blake3::Hash::from(result.source.blake3)
    );
    assert_ne!(
        preserve(&storage, &altered, budget(altered.len()))
            .unwrap()
            .path,
        result.source.path
    );
}

#[test]
fn existing_sync_and_worker_paths_keep_original_associations() {
    let _guard = INGEST_TEST_LOCK.lock().unwrap();
    crate::dicom_ingest::reset_ingest_registry_for_tests();
    let tmp = TempDir::new().unwrap();
    let bytes = fixture();
    let path = tmp.path().join("synthetic.dcm");
    fs::write(&path, &bytes).unwrap();
    let record = crate::dicom_ingest::split_ingest_sync(tmp.path(), &path, 43).unwrap();
    assert_eq!(
        source_for_record(tmp.path(), &record, budget(bytes.len()))
            .unwrap()
            .blake3,
        *blake3::hash(&bytes).as_bytes()
    );
    // Exercise actual queued worker and its existing completion/status surface.
    let job = crate::dicom_ingest::submit_dicom_ingest(tmp.path(), &path, 44).unwrap();
    let end = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while crate::dicom_ingest::dicom_ingest_status(job) == crate::dicom_ingest::JOB_PENDING {
        assert!(std::time::Instant::now() < end, "worker deadline");
        std::thread::yield_now();
    }
    assert_eq!(
        crate::dicom_ingest::dicom_ingest_status(job),
        crate::dicom_ingest::JOB_COMPLETE
    );
    let mut records = [crate::dicom_ingest::DicomSeriesRecord::default(); 2];
    assert_eq!(
        crate::dicom_ingest::series_records_snapshot(&mut records),
        2
    );
    let source = source_for_record(tmp.path(), &records[1], budget(bytes.len())).unwrap();
    assert_eq!(fs::read(source.path).unwrap(), bytes);
}

#[test]
fn identical_publication_verifies_existing_and_rejects_corrupt_or_truncated_objects() {
    let tmp = TempDir::new().unwrap();
    let bytes = fixture();
    let a = preserve(tmp.path(), &bytes, budget(bytes.len())).unwrap();
    assert_eq!(
        preserve(tmp.path(), &bytes, budget(bytes.len())).unwrap(),
        a
    );
    let mut corrupted = bytes.clone();
    *corrupted.last_mut().unwrap() ^= 1;
    fs::write(&a.path, &corrupted).unwrap();
    assert!(matches!(
        preserve(tmp.path(), &bytes, budget(bytes.len())),
        Err(DicomSourceError::Conflict)
    ));
    assert_eq!(fs::read(&a.path).unwrap(), corrupted);
    assert!(matches!(
        super::storage::verify(tmp.path(), a.blake3, a.byte_length, budget(bytes.len())),
        Err(DicomSourceError::Conflict)
    ));
    fs::write(&a.path, &bytes[..bytes.len() - 1]).unwrap();
    assert!(matches!(
        preserve(tmp.path(), &bytes, budget(bytes.len())),
        Err(DicomSourceError::Conflict)
    ));
    assert_eq!(names(a.path.parent().unwrap()).len(), 1);
}

#[test]
fn association_corruption_or_wrong_record_fails_closed() {
    let _guard = INGEST_TEST_LOCK.lock().unwrap();
    crate::dicom_ingest::reset_ingest_registry_for_tests();
    let tmp = TempDir::new().unwrap();
    let bytes = fixture();
    let path = tmp.path().join("synthetic.dcm");
    fs::write(&path, &bytes).unwrap();
    let result = split_ingest_sync_with_source(tmp.path(), &path, 42, budget(bytes.len())).unwrap();
    let mut wrong = result.record;
    wrong.patient_did_hash ^= 1;
    assert!(matches!(
        source_for_record(tmp.path(), &wrong, budget(bytes.len())),
        Err(DicomSourceError::InvalidAssociation)
    ));
    // An existing conflicting receipt is never overwritten.
    assert!(matches!(
        super::association::associate(tmp.path(), &wrong, &result.source),
        Err(DicomSourceError::Conflict)
    ));
    let dir = tmp.path().join("Imaging/dicom_source_refs");
    let receipt = fs::read_dir(&dir).unwrap().next().unwrap().unwrap().path();
    fs::write(&receipt, b"{").unwrap();
    assert!(matches!(
        source_for_record(tmp.path(), &result.record, budget(bytes.len())),
        Err(DicomSourceError::InvalidAssociation)
    ));
    assert!(matches!(
        super::association::associate(tmp.path(), &result.record, &result.source),
        Err(DicomSourceError::Conflict)
    ));
    assert_eq!(names(&dir).len(), 1);
    fs::write(&receipt, vec![0; 2049]).unwrap();
    assert!(matches!(
        source_for_record(tmp.path(), &result.record, budget(bytes.len())),
        Err(DicomSourceError::InvalidAssociation)
    ));
}

#[test]
fn byte_admission_and_invalid_dicom_fail_before_source_or_pixel_publication() {
    let _guard = INGEST_TEST_LOCK.lock().unwrap();
    crate::dicom_ingest::reset_ingest_registry_for_tests();
    let tmp = TempDir::new().unwrap();
    let bytes = fixture();
    let path = tmp.path().join("synthetic.dcm");
    fs::write(&path, &bytes).unwrap();
    let store = tmp.path().join("store");
    assert!(matches!(
        read_source(&path, budget(bytes.len() - 1)),
        Err(DicomSourceError::BudgetExceeded)
    ));
    assert!(split_ingest_sync_with_source(&store, &path, 42, budget(bytes.len() - 1)).is_err());
    assert!(!store.exists());
    assert!(matches!(
        preserve(&store, &bytes, budget(bytes.len() - 1)),
        Err(DicomSourceError::BudgetExceeded)
    ));
    assert!(!store.exists());
    fs::write(&path, b"not Part 10").unwrap();
    assert!(matches!(
        split_ingest_sync_with_source(&store, &path, 42, budget(128)),
        Err(crate::dicom_ingest::DicomIngestError::Parse(_))
    ));
    assert!(!store.exists());
}

#[test]
fn publication_failure_cleans_temporary_files_and_rejects_path_redirection() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("destination");
    fs::create_dir(&path).unwrap();
    assert!(publish_exact(&path, b"synthetic").is_err());
    assert_eq!(names(tmp.path()), ["destination"]);
    let bytes = fixture();
    let storage = tmp.path().join("blocked");
    fs::create_dir(&storage).unwrap();
    fs::write(storage.join("Imaging"), b"not a directory").unwrap();
    assert!(preserve(&storage, &bytes, budget(bytes.len())).is_err());
    assert_eq!(names(&storage), ["Imaging"]);
    let good = tmp.path().join("good");
    let mut source = preserve(&good, &bytes, budget(bytes.len())).unwrap();
    source.path = tmp.path().join("outside.dcm");
    let record = crate::dicom_ingest::DicomSeriesRecord::default();
    assert!(matches!(
        super::association::associate(&good, &record, &source),
        Err(DicomSourceError::InvalidPath)
    ));
    assert!(!source.path.exists());
}

#[cfg(unix)]
#[test]
fn symlinked_imaging_directory_cannot_escape_storage_root() {
    let tmp = TempDir::new().unwrap();
    let outside = TempDir::new().unwrap();
    let root = tmp.path().join("store");
    fs::create_dir(&root).unwrap();
    std::os::unix::fs::symlink(outside.path(), root.join("Imaging")).unwrap();
    assert!(matches!(
        preserve(&root, b"synthetic", budget(9)),
        Err(DicomSourceError::InvalidPath)
    ));
    assert!(names(outside.path()).is_empty());
}

#[test]
fn archive_only_preserves_unsupported_syntax_and_non_pixel_objects() {
    let tmp = TempDir::new().unwrap();
    let source = tmp.path().join("synthetic-non-pixel.dcm");
    let root = tmp.path().join("archive");
    let mut bytes = vec![0xa7; 128];
    bytes.extend_from_slice(b"DICM");
    let syntax = b"1.2.840.10008.1.2.4.999\0";
    bytes.extend_from_slice(&[2, 0, 0x10, 0, b'U', b'I']);
    bytes.extend_from_slice(&(syntax.len() as u16).to_le_bytes());
    bytes.extend_from_slice(syntax);
    // This source has no Pixel Data and is outside parser transfer-syntax support.
    assert!(crate::dicom::split_dicom_payload(&bytes).is_err());
    fs::write(&source, &bytes).unwrap();
    let archived = archive_part10_source(&root, &source, budget(bytes.len())).unwrap();
    assert_eq!(archived.blake3, *blake3::hash(&bytes).as_bytes());
    assert_eq!(fs::read(&archived.path).unwrap(), bytes);
    assert_eq!(fs::read(&source).unwrap(), bytes);
    assert!(!root.join("Imaging/dicom_pixels.blob").exists());
    assert!(matches!(
        archive_part10_source(&root, &source, budget(bytes.len() - 1)),
        Err(DicomSourceError::BudgetExceeded)
    ));
    fs::write(&source, vec![0; 132]).unwrap();
    let invalid_root = tmp.path().join("invalid");
    assert!(matches!(
        archive_part10_source(&invalid_root, &source, budget(132)),
        Err(DicomSourceError::NotPart10)
    ));
    assert!(!invalid_root.exists());
}
