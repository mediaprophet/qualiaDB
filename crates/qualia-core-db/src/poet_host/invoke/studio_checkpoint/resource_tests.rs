//! Canonical BundleReader/Writer coverage; resources are inert test fixtures,
//! not accepted masters, a complete species or a review/adoption claim.
use super::super::{create_artifact_if_absent, mutate_artifact_stamped, StudioArtifact};
use super::*;
use crate::bundle::{BundleReader, BundleWriter};

fn mixed(id: &str) -> (Vec<u8>, super::super::StoreStamp) {
    let (_, stamp) = create_artifact_if_absent(StudioArtifact::new(
        id,
        "Geometry",
        "Mixed resource fixture",
    ))
    .unwrap();
    let checkpoint = capture(id, &stamp, Default::default()).unwrap();
    let asset = crate::render::compile_10d::compile_asset(
        b"v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
        Some("obj"),
        "urn:fixture:mixed-geometry",
        "obj",
    )
    .unwrap();
    let bytes = encode(
        &checkpoint,
        vec![("derived/model.10d".into(), asset)],
        Default::default(),
    )
    .unwrap();
    let reader = BundleReader::parse(&bytes).unwrap();
    let mut writer = BundleWriter::new();
    for entry in reader.entries() {
        let meta = if entry.key == "authoring/native-source.json" {
            Some(vec![0xa1, 0x61, b'x', 1])
        } else {
            entry.meta.clone()
        };
        writer
            .add_file(
                &entry.key,
                &entry.kind,
                reader.get(&entry.key).unwrap().to_vec(),
                meta,
            )
            .unwrap();
    }
    for (key, kind, data) in [
        (
            "masters/source.obj",
            "text/plain",
            b"# editable master fixture\nv 1 2 3\n".as_slice(),
        ),
        (
            "rigs/source.json",
            "application/json",
            b"{\"joints\":[],\"fixture\":true}".as_slice(),
        ),
        (
            "textures/raw.bin",
            "application/vnd.future.texture",
            &[0, 1, 255, 42],
        ),
        (
            "private/future.resource",
            "future/unknown+v9",
            b"\0exact unknown resource\xff".as_slice(),
        ),
        (
            "inert/untrusted.vibe",
            "text/vibescript",
            b"grant(\"filesystem\"); effect(\"must-never-run\");".as_slice(),
        ),
    ] {
        writer
            .add_file(key, kind, data.to_vec(), Some(vec![0xa1, 0x61, b'v', 9]))
            .unwrap();
    }
    (writer.build().unwrap(), stamp)
}

#[test]
fn mixed_package_source_material_edit_preserves_raw_bytes_graph_lexicon_and_metadata() {
    let id = "urn:checkpoint:resource-byte-exact";
    let (original, stamp) = mixed(id);
    let carrier = decode_source_carrier(&original, Default::default()).unwrap();
    let old = carrier.checkpoint();
    let (_, stamp) = mutate_artifact_stamped(id, &stamp, |artifact| {
        artifact
            .nodes
            .entry("poet.native.submission/00000000000000000001".into())
            .or_default()
            .insert("text".into(), "let material = {roughness: 0.61};\n".into());
        Ok(())
    })
    .unwrap();
    let fresh = capture(id, &stamp, Default::default())
        .unwrap()
        .with_source_carrier(&carrier, Default::default())
        .unwrap()
        .with_parent_digest(old.file_digest().unwrap());
    let edited = encode(&fresh, vec![], Default::default()).unwrap();
    let next = decode(&edited, Default::default()).unwrap();
    for resource in old.resources() {
        assert!(next.resources().contains(resource));
    }
    assert_eq!(next.resources().len(), old.resources().len() + 1);
    assert_ne!(old.artifact(), next.artifact());
    assert_eq!(next.parent_digest(), old.file_digest());
    let before = BundleReader::parse(&original).unwrap();
    let after = BundleReader::parse(&edited).unwrap();
    assert_eq!(after.entry(SOURCE_RESOURCE_KEY).unwrap().meta, None);
    let archived = after.get(&carrier.origin_key()).unwrap();
    assert_eq!(archived, original.as_slice());
    let origin = BundleReader::parse(archived).unwrap();
    assert_eq!(
        before.entry(SOURCE_RESOURCE_KEY).unwrap().meta,
        origin.entry(SOURCE_RESOURCE_KEY).unwrap().meta
    );
    for entry in before
        .entries()
        .iter()
        .filter(|entry| entry.key != "authoring/native-source.json")
    {
        let retained = after.entry(&entry.key).unwrap();
        assert_eq!(entry.sha256, retained.sha256);
        assert_eq!(entry.kind, retained.kind);
        assert_eq!(entry.meta, retained.meta);
        assert_eq!(before.get(&entry.key), after.get(&entry.key));
    }
    let graph = after.get("graphs/derived/model.10d.q42").unwrap();
    let native = crate::q42_volume::WasmSliceQ42Reader::open(graph).unwrap();
    assert!(native.is_unified_volume());
    assert!(!native.collect_all_quins().unwrap().is_empty());
}

