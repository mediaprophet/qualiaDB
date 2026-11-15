use super::super::{
    create_artifact_if_absent, get_artifact_stamped, mutate_artifact_stamped,
    redo_artifact_stamped, undo_artifact_stamped, StudioArtifact,
};
use super::*;
use crate::bundle::{BundleReader, BundleWriter};
use std::path::Path;

const CELL1: &str =
    "let poet_geometry = { vertices: [[0,0,0],[1,0,0],[0,1,0]], triangles: [[0,1,2]] };\n";
const CELL2: &str = "let poet_material = { base_color: [0.17,0.26,0.08,1.0], metallic: 0.0, roughness: 0.72 };\n{geometry: poet_geometry, material: poet_material}";

fn fixture(id: &str) -> (StudioArtifact, super::super::StoreStamp) {
    let mut initial = StudioArtifact::new(id, "Geometry", "Editable source — retained provenance");
    initial
        .properties
        .insert("source.identifier.scheme".into(), "urn:poet:source".into());
    initial
        .properties
        .insert("contribution.status".into(), "unpaid; unreviewed".into());
    let (_, stamp) = create_artifact_if_absent(initial).unwrap();
    let (_, stamp) = mutate_artifact_stamped(id, &stamp, |a| {
        a.nodes
            .entry("poet.native.submission/00000000000000000001".into())
            .or_default()
            .insert("text".into(), CELL1.into());
        Ok(())
    })
    .unwrap();
    let (_, stamp) = mutate_artifact_stamped(id, &stamp, |a| {
        a.nodes
            .entry("poet.native.submission/00000000000000000002".into())
            .or_default()
            .insert("text".into(), CELL2.into());
        Ok(())
    })
    .unwrap();
    undo_artifact_stamped(id, &stamp).unwrap()
}

fn bytes(id: &str) -> (StudioArtifact, Vec<u8>) {
    let (artifact, stamp) = fixture(id);
    let checkpoint = capture(id, &stamp, Default::default()).unwrap();
    let asset = crate::render::compile_10d::compile_asset(
        b"v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
        Some("obj"),
        "urn:source:checkpoint-derived",
        "obj",
    )
    .unwrap();
    let bytes = encode(
        &checkpoint,
        vec![("derived/geometry.10d".into(), asset)],
        Default::default(),
    )
    .unwrap();
    (artifact, bytes)
}

#[test]
fn exact_source_schema_lineage_and_real_10d_roundtrip() {
    let (artifact, bytes) = bytes("hmc-roundtrip");
    assert_eq!(&bytes[..4], b"QBDL");
    let decoded = decode(&bytes, Default::default()).unwrap();
    assert_eq!(decoded.artifact(), &artifact);
    assert_eq!(decoded.file_digest(), Some(super::codec::digest(&bytes)));
    let linked = decoded.with_parent_digest([7; 32]);
    let next = encode(&linked, vec![], Default::default()).unwrap();
    assert_eq!(
        decode(&next, Default::default()).unwrap().parent_digest(),
        Some([7; 32])
    );
    let reader = BundleReader::parse(&bytes).unwrap();
    let graph = reader.get("graphs/derived/geometry.10d.q42").unwrap();
    let native_graph = crate::q42_volume::WasmSliceQ42Reader::open(graph).unwrap();
    assert!(native_graph.is_unified_volume());
    assert!(!native_graph.collect_all_quins().unwrap().is_empty());
    let mesh =
        crate::render::compile_10d::decode_10d_mesh(reader.get("derived/geometry.10d").unwrap())
            .unwrap();
    assert_eq!(mesh.vertex_count(), 3);
    assert_eq!(mesh.triangle_count(), 1);
}

#[test]
fn tamper_truncation_schema_sequence_and_budget_fail_without_promotion() {
    let (_, original) = bytes("hmc-invalid");
    let mut tampered = original.clone();
    let source_offset = BundleReader::parse(&original)
        .unwrap()
        .entry("authoring/native-source.json")
        .unwrap()
        .offset as usize;
    tampered[source_offset] ^= 1;
    assert!(decode(&tampered, Default::default()).is_err());
    for size in [0, 1, 63, original.len() - 1] {
        assert!(decode(&original[..size], Default::default()).is_err());
    }
    let mut tiny = CheckpointLimits::default();
    tiny.file_bytes = original.len() - 1;
    assert!(decode(&original, tiny).is_err());
    for change in ["schema", "cells", "journal"] {
        let r = BundleReader::parse(&original).unwrap();
        let mut record: serde_json::Value =
            serde_json::from_slice(r.get("authoring/native-source.json").unwrap()).unwrap();
        // Use a previously unseen target to prove refusal precedes activation.
        record["artifact"]["id"] = "hmc-never-activated".into();
        match change {
            "schema" => record["schema"] = "unknown/2".into(),
            "cells" => {
                let nodes = record["artifact"]["nodes"].as_object_mut().unwrap();
                let cell = nodes
                    .remove("poet.native.submission/00000000000000000001")
                    .unwrap();
                nodes.insert("poet.native.submission/00000000000000000003".into(), cell);
            }
            _ => record["journal"]["limits"]["steps"] = 1000000.into(),
        }
        let mut w = BundleWriter::new();
        w.add_file(
            "authoring/native-source.json",
            "application/vnd.qualia.studio-source+json",
            serde_json::to_vec(&record).unwrap(),
            None,
        )
        .unwrap();
        assert!(decode(&w.build().unwrap(), Default::default()).is_err());
        assert!(get_artifact_stamped("hmc-never-activated").is_none());
    }
}

