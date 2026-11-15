//! Public canonical consumer coverage. Fixtures are inert, unreviewed data.
use super::super::{create_artifact_if_absent, mutate_artifact_stamped, StudioArtifact};
use super::*;
use crate::bundle::{BundleReader, BundleWriter};

fn resource() -> ResourceData {
    ResourceData {
        key: "future/input".into(),
        kind: "application/vnd.unknown+v9".into(),
        bytes: vec![0, 255, 42],
        meta: Some(vec![0xa1, 0x61, b'v', 9]),
    }
}
fn package(id: &str, meta: Option<Vec<u8>>) -> (Vec<u8>, super::super::StoreStamp) {
    let (_, stamp) =
        create_artifact_if_absent(StudioArtifact::new(id, "Geometry", "carrier fixture")).unwrap();
    let checkpoint = capture(id, &stamp, Default::default())
        .unwrap()
        .apply_resource_edits(vec![ResourceEdit::Add(resource())], Default::default())
        .unwrap();
    let bytes = encode(&checkpoint, vec![], Default::default()).unwrap();
    let reader = BundleReader::parse(&bytes).unwrap();
    let mut writer = BundleWriter::new();
    for entry in reader.entries() {
        let (payload, metadata) = if entry.key == SOURCE_RESOURCE_KEY {
            // Noncanonical JSON whitespace proves the origin is never rebuilt.
            let mut payload = b" \n".to_vec();
            payload.extend_from_slice(reader.get(&entry.key).unwrap());
            payload.push(b'\n');
            (payload, meta.clone())
        } else {
            (reader.get(&entry.key).unwrap().to_vec(), entry.meta.clone())
        };
        writer
            .add_file(&entry.key, &entry.kind, payload, metadata)
            .unwrap();
    }
    (writer.build().unwrap(), stamp)
}
fn fresh(input: &VerifiedSourceCarrier, stamp: &super::super::StoreStamp) -> Checkpoint {
    capture(&input.checkpoint().artifact().id, stamp, Default::default()).unwrap()
}
fn encoded(checkpoint: &Checkpoint) -> Vec<u8> {
    encode(checkpoint, vec![], Default::default()).unwrap()
}

#[test]
fn old_reader_compatible_but_unqualified_metadata_writer_and_legacy_copy_refuse() {
    let (bytes, stamp) = package("urn:carrier:writer-compatibility", Some(vec![0xa0]));
    let old = decode(&bytes, Default::default()).unwrap();
    assert_eq!(old.resources(), &[resource()]);
    assert!(encode(&old, vec![], Default::default()).is_err());
    assert!(encode(
        &old.clone().with_parent_digest([3; 32]),
        vec![],
        Default::default()
    )
    .is_err());
    let carrier = decode_source_carrier(&bytes, Default::default()).unwrap();
    assert!(fresh(&carrier, &stamp)
        .with_resources_from(&old, Default::default())
        .is_err());
    assert!(old
        .clone()
        .with_resources_from(&fresh(&carrier, &stamp), Default::default())
        .is_err());
    assert!(old
        .with_source_carrier(&carrier, Default::default())
        .is_err());
    assert_eq!(carrier.origin_digest(), super::codec::digest(&bytes));
}

