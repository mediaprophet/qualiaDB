use super::*;
use crate::audio::audio_sidecar_link::{
    format_sidecar_relpath, publish_current_signal, ProductionOptions,
};
use crate::audio::qac::signal::{
    writer::{
        PcmDocument, PcmQuantity, PcmSamples, Provenance, ProvenanceRole, SignalRelation,
        WriteBuffers, WriteOptions,
    },
    Rate, Timing,
};
use crate::q42::field_semantics as sem;
const CURRENT: &[u8] =
    include_bytes!("../../q42/field_envelope/fixtures/candidate/qac_candidate-opaque.qac");
fn current_options() -> QacPublishOptions {
    QacPublishOptions::default()
}
fn current_object(samples: &[f32], rate: u64) -> Vec<u8> {
    let document = PcmDocument {
        samples: PcmSamples::F32(samples),
        timing: Timing {
            scale: 0,
            epoch_seconds: 0,
            epoch_nanoseconds: 0,
            ticks: Rate::new(rate, 1).unwrap(),
            samples: Rate::new(rate, 1).unwrap(),
            start_tick: 0,
        },
        stream_id: 1,
        channels: 1,
        channel_order: "channel-A",
        purpose: "publication-fixture",
        quantity: PcmQuantity::DigitalSample,
        relation: SignalRelation::DigitalUnrelatedToPressure,
        units_per_stored_sample: 1.0,
        stored_offset: 0.0,
        provenance: Provenance {
            role: ProvenanceRole::Model,
            algorithm: Some("urn:test:fixture"),
            algorithm_version: Some("1"),
            instrument_uri: None,
            lineage: None,
        },
        original: None,
    };
    let mut metadata = [0; 4096];
    let mut payload = [0; 4096];
    let mut file = [0; 16384];
    let mut out = vec![0; 16384];
    let mut m = [sem::NodeState::default(); 32];
    let mut e = m;
    let n = document
        .encode_into(
            WriteBuffers {
                metadata: &mut metadata,
                payload: &mut payload,
                file: &mut file,
            },
            &mut sem::Scratch {
                metadata: &mut m,
                entries: &mut e,
            },
            &mut out,
            WriteOptions::default(),
        )
        .unwrap();
    out.truncate(n);
    out
}
fn publish_native(root: &Path, bytes: &[u8]) -> QacArtifact {
    let mut m = [sem::NodeState::default(); 32];
    let mut e = m;
    publish_current_signal(
        root,
        bytes,
        &mut sem::Scratch {
            metadata: &mut m,
            entries: &mut e,
        },
        ProductionOptions::default(),
    )
    .unwrap()
}

fn names(dir: &Path) -> Vec<PathBuf> {
    let mut paths: Vec<_> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    paths.sort();
    paths
}

#[test]
fn final_bytes_distinguish_actual_samples_frame_counts_and_rates() {
    let root = tempfile::tempdir().unwrap();
    let data = [0.5f32, 1.0, -1.0];
    let other = [-0.5f32, 1.0];
    let mut objects = Vec::new();
    for (samples, rate) in [
        (&data[..2], 48_000),
        (&data[..2], 44_100),
        (&data[..3], 48_000),
        (&other[..], 48_000),
    ] {
        let bytes = current_object(samples, rate);
        let artifact = publish_native(root.path(), &bytes);
        assert_eq!(artifact.blake3, *blake3::hash(&bytes).as_bytes());
        assert_eq!(fs::read(&artifact.path).unwrap(), bytes);
        objects.push(artifact);
    }
    for (i, a) in objects.iter().enumerate() {
        for b in &objects[i + 1..] {
            assert_ne!(a.path, b.path)
        }
    }
    assert_eq!(names(objects[0].path.parent().unwrap()).len(), 4);
}