#[test]
fn stale_capture_and_activation_conflict_preserve_accepted_work() {
    let (artifact, stamp) = fixture("hmc-conflict");
    let cp = capture(&artifact.id, &stamp, Default::default()).unwrap();
    let before = get_artifact_stamped(&artifact.id).unwrap();
    assert!(activate(cp, Default::default()).is_err());
    assert_eq!(get_artifact_stamped(&artifact.id).unwrap(), before);
    let (_, fresh) = redo_artifact_stamped(&artifact.id, &stamp).unwrap();
    assert!(capture(&artifact.id, &stamp, Default::default()).is_err());
    assert!(capture(&artifact.id, &fresh, Default::default()).is_ok());
    let cp = capture(&artifact.id, &fresh, Default::default()).unwrap();
    // Test-only removal simulates process loss while retaining the old stamp.
    super::super::store().lock().unwrap().remove(&artifact.id);
    let (_, new_incarnation) = activate(cp, Default::default()).unwrap();
    assert_ne!(new_incarnation, fresh);
    assert!(!super::super::artifact_stamp_is_current(
        &artifact.id,
        &fresh
    ));
}

#[test]
fn native_save_conflicts_invalid_bytes_and_budget_preserve_destination() {
    let (_, bytes) = bytes("hmc-file");
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("source.hmc");
    let limits = CheckpointLimits::default();
    let saved = save(&path, &bytes, ExpectedFile::Absent, limits).unwrap();
    assert!(save(&path, &bytes, ExpectedFile::Absent, limits).is_err());
    assert!(save(&path, &bytes, ExpectedFile::Sha256([0; 32]), limits).is_err());
    assert!(save(
        &path,
        b"bad bytes",
        ExpectedFile::Sha256(saved.digest),
        limits
    )
    .is_err());
    let mut tiny = limits;
    tiny.file_bytes = bytes.len() - 1;
    assert!(save(&path, &bytes, ExpectedFile::Sha256(saved.digest), tiny).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    save(&path, &bytes, ExpectedFile::Sha256(saved.digest), limits).unwrap();
    assert_eq!(
        load(&path, limits).unwrap().file_digest(),
        Some(saved.digest)
    );
    let lock = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(dir.path().join("source.hmc.poet-source.lock"))
        .unwrap();
    lock.try_lock().unwrap();
    assert!(save(&path, &bytes, ExpectedFile::Sha256(saved.digest), limits).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    drop(lock);
    assert!(save(
        &dir.path().join("missing/source.hmc"),
        &bytes,
        ExpectedFile::Absent,
        limits
    )
    .is_err());
    assert!(!std::fs::read_dir(dir.path()).unwrap().any(|entry| entry
        .unwrap()
        .file_name()
        .to_string_lossy()
        .starts_with(".poet-source-")));
}

#[test]
fn restart_process_restores_exact_source_and_private_undo_redo() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("restart.hmc");
    for mode in ["write", "read"] {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["checkpoint_child", "--nocapture", "--test-threads=1"])
            .env("POET_CHECKPOINT_CHILD_PATH", &path)
            .env("POET_CHECKPOINT_CHILD_MODE", mode)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "child {mode}: {} {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn checkpoint_child() {
    let Some(path) = std::env::var_os("POET_CHECKPOINT_CHILD_PATH") else {
        return;
    };
    let path = Path::new(&path);
    if std::env::var("POET_CHECKPOINT_CHILD_MODE").unwrap() == "write" {
        let (_, bytes) = bytes("hmc-restart-source");
        save(path, &bytes, ExpectedFile::Absent, Default::default()).unwrap();
    } else {
        assert!(get_artifact_stamped("hmc-restart-source").is_none());
        let cp = load(path, Default::default()).unwrap();
        // The entire current artifact equality includes all identifiers, source
        // nodes/properties, audit, metadata and redo markers, before activation.
        let before = cp.artifact().clone();
        let mut expected = StudioArtifact::new(
            "hmc-restart-source",
            "Geometry",
            "Editable source — retained provenance",
        );
        expected
            .properties
            .insert("source.identifier.scheme".into(), "urn:poet:source".into());
        expected
            .properties
            .insert("contribution.status".into(), "unpaid; unreviewed".into());
        expected
            .nodes
            .entry("poet.native.submission/00000000000000000001".into())
            .or_default()
            .insert("text".into(), CELL1.into());
        expected.lamport_clock = 4;
        expected.history = ["edit:1->2", "edit:2->3", "undo:3->4"]
            .map(String::from)
            .to_vec();
        expected.redo_stack = vec!["clock:3".into()];
        assert_eq!(before, expected);
        assert_eq!(before.lamport_clock, 4);
        assert_eq!(before.nodes.len(), 1);
        assert_eq!(before.nodes.values().next().unwrap()["text"], CELL1);
        let (active, stamp) = activate(cp, Default::default()).unwrap();
        assert_eq!(active, before);
        let (redone, stamp) = redo_artifact_stamped(&active.id, &stamp).unwrap();
        assert_eq!(
            redone.nodes["poet.native.submission/00000000000000000002"]["text"],
            CELL2
        );
        let (undone, stamp) = undo_artifact_stamped(&active.id, &stamp).unwrap();
        assert_eq!(undone.nodes, before.nodes);
        let (original, _) = undo_artifact_stamped(&active.id, &stamp).unwrap();
        assert!(original.nodes.is_empty());
    }
}

#[cfg(windows)]
mod windows_install {
    use super::*;
    use std::io::Write;
    use std::os::windows::fs::OpenOptionsExt;

    fn blocked_destination(dir: &Path) -> (std::path::PathBuf, std::fs::File) {
        let path = dir.join("blocked.hmc");
        std::fs::write(&path, b"old checkpoint").unwrap();
        // Permit reads/writes, but refuse deletion/rename while this handle lives.
        let held = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(3)
            .open(&path)
            .unwrap();
        (path, held)
    }
    fn staged(dir: &Path) -> tempfile::NamedTempFile {
        let mut file = tempfile::Builder::new()
            .prefix(".poet-source-")
            .tempfile_in(dir)
            .unwrap();
        file.write_all(b"new checkpoint").unwrap();
        file.as_file().sync_all().unwrap();
        file
    }
    fn clean(dir: &Path) {
        assert!(!std::fs::read_dir(dir).unwrap().any(|entry| entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".poet-source-")));
    }
    #[test]
    fn real_deny_delete_handle_release_allows_bounded_retry() {
        let dir = tempfile::tempdir().unwrap();
        let (path, held) = blocked_destination(dir.path());
        let mut held = Some(held);
        let mut observed = 0;
        let (installed, attempts) = super::super::native_io::persist_staged(
            staged(dir.path()),
            &path,
            ExpectedFile::Sha256(super::super::codec::digest(b"old checkpoint")),
            1024,
            |_| {
                observed += 1;
                drop(held.take());
            },
        )
        .unwrap();
        assert!(observed >= 1 && attempts >= 2 && attempts <= 7);
        drop(installed);
        assert_eq!(std::fs::read(&path).unwrap(), b"new checkpoint");
        clean(dir.path());
    }
    #[test]
    fn permanent_deny_delete_refusal_preserves_destination_and_cleans_staging() {
        let dir = tempfile::tempdir().unwrap();
        let (path, held) = blocked_destination(dir.path());
        let mut observed = 0;
        let error = super::super::native_io::persist_staged(
            staged(dir.path()),
            &path,
            ExpectedFile::Sha256(super::super::codec::digest(b"old checkpoint")),
            1024,
            |_| {
                observed += 1;
            },
        )
        .unwrap_err();
        assert!(observed >= 1 && observed <= 6);
        for context in [
            "retry budget exhausted",
            "first persist error:",
            "last persist error:",
            "destination",
            "staged",
        ] {
            assert!(error.contains(context), "{error}");
        }
        assert_eq!(std::fs::read(&path).unwrap(), b"old checkpoint");
        drop(held);
        clean(dir.path());
    }
    #[test]
    fn changed_expected_bytes_stop_retry_before_replacement() {
        let dir = tempfile::tempdir().unwrap();
        let (path, held) = blocked_destination(dir.path());
        let mut held = Some(held);
        let mut observed = 0;
        let error = super::super::native_io::persist_staged(
            staged(dir.path()),
            &path,
            ExpectedFile::Sha256(super::super::codec::digest(b"old checkpoint")),
            1024,
            |_| {
                observed += 1;
                std::fs::write(&path, b"another writer's checkpoint").unwrap();
                drop(held.take());
            },
        )
        .unwrap_err();
        assert_eq!(observed, 1);
        assert!(error.contains("destination digest changed"), "{error}");
        assert_eq!(
            std::fs::read(&path).unwrap(),
            b"another writer's checkpoint"
        );
        clean(dir.path());
    }
}