#[test]
fn changed_current_source_journal_and_parent_stay_current_while_exact_origin_is_archived() {
    let id = "urn:carrier:current-source";
    let meta = Some(vec![0xa1, 0x61, b'x', 9]);
    let (original, stamp) = package(id, meta.clone());
    let input = decode_source_carrier(&original, Default::default()).unwrap();
    let (_, stamp) = mutate_artifact_stamped(id, &stamp, |a| {
        a.nodes
            .entry("poet.native.submission/00000000000000000001".into())
            .or_default()
            .insert("text".into(), "let accepted_current = 42;".into());
        Ok(())
    })
    .unwrap();
    let current = fresh(&input, &stamp).with_parent_digest(input.origin_digest());
    let expected_artifact = current.artifact().clone();
    let expected = encoded(&current);
    let merged = current
        .with_source_carrier(&input, Default::default())
        .unwrap();
    assert_eq!(merged.artifact(), &expected_artifact);
    assert_ne!(merged.artifact(), input.checkpoint().artifact());
    assert_eq!(merged.parent_digest(), Some(input.origin_digest()));
    assert_eq!(merged.file_digest(), None);
    let bytes = encoded(&merged);
    let reader = BundleReader::parse(&bytes).unwrap();
    assert_eq!(
        reader.get(SOURCE_RESOURCE_KEY),
        BundleReader::parse(&expected)
            .unwrap()
            .get(SOURCE_RESOURCE_KEY)
    );
    assert_eq!(reader.entry(SOURCE_RESOURCE_KEY).unwrap().meta, None);
    let archive_key = input.origin_key();
    let archive = reader.entry(&archive_key).unwrap();
    assert_eq!(archive.kind, "hmc");
    assert_eq!(archive.meta, None);
    assert_eq!(reader.get(&archive_key).unwrap(), original.as_slice());
    let old = BundleReader::parse(reader.get(&archive_key).unwrap()).unwrap();
    let original_reader = BundleReader::parse(&original).unwrap();
    assert_eq!(
        old.get(SOURCE_RESOURCE_KEY),
        original_reader.get(SOURCE_RESOURCE_KEY)
    );
    assert_eq!(old.entry(SOURCE_RESOURCE_KEY).unwrap().meta, meta);
    assert_eq!(reader.get("future/input"), old.get("future/input"));
    assert_eq!(reader.entry("future/input").unwrap().meta, resource().meta);
    assert!(super::super::artifact_stamp_is_current(id, &stamp));
    assert_eq!(
        decode(&bytes, Default::default()).unwrap().artifact(),
        &expected_artifact
    );
}

#[test]
fn source_without_metadata_needs_no_archive_and_exact_union_is_idempotent() {
    let (bytes, stamp) = package("urn:carrier:metadata-absent", None);
    let input = decode_source_carrier(&bytes, Default::default()).unwrap();
    let merged = fresh(&input, &stamp)
        .with_source_carrier(&input, Default::default())
        .unwrap();
    assert_eq!(merged.resources(), &[resource()]);
    let repeat = merged
        .with_source_carrier(&input, Default::default())
        .unwrap();
    assert_eq!(repeat.resources(), &[resource()]);
    assert_eq!(
        BundleReader::parse(&encoded(&repeat))
            .unwrap()
            .entry(SOURCE_RESOURCE_KEY)
            .unwrap()
            .meta,
        None
    );
}

#[test]
fn resource_and_archive_payload_kind_metadata_conflicts_refuse_without_mutation() {
    let (bytes, stamp) = package("urn:carrier:conflicts", Some(vec![0xa0]));
    let input = decode_source_carrier(&bytes, Default::default()).unwrap();
    for field in ["payload", "kind", "metadata", "metadata-absence"] {
        let mut conflicting = resource();
        match field {
            "payload" => conflicting.bytes.push(1),
            "kind" => conflicting.kind.push_str("-other"),
            "metadata" => conflicting.meta = Some(vec![0xa0]),
            _ => conflicting.meta = None,
        }
        let current = fresh(&input, &stamp)
            .apply_resource_edits(vec![ResourceEdit::Add(conflicting)], Default::default())
            .unwrap();
        let before = encoded(&current);
        assert!(current
            .clone()
            .with_source_carrier(&input, Default::default())
            .is_err());
        assert_eq!(encoded(&current), before);
    }
    let current = fresh(&input, &stamp)
        .with_source_carrier(&input, Default::default())
        .unwrap();
    let archive = current
        .resources()
        .iter()
        .find(|r| r.key == input.origin_key())
        .unwrap();
    for edit in [
        ResourceEdit::Add(archive.clone()),
        ResourceEdit::Remove {
            key: archive.key.clone(),
            expected: archive.summary().digest,
        },
        ResourceEdit::ReplaceBytes {
            key: archive.key.clone(),
            expected: archive.summary().digest,
            bytes: vec![1],
        },
    ] {
        assert!(current
            .apply_resource_edits(vec![edit], Default::default())
            .is_err());
    }
    let merged = current
        .with_source_carrier(&input, Default::default())
        .unwrap();
    assert_eq!(
        merged
            .resources()
            .iter()
            .filter(|r| r.key == input.origin_key())
            .count(),
        1
    );
}

