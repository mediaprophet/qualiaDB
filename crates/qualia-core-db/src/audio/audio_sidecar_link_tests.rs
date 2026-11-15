#![cfg(not(target_arch = "wasm32"))]
use super::*;
use crate::audio::qac::signal::{
    writer::{
        PcmDocument, PcmQuantity, PcmSamples, Provenance, ProvenanceRole, SignalRelation,
        WriteBuffers,
    },
    Rate, Timing,
};
use crate::q42::field_semantics as sem;
fn doc<'a>(samples: &'a [f32]) -> PcmDocument<'a> {
    PcmDocument {
        samples: PcmSamples::F32(samples),
        timing: Timing {
            scale: 0,
            epoch_seconds: 0,
            epoch_nanoseconds: 0,
            ticks: Rate::new(48_000, 1).unwrap(),
            samples: Rate::new(48_000, 1).unwrap(),
            start_tick: 0,
        },
        stream_id: 1,
        channels: 1,
        channel_order: "channel-A",
        purpose: "native-default-fixture",
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
    }
}
#[test]
fn full_digest_formatter_retains_last_byte_and_caller_tail() {
    let mut a = [0u8; 32];
    a[0] = 0xab;
    a[31] = 0xcd;
    let mut out = [0xa5; 96];
    let n = format_sidecar_relpath(&a, &mut out);
    assert_eq!(
        std::str::from_utf8(&out[..n]).unwrap(),
        format!("spectral/audio/{}.qac", blake3::Hash::from(a).to_hex())
    );
    assert!(out[n..].iter().all(|x| *x == 0xa5));
    let first = out;
    let mut b = a;
    b[31] ^= 1;
    format_sidecar_relpath(&b, &mut out);
    assert_ne!(first, out);
    let mut tiny = [0xa5; 82];
    assert_eq!(format_sidecar_relpath(&a, &mut tiny), 0);
    assert!(tiny.iter().all(|x| *x == 0xa5));
}
#[test]
fn registry_index_remains_distinct_from_artifact_digest() {
    let q = compile_spectral_sheet_quin(0xabc, u32::MAX);
    assert_eq!(q.subject, 0xabc);
    assert_eq!(q.predicate, PRED_HAS_SPECTRAL_SHEET);
    assert_eq!(q.object, u64::from(u32::MAX));
    assert_eq!(
        crate::tensor::bake_pipeline::sigma_sheet_index_from_nquin(&q),
        Some(u32::MAX)
    );
}
#[test]
fn native_default_produces_only_complete_digest_current_qac_and_reuses_exact_bytes() {
    let root = tempfile::tempdir().unwrap();
    let values = [f32::from_bits(0x80000000), f32::from_bits(1), 1.0];
    let document = doc(&values);
    let mut metadata = [0; 4096];
    let mut payload = [0; 4096];
    let mut file = [0; 16384];
    let mut out = [0xa5; 16384];
    let mut m = [sem::NodeState::default(); 32];
    let mut e = m;
    let artifact = produce_pcm(
        root.path(),
        &document,
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
        ProductionOptions::default(),
    )
    .unwrap();
    let bytes = &out[..artifact.byte_length as usize];
    assert_eq!(std::fs::read(&artifact.path).unwrap(), bytes);
    assert_eq!(artifact.blake3, *blake3::hash(bytes).as_bytes());
    let mut rel = [0; 96];
    let n = format_sidecar_relpath(&artifact.blake3, &mut rel);
    assert!(artifact
        .path
        .ends_with(std::str::from_utf8(&rel[..n]).unwrap()));
    let same = publish_current_signal(
        root.path(),
        bytes,
        &mut sem::Scratch {
            metadata: &mut m,
            entries: &mut e,
        },
        ProductionOptions::default(),
    )
    .unwrap();
    assert_eq!(same, artifact);
    let names = std::fs::read_dir(artifact.path.parent().unwrap())
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect::<Vec<_>>();
    assert_eq!(names, vec![artifact.path]);
}
#[test]
fn malformed_native_inputs_do_not_publish_or_mutate_final_output() {
    let root = tempfile::tempdir().unwrap();
    let absent = root.path().join("not-created");
    let values = [1f32, -1.];
    let mut document = doc(&values);
    document.quantity = PcmQuantity::Pressure;
    let mut metadata = [0; 4096];
    let mut payload = [0; 4096];
    let mut file = [0; 16384];
    let mut out = [0xa5; 16384];
    let mut m = [sem::NodeState::default(); 32];
    let mut e = m;
    assert!(produce_pcm(
        &absent,
        &document,
        WriteBuffers {
            metadata: &mut metadata,
            payload: &mut payload,
            file: &mut file
        },
        &mut sem::Scratch {
            metadata: &mut m,
            entries: &mut e
        },
        &mut out,
        ProductionOptions::default()
    )
    .is_err());
    assert!(out.iter().all(|x| *x == 0xa5));
    assert!(!absent.exists());
    assert!(publish_current_signal(
        &absent,
        b"not a current signal",
        &mut sem::Scratch {
            metadata: &mut m,
            entries: &mut e
        },
        ProductionOptions::default()
    )
    .is_err());
    assert!(!absent.exists());
}