#[test]
fn explicit_add_replace_remove_are_digest_guarded_transactional_and_keep_metadata() {
    let id = "urn:checkpoint:explicit-resources";
    let (bytes, stamp) = mixed(id);
    let carrier = decode_source_carrier(&bytes, Default::default()).unwrap();
    let base = capture(id, &stamp, Default::default())
        .unwrap()
        .with_source_carrier(&carrier, Default::default())
        .unwrap();
    let old = base
        .resources()
        .iter()
        .find(|entry| entry.key == "masters/source.obj")
        .unwrap();
    let remove = base
        .inventory()
        .into_iter()
        .find(|entry| entry.key == "rigs/source.json")
        .unwrap();
    let edits = vec![
        ResourceEdit::ReplaceBytes {
            key: old.key.clone(),
            expected: old.summary().digest,
            bytes: b"# explicitly revised source\n".to_vec(),
        },
        ResourceEdit::Remove {
            key: remove.key.clone(),
            expected: remove.digest,
        },
        ResourceEdit::Add(ResourceData {
            key: "evidence/change.txt".into(),
            kind: "text/plain".into(),
            bytes: b"unreviewed explicit edit".to_vec(),
            meta: None,
        }),
    ];
    let edited = base
        .apply_resource_edits(edits, Default::default())
        .unwrap();
    let serialized = encode(&edited, vec![], Default::default()).unwrap();
    let reopened = decode(&serialized, Default::default()).unwrap();
    let replaced = reopened
        .resources()
        .iter()
        .find(|entry| entry.key == old.key)
        .unwrap();
    assert_eq!(replaced.kind, old.kind);
    assert_eq!(replaced.meta, old.meta);
    assert_ne!(replaced.bytes, old.bytes);
    assert!(!reopened
        .resources()
        .iter()
        .any(|entry| entry.key == remove.key));
    assert!(reopened
        .resources()
        .iter()
        .any(|entry| entry.key == "evidence/change.txt"));
    for entry in base
        .resources()
        .iter()
        .filter(|entry| entry.key != old.key && entry.key != remove.key)
    {
        assert!(reopened.resources().contains(entry));
    }
    for edits in [
        vec![ResourceEdit::Remove {
            key: old.key.clone(),
            expected: [0; 32],
        }],
        vec![ResourceEdit::Add(old.clone())],
        vec![ResourceEdit::Remove {
            key: "authoring/native-source.json".into(),
            expected: [0; 32],
        }],
        vec![ResourceEdit::Add(ResourceData {
            key: "bad-graph.q42".into(),
            kind: "q42".into(),
            bytes: b"not a native Q42 graph".to_vec(),
            meta: None,
        })],
    ] {
        assert!(base
            .apply_resource_edits(edits, Default::default())
            .is_err());
        assert_eq!(
            base.resources()
                .iter()
                .find(|entry| entry.key == old.key)
                .unwrap(),
            old
        );
    }
}