#[test]
fn deterministic_reuse_checks_existing_exact_bytes_and_cleans_scratch() {
    let root = tempfile::tempdir().unwrap();
    let artifact = publish_qac(root.path(), CURRENT, current_options()).unwrap();
    assert_eq!(
        publish_qac(root.path(), CURRENT, current_options()).unwrap(),
        artifact
    );
    assert_eq!(
        names(artifact.path.parent().unwrap()),
        vec![artifact.path.clone()]
    );
    let mut corrupt = CURRENT.to_vec();
    corrupt[20] ^= 1; // same length, different bytes at the expected digest path
    fs::write(&artifact.path, &corrupt).unwrap();
    assert!(matches!(
        publish_qac(root.path(), CURRENT, current_options()),
        Err(QacPublishError::Conflict)
    ));
    assert_eq!(fs::read(&artifact.path).unwrap(), corrupt);
    assert_eq!(
        names(artifact.path.parent().unwrap()),
        vec![artifact.path.clone()]
    );
    fs::write(&artifact.path, b"short").unwrap();
    assert!(matches!(
        publish_qac(root.path(), CURRENT, current_options()),
        Err(QacPublishError::Conflict)
    ));
    assert_eq!(names(artifact.path.parent().unwrap()), vec![artifact.path]);
}

#[test]
fn native_writer_has_no_alias_and_preserves_full_digest_identity() {
    let root = tempfile::tempdir().unwrap();
    let a = current_object(&[1f32, -1.], 48_000);
    let b = current_object(&[0.5f32, -0.5], 48_000);
    let one = publish_native(root.path(), &a);
    let two = publish_native(root.path(), &b);
    assert_ne!(one.blake3, two.blake3);
    assert_eq!(publish_native(root.path(), &a), one);
    for artifact in [&one, &two] {
        let mut rel = [0; 96];
        let n = format_sidecar_relpath(&artifact.blake3, &mut rel);
        assert!(artifact
            .path
            .ends_with(std::str::from_utf8(&rel[..n]).unwrap()));
        let bytes = fs::read(&artifact.path).unwrap();
        assert_eq!(artifact.blake3, *blake3::hash(&bytes).as_bytes());
    }
    let paths = names(one.path.parent().unwrap());
    assert_eq!(paths.len(), 2);
    assert!(paths.iter().all(|p| p.extension().unwrap() == "qac"));
}

#[test]
fn budget_and_admission_fail_before_storage_creation() {
    let root = tempfile::tempdir().unwrap();
    let absent = root.path().join("absent");
    let options = QacPublishOptions {
        max_bytes: CURRENT.len() - 1,
        ..Default::default()
    };
    assert!(matches!(
        publish_qac(&absent, CURRENT, options),
        Err(QacPublishError::Budget)
    ));
    assert!(matches!(
        publish_qac(&absent, b"invalid", current_options()),
        Err(QacPublishError::Admission(_))
    ));
    assert!(!absent.exists());
}

#[test]
fn current_family_policy_and_chunk_integrity_are_checked() {
    let root = tempfile::tempdir().unwrap();
    let options = QacPublishOptions::default();
    assert!(publish_qac(
        root.path(),
        include_bytes!("fixtures/q4au-v2-contract.qac"),
        options
    )
    .is_err());
    let artifact = publish_qac(root.path(), CURRENT, options).unwrap();
    assert_eq!(
        artifact.admission,
        QacPublicationAdmission::CurrentStructure
    );
    assert_eq!(fs::read(artifact.path).unwrap(), CURRENT);
    let bad =
        include_bytes!("../../q42/field_envelope/fixtures/malformed/qac_candidate-payload.qac");
    assert!(matches!(
        publish_qac(root.path(), bad, options),
        Err(QacPublishError::Admission(QacStorageError::Envelope(_)))
    ));
}

#[test]
fn non_file_conflict_and_unwind_leave_no_staging_files() {
    let root = tempfile::tempdir().unwrap();
    let artifact = publish_qac(root.path(), CURRENT, current_options()).unwrap();
    fs::remove_file(&artifact.path).unwrap();
    fs::create_dir(&artifact.path).unwrap();
    assert!(publish_qac(root.path(), CURRENT, current_options()).is_err());
    assert_eq!(
        names(artifact.path.parent().unwrap()),
        vec![artifact.path.clone()]
    );
    let dir = artifact.path.parent().unwrap();
    let before = names(dir);
    // Interrupt the production staging lifecycle after write/sync, before publish.
    let _ = std::panic::catch_unwind(|| {
        let _staging = stage_file(&artifact.path, CURRENT).unwrap();
        panic!("simulated interruption before publication");
    });
    assert_eq!(names(dir), before);
}
