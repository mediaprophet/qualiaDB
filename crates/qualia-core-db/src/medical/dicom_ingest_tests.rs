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

    let _guard = INGEST_TEST_LOCK.lock().unwrap();
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