#[test]
fn combined_resource_budget_stale_package_and_corruption_preserve_disk() {
    let (bytes, _) = mixed("urn:checkpoint:resource-failure");
    let base = decode(&bytes, Default::default()).unwrap();
    let limits = CheckpointLimits {
        file_bytes: bytes.len() + 4096,
        ..Default::default()
    };
    let additions = ["a", "b"]
        .map(|key| {
            ResourceEdit::Add(ResourceData {
                key: key.into(),
                kind: "opaque".into(),
                bytes: vec![0; limits.file_bytes / 2],
                meta: None,
            })
        })
        .to_vec();
    assert!(base.apply_resource_edits(additions, limits).is_err());
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("mixed.hmc");
    let saved = save(&path, &bytes, ExpectedFile::Absent, Default::default()).unwrap();
    assert!(save(
        &path,
        &bytes,
        ExpectedFile::Sha256([0; 32]),
        Default::default()
    )
    .is_err());
    let mut corrupt = bytes.clone();
    let reader = BundleReader::parse(&bytes).unwrap();
    corrupt[reader.entry("private/future.resource").unwrap().offset as usize] ^= 1;
    assert!(decode(&corrupt, Default::default()).is_err());
    assert!(save(
        &path,
        &corrupt,
        ExpectedFile::Sha256(saved.digest),
        Default::default()
    )
    .is_err());
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    assert!(decode(
        &bytes,
        CheckpointLimits {
            entries: 2,
            ..Default::default()
        }
    )
    .is_err());
}

#[test]
fn source_only_legacy_checkpoint_remains_compatible_and_resources_never_execute() {
    let id = "urn:checkpoint:resources-inert";
    let (bytes, _) = mixed(id);
    let base = decode(&bytes, Default::default()).unwrap();
    let before = base.artifact().clone();
    super::super::store().lock().unwrap().remove(id);
    let (active, stamp) = activate(base, Default::default()).unwrap();
    assert_eq!(active, before);
    // The new active store contains only the source record, not parsed grants
    // or resource-triggered effects; runtime activation remains separately held.
    assert!(!active.properties.contains_key("filesystem"));
    let source_only = capture(id, &stamp, Default::default()).unwrap();
    let encoded = encode(&source_only, vec![], Default::default()).unwrap();
    assert_eq!(BundleReader::parse(&encoded).unwrap().entries().len(), 1);
    assert!(decode(&encoded, Default::default())
        .unwrap()
        .resources()
        .is_empty());
}

#[test]
fn native_texture_and_compiled_graph_dependency_invalidations_are_refused() {
    use crate::container_10d::{
        material_container::attach_material_section, MaterialRecord, SubmeshRange,
    };
    let (bytes, _) = mixed("urn:checkpoint:native-resource-dependencies");
    let base = decode(&bytes, Default::default()).unwrap();
    let asset = crate::render::compile_10d::compile_asset(
        b"v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
        Some("obj"),
        "urn:fixture:textured-resource",
        "obj",
    )
    .unwrap();
    let plain = crate::render::compile_10d::compile_mesh_to_10d(&asset.mesh).unwrap();
    let payload = b"opaque original image codec bytes".to_vec();
    let digest = super::codec::digest(&payload);
    let key = crate::render::asset_package::texture_hmc_key(&digest);
    let material = MaterialRecord {
        base_color_texture: digest,
        ..MaterialRecord::legacy_default()
    };
    let textured = attach_material_section(
        &plain,
        &[material],
        &[SubmeshRange {
            first_index: 0,
            index_count: 3,
            material_id: material.id,
            semantic_id: 0,
        }],
        3,
    )
    .unwrap();
    let edited = base
        .apply_resource_edits(
            vec![
                ResourceEdit::Add(ResourceData {
                    key: "textured/model.10d".into(),
                    kind: "10d".into(),
                    bytes: textured,
                    meta: None,
                }),
                ResourceEdit::Add(ResourceData {
                    key: key.clone(),
                    kind: "image/opaque-fixture".into(),
                    bytes: payload,
                    meta: None,
                }),
            ],
            Default::default(),
        )
        .unwrap();
    for edit in [
        ResourceEdit::Remove {
            key: key.clone(),
            expected: digest,
        },
        ResourceEdit::ReplaceBytes {
            key: key.clone(),
            expected: digest,
            bytes: b"wrong content under old digest key".to_vec(),
        },
    ] {
        assert!(edited
            .apply_resource_edits(vec![edit], Default::default())
            .is_err());
    }
    let graph = base
        .inventory()
        .into_iter()
        .find(|entry| entry.kind == "q42")
        .unwrap();
    assert!(base
        .apply_resource_edits(
            vec![ResourceEdit::Remove {
                key: graph.key.clone(),
                expected: graph.digest
            }],
            Default::default()
        )
        .is_err());
    let geometry = base
        .inventory()
        .into_iter()
        .find(|entry| entry.kind == "10d")
        .unwrap();
    assert!(base
        .apply_resource_edits(
            vec![
                ResourceEdit::Remove {
                    key: graph.key,
                    expected: graph.digest
                },
                ResourceEdit::Remove {
                    key: geometry.key,
                    expected: geometry.digest
                },
            ],
            Default::default()
        )
        .is_ok());
}