#[test]
fn archive_namespace_key_kind_metadata_and_exact_digest_are_checked_by_reader_and_writer() {
    let (bytes, stamp) = package("urn:carrier:archive-integrity", Some(vec![0xa0]));
    let input = decode_source_carrier(&bytes, Default::default()).unwrap();
    let good = fresh(&input, &stamp)
        .with_source_carrier(&input, Default::default())
        .unwrap();
    for field in ["payload", "kind", "metadata", "uppercase", "malformed"] {
        let mut invalid = good.clone();
        let archive = invalid
            .resources
            .iter_mut()
            .find(|r| r.key == input.origin_key())
            .unwrap();
        match field {
            "payload" => archive.bytes.push(1),
            "kind" => archive.kind = "other".into(),
            "metadata" => archive.meta = Some(vec![0xa0]),
            "uppercase" => archive.key = archive.key.to_ascii_uppercase(),
            _ => archive.key = "authoring/origins/sha256/short.hmc".into(),
        }
        // Retain the changed fixture without holding a mutable borrow of invalid.
        let archive = archive.clone();
        assert!(encode(&invalid, vec![], Default::default()).is_err());
        let original = encoded(&good);
        let reader = BundleReader::parse(&original).unwrap();
        let mut writer = BundleWriter::new();
        for entry in reader.entries() {
            if entry.key == input.origin_key() {
                writer
                    .add_file(
                        &archive.key,
                        &archive.kind,
                        archive.bytes.clone(),
                        archive.meta.clone(),
                    )
                    .unwrap();
            } else {
                writer
                    .add_file(
                        &entry.key,
                        &entry.kind,
                        reader.get(&entry.key).unwrap().to_vec(),
                        entry.meta.clone(),
                    )
                    .unwrap();
            }
        }
        assert!(decode_source_carrier(&writer.build().unwrap(), Default::default()).is_err());
    }
}

#[test]
fn distinct_verified_bundle_origins_preserve_each_original_metadata_without_equivalence() {
    let (one, stamp) = package("urn:carrier:distinct-origins", Some(vec![0xa0]));
    let reader = BundleReader::parse(&one).unwrap();
    let mut writer = BundleWriter::new();
    for entry in reader.entries() {
        writer
            .add_file(
                &entry.key,
                &entry.kind,
                reader.get(&entry.key).unwrap().to_vec(),
                if entry.key == SOURCE_RESOURCE_KEY {
                    Some(vec![0x80])
                } else {
                    entry.meta.clone()
                },
            )
            .unwrap();
    }
    let two = writer.build().unwrap();
    let first = decode_source_carrier(&one, Default::default()).unwrap();
    let second = decode_source_carrier(&two, Default::default()).unwrap();
    assert_ne!(first.origin_digest(), second.origin_digest());
    let merged = fresh(&first, &stamp)
        .with_source_carrier(&first, Default::default())
        .unwrap()
        .with_source_carrier(&second, Default::default())
        .unwrap();
    let bytes = encoded(&merged);
    let reader = BundleReader::parse(&bytes).unwrap();
    assert_eq!(reader.get(&first.origin_key()), Some(one.as_slice()));
    assert_eq!(reader.get(&second.origin_key()), Some(two.as_slice()));
    assert_eq!(reader.entry(SOURCE_RESOURCE_KEY).unwrap().meta, None);
}