#[test]
fn explicit_guarded_companion_graph_edit_may_retain_identical_authoritative_bytes() {
    use crate::container_10d::{
        material_container::attach_material_section, MaterialRecord, SubmeshRange,
    };
    let (bytes, _) = mixed("urn:checkpoint:explicit-identical-companion");
    let base = decode(&bytes, Default::default()).unwrap();
    let geometry = base
        .resources()
        .iter()
        .find(|entry| entry.kind == "10d")
        .unwrap();
    let graph = base
        .resources()
        .iter()
        .find(|entry| entry.kind == "q42")
        .unwrap();
    let material = MaterialRecord {
        roughness: 0.37,
        ..MaterialRecord::legacy_default()
    };
    let changed = attach_material_section(
        &geometry.bytes,
        &[material],
        &[SubmeshRange {
            first_index: 0,
            index_count: 3,
            material_id: material.id,
            semantic_id: 0,
        }],
        3,
    )
    .unwrap();
    let replacement = ResourceEdit::ReplaceBytes {
        key: geometry.key.clone(),
        expected: geometry.summary().digest,
        bytes: changed,
    };
    assert!(base
        .apply_resource_edits(vec![replacement.clone()], Default::default())
        .is_err());
    let candidate = base
        .apply_resource_edits(
            vec![
                replacement,
                ResourceEdit::ReplaceBytes {
                    key: graph.key.clone(),
                    expected: graph.summary().digest,
                    bytes: graph.bytes.clone(),
                },
            ],
            Default::default(),
        )
        .unwrap();
    assert_eq!(
        candidate
            .resources()
            .iter()
            .find(|entry| entry.key == graph.key)
            .unwrap(),
        graph
    );
    // This establishes explicit, guarded intent and byte preservation. Matching
    // scientific/semantic geometry-to-graph claims still require their owner.
}

#[cfg(unix)]
#[test]
fn symlink_inputs_and_destinations_cannot_mutate_external_resources() {
    let (bytes, _) = mixed("urn:checkpoint:resource-symlink");
    let dir = tempfile::tempdir().unwrap();
    let outside = dir.path().join("outside.hmc");
    std::fs::write(&outside, &bytes).unwrap();
    let link = dir.path().join("link.hmc");
    std::os::unix::fs::symlink(&outside, &link).unwrap();
    assert!(load(&link, Default::default()).is_err());
    assert!(save(
        &link,
        &bytes,
        ExpectedFile::Sha256(super::codec::digest(&bytes)),
        Default::default()
    )
    .is_err());
    assert_eq!(std::fs::read(outside).unwrap(), bytes);
}