#[test]
fn foreign_subject_domain_and_archive_union_budgets_fail_closed() {
    let (bytes, stamp) = package("urn:carrier:bounds", Some(vec![0xa0]));
    let input = decode_source_carrier(&bytes, Default::default()).unwrap();
    let (foreign, _) = package("urn:carrier:foreign", None);
    let foreign = decode_source_carrier(&foreign, Default::default()).unwrap();
    assert!(fresh(&input, &stamp)
        .with_source_carrier(&foreign, Default::default())
        .is_err());
    let reader = BundleReader::parse(&bytes).unwrap();
    let mut record: serde_json::Value =
        serde_json::from_slice(reader.get(SOURCE_RESOURCE_KEY).unwrap()).unwrap();
    record["artifact"]["domain"] = "Material".into();
    let mut writer = BundleWriter::new();
    let entry = reader.entry(SOURCE_RESOURCE_KEY).unwrap();
    writer
        .add_file(
            SOURCE_RESOURCE_KEY,
            &entry.kind,
            serde_json::to_vec(&record).unwrap(),
            entry.meta.clone(),
        )
        .unwrap();
    let domain = decode_source_carrier(&writer.build().unwrap(), Default::default()).unwrap();
    assert!(fresh(&input, &stamp)
        .with_source_carrier(&domain, Default::default())
        .is_err());
    for limits in [
        CheckpointLimits {
            entries: 2,
            ..Default::default()
        },
        CheckpointLimits {
            source_bytes: 1,
            ..Default::default()
        },
        CheckpointLimits {
            file_bytes: bytes.len(),
            ..Default::default()
        },
    ] {
        assert!(fresh(&input, &stamp)
            .with_source_carrier(&input, limits)
            .is_err());
    }
    let key = input.origin_key();
    let current = fresh(&input, &stamp);
    let serialized = encoded(&current);
    let source = BundleReader::parse(&serialized).unwrap();
    let mut charge = 128;
    super::codec::charge(
        &mut charge,
        SOURCE_RESOURCE_KEY,
        &entry.kind,
        source.get(SOURCE_RESOURCE_KEY).unwrap().len(),
        0,
        usize::MAX,
    )
    .unwrap();
    let r = resource();
    super::codec::charge(
        &mut charge,
        &r.key,
        &r.kind,
        r.bytes.len(),
        r.meta.as_ref().map_or(0, Vec::len),
        usize::MAX,
    )
    .unwrap();
    super::codec::charge(&mut charge, &key, "hmc", bytes.len(), 0, usize::MAX).unwrap();
    assert!(current
        .clone()
        .with_source_carrier(
            &input,
            CheckpointLimits {
                file_bytes: charge - 1,
                ..Default::default()
            }
        )
        .is_err());
    assert!(current
        .with_source_carrier(
            &input,
            CheckpointLimits {
                file_bytes: charge,
                ..Default::default()
            }
        )
        .is_ok());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn native_fresh_save_reopen_save_as_preserve_origin_without_repeated_wrapping() {
    let (bytes, stamp) = package("urn:carrier:native-reopen", Some(vec![0xa0]));
    let input = decode_source_carrier(&bytes, Default::default()).unwrap();
    let (_, stamp) = mutate_artifact_stamped(&input.checkpoint().artifact().id, &stamp, |a| {
        a.properties
            .insert("accepted.current".into(), "new-source".into());
        Ok(())
    })
    .unwrap();
    let artifact = fresh(&input, &stamp).artifact().clone();
    let merged = fresh(&input, &stamp)
        .with_source_carrier(&input, Default::default())
        .unwrap();
    let output = encoded(&merged);
    let dir = tempfile::TempDir::new().unwrap();
    let first_path = dir.path().join("fresh.hmc");
    let receipt = save(
        &first_path,
        &output,
        ExpectedFile::Absent,
        Default::default(),
    )
    .unwrap();
    drop(merged);
    let reopened = load_source_carrier(&first_path, Default::default()).unwrap();
    assert_eq!(reopened.checkpoint().artifact(), &artifact);
    let copied = fresh(&reopened, &stamp)
        .with_parent_digest(receipt.digest)
        .with_source_carrier(&reopened, Default::default())
        .unwrap();
    assert_eq!(
        copied.resources(),
        reopened.resources(),
        "current metadata absent: no new recursive archive"
    );
    let copied_bytes = encoded(&copied);
    let copy_path = dir.path().join("copy.hmc");
    save(
        &copy_path,
        &copied_bytes,
        ExpectedFile::Absent,
        Default::default(),
    )
    .unwrap();
    let copy = load_source_carrier(&copy_path, Default::default()).unwrap();
    assert_eq!(copy.checkpoint().parent_digest(), Some(receipt.digest));
    assert_eq!(copy.checkpoint().artifact(), &artifact);
    assert_eq!(copy.resources(), copied.resources());
    let reader = BundleReader::parse(&copied_bytes).unwrap();
    assert_eq!(reader.get(&input.origin_key()), Some(bytes.as_slice()));
    assert_eq!(reader.entry(SOURCE_RESOURCE_KEY).unwrap().meta, None);
    assert!(save(
        &first_path,
        &copied_bytes,
        ExpectedFile::Sha256([0; 32]),
        Default::default()
    )
    .is_err());
    for key in [SOURCE_RESOURCE_KEY, "future/input"] {
        let mut corrupt = output.clone();
        let offset = BundleReader::parse(&output)
            .unwrap()
            .entry(key)
            .unwrap()
            .offset as usize;
        corrupt[offset] ^= 1;
        assert!(decode_source_carrier(&corrupt, Default::default()).is_err());
    }
    assert_eq!(std::fs::read(&first_path).unwrap(), output);
    assert!(super::super::artifact_stamp_is_current(
        &artifact.id,
        &stamp
    ));
    let cleanup_path = dir.path().to_path_buf();
    drop(dir);
    assert!(!cleanup_path.exists());
}
