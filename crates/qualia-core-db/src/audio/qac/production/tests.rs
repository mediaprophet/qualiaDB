use super::*;
use crate::audio::qac::signal::{
    self,
    writer::{PcmQuantity, PcmSamples, Provenance, ProvenanceRole, SignalRelation},
    Rate, Sample, Timing,
};
use crate::audio::qac::{
    persistence::{QacArtifact, QacPublishError},
    signal::writer::{PcmDocument, WriteBuffers, WriteOptions},
};
use crate::q42::{
    field_envelope as env,
    field_envelope::{Family, StructuralView},
    field_semantics as sem,
};
use std::fs;
use std::path::Path;
#[path = "../signal/test_support.rs"]
mod signal_support;
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
        channel_order: "sensor-A",
        purpose: "actual-original-samples",
        quantity: PcmQuantity::DigitalSample,
        relation: SignalRelation::DigitalUnrelatedToPressure,
        units_per_stored_sample: 1.0,
        stored_offset: 0.0,
        provenance: Provenance {
            role: ProvenanceRole::Measured,
            algorithm: None,
            algorithm_version: None,
            instrument_uri: None,
            lineage: None,
        },
        original: None,
    }
}
fn encode(document: &PcmDocument<'_>) -> Vec<u8> {
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
fn produce(
    root: &Path,
    document: &PcmDocument<'_>,
    options: ProductionOptions,
) -> Result<(QacArtifact, Vec<u8>), Error> {
    let mut metadata = [0; 4096];
    let mut payload = [0; 4096];
    let mut file = [0; 16384];
    let mut out = vec![0xa5; 16384];
    let mut m = [sem::NodeState::default(); 32];
    let mut e = m;
    let artifact = produce_pcm(
        root,
        document,
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
        options,
    )?;
    out.truncate(artifact.byte_length as usize);
    Ok((artifact, out))
}
fn publish(root: &Path, bytes: &[u8]) -> Result<QacArtifact, Error> {
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
}
#[test]
fn actual_samples_publish_readback_and_identical_reuse_are_exact() {
    let root = tempfile::tempdir().unwrap();
    let values = [f32::from_bits(0x80000000), f32::from_bits(1), 1.0, -1.0];
    let mut document = doc(&values);
    document.original = Some(b"exact opaque original\0\xff");
    let (a, bytes) = produce(root.path(), &document, ProductionOptions::default()).unwrap();
    assert_eq!(fs::read(&a.path).unwrap(), bytes);
    assert_eq!(a.blake3, *blake3::hash(&bytes).as_bytes());
    assert_eq!(
        a.path.file_name().unwrap().to_str().unwrap(),
        format!("{}.qac", blake3::hash(&bytes).to_hex())
    );
    let (b, again) = produce(root.path(), &document, ProductionOptions::default()).unwrap();
    assert_eq!(a, b);
    assert_eq!(bytes, again);
    assert_eq!(publish(root.path(), &bytes).unwrap(), a);
    let view =
        StructuralView::open(&bytes, Family::QacV1Candidate, env::Limits::default()).unwrap();
    let mut m = [sem::NodeState::default(); 32];
    let mut e = m;
    let decoded = signal::admit(
        &view,
        2,
        &mut sem::Scratch {
            metadata: &mut m,
            entries: &mut e,
        },
        sem::Limits::default(),
        signal::Limits::default(),
    )
    .unwrap();
    for (i, bits) in [0x80000000u32, 1, 0x3f800000, 0xbf800000]
        .into_iter()
        .enumerate()
    {
        let Sample::F32(value) = decoded.sample(i as u64, 0, 0).unwrap() else {
            panic!()
        };
        assert_eq!(value.to_bits(), bits)
    }
    assert_eq!(view.verify_chunk(1).unwrap(), document.original.unwrap());
    let names = fs::read_dir(a.path.parent().unwrap())
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect::<Vec<_>>();
    assert_eq!(names, vec![a.path]);
    assert!(names.iter().all(|p| p.extension().unwrap() == "qac"));
}
#[test]
fn actual_data_and_metadata_identity_never_use_lossy_preview_aliases() {
    let root = tempfile::tempdir().unwrap();
    let first = [1.0f32, -1.0];
    let second = [0.5f32, -0.5];
    assert_eq!(first.iter().sum::<f32>(), second.iter().sum::<f32>());
    let (a, _) = produce(root.path(), &doc(&first), ProductionOptions::default()).unwrap();
    let (b, _) = produce(root.path(), &doc(&second), ProductionOptions::default()).unwrap();
    assert_ne!(a.blake3, b.blake3);
    let mut changed = doc(&first);
    changed.purpose = "different-acquisition-purpose";
    let (c, _) = produce(root.path(), &changed, ProductionOptions::default()).unwrap();
    assert_ne!(a.blake3, c.blake3);
    assert_eq!(fs::read_dir(a.path.parent().unwrap()).unwrap().count(), 3);
}
#[test]
fn typed_invalid_documents_and_budget_failures_create_no_artifact() {
    let root = tempfile::tempdir().unwrap();
    let absent = root.path().join("never-created");
    let values = [1.0f32, -1.0];
    let mut document = doc(&values);
    document.quantity = PcmQuantity::Pressure;
    assert!(produce(&absent, &document, ProductionOptions::default()).is_err());
    assert!(!absent.exists());
    let bytes = encode(&doc(&values));
    // Replace only the units metadata through the shared builder, then reseal the
    // structural envelope; index integrity is valid but scientific pairing is wrong.
    let view =
        StructuralView::open(&bytes, Family::QacV1Candidate, env::Limits::default()).unwrap();
    let fspec: Vec<Vec<env::FieldInput<'_>>> = view
        .blocks()
        .map(|b| {
            b.fields()
                .map(|f| env::FieldInput {
                    key: f.key(),
                    flags: f.flags(),
                    value: if b.id() == 5 && f.key() == 8 {
                        env::FieldValue::Utf8("Pa")
                    } else {
                        env::FieldValue::Raw {
                            type_tag: f.type_tag(),
                            element_count: f.element_count(),
                            bytes: f.value(),
                        }
                    },
                })
                .collect()
        })
        .collect();
    let blocks: Vec<_> = view
        .blocks()
        .zip(&fspec)
        .map(|(b, fields)| env::BlockInput {
            id: b.id(),
            schema_type: b.schema_type(),
            revision: b.revision(),
            flags: b.flags(),
            fields,
        })
        .collect();
    let mut metadata = [0; 4096];
    let meta = env::MetadataInput { blocks: &blocks }
        .encode_into(&mut metadata, env::Limits::default())
        .unwrap();
    let entry = view.entry(1).unwrap();
    let entries = [env::EntryInput {
        mode: entry.mode(),
        revision: 1,
        flags: entry.flags(),
        stream_id: entry.stream_id(),
        frame_id: 0,
        start_tick: 0,
        time_count: entry.time_count(),
        spatial_key: 0,
        record_count: entry.record_count(),
        stride: entry.stride(),
        codec: 0,
        descriptor_id: entry.descriptor_id(),
        payload: view.verify_chunk(1).unwrap(),
    }];
    let mut invalid = [0; 16384];
    let invalid = env::EnvelopeInput {
        family: Family::QacV1Candidate,
        flags: 0,
        metadata: meta,
        entries: &entries,
    }
    .encode_into(&mut invalid, env::Limits::default())
    .unwrap();
    assert!(StructuralView::open(invalid, Family::QacV1Candidate, env::Limits::default()).is_ok());
    assert!(matches!(publish(&absent, invalid), Err(Error::Signal(_))));
    assert!(!absent.exists());
    let mut nodes = [sem::NodeState::default(); 32];
    let mut entries = nodes;
    let mut options = ProductionOptions::default();
    options.writer.signal.work = 0;
    assert!(matches!(
        publish_current_signal(
            &absent,
            &bytes,
            &mut sem::Scratch {
                metadata: &mut nodes,
                entries: &mut entries
            },
            options
        ),
        Err(Error::Signal(signal::Error::Limit))
    ));
    assert!(!absent.exists());
    assert!(produce(
        &absent,
        &doc(&values),
        ProductionOptions {
            publication_max_bytes: 1,
            ..Default::default()
        }
    )
    .is_err());
    assert!(!absent.exists());
    assert!(publish(&absent, b"not a current QAC signal").is_err());
    assert!(!absent.exists());
}
#[test]
fn publication_conflict_is_visible_and_existing_bytes_are_not_replaced() {
    let root = tempfile::tempdir().unwrap();
    let values = [1f32, -1.];
    let (a, bytes) = produce(root.path(), &doc(&values), ProductionOptions::default()).unwrap();
    let corrupt = vec![0xcc; bytes.len()];
    fs::write(&a.path, &corrupt).unwrap();
    assert!(matches!(
        produce(root.path(), &doc(&values), ProductionOptions::default()),
        Err(Error::Publication(QacPublishError::Conflict))
    ));
    assert_eq!(fs::read(&a.path).unwrap(), corrupt);
    assert_eq!(fs::read_dir(a.path.parent().unwrap()).unwrap().count(), 1);
}
#[test]
fn already_current_rectangular_spectra_publish_but_hann_is_not_reinterpreted() {
    let root = tempfile::tempdir().unwrap();
    let payload = include_bytes!("../signal/fixtures/stft-n4-imaginary.le");
    let bytes = signal_support::build(
        payload,
        signal_support::Config {
            spectral: true,
            scalar: 6,
            ..Default::default()
        },
        |_| {},
    );
    let artifact = publish(root.path(), &bytes).unwrap();
    assert_eq!(fs::read(artifact.path).unwrap(), bytes);
    let unsupported = signal_support::build(
        payload,
        signal_support::Config {
            spectral: true,
            scalar: 6,
            ..Default::default()
        },
        |blocks| {
            *signal_support::field(blocks, 5, 4) = signal_support::strf(4, "Hann");
        },
    );
    let absent = root.path().join("unsupported");
    assert!(publish(&absent, &unsupported).is_err());
    assert!(!absent.exists());
}

#[test]
fn produce_stft_publishes_admitted_hann_and_pcm_artifact_without_clobber() {
    use crate::audio::qac::{
        metadata::{OriginBasis, WindowKind},
        signal::stft::{StftDocument, StftQuantity, StftWriteBuffers},
    };
    let root = tempfile::tempdir().unwrap();
    let samples = [0.25f32, -0.5, 0.75, -0.25, 0.5, -0.75];
    let stft_doc = StftDocument {
        samples: PcmSamples::F32(&samples),
        timing: doc(&samples).timing,
        stream_id: 9,
        channels: 1,
        channel_order: "mic-A",
        fft_size: 4,
        hop: 2,
        pcm_purpose: "original-acquisition",
        stft_purpose: "hann-complex-stft",
        quantity: StftQuantity::DigitalSample,
        relation: SignalRelation::DigitalUnrelatedToPressure,
        pcm_provenance: doc(&samples).provenance,
        stft_provenance: Provenance {
            role: ProvenanceRole::Derived,
            algorithm: Some("urn:etalon:signal:hann-stft-f32"),
            algorithm_version: Some("1"),
            instrument_uri: None,
            lineage: Some("symmetric-hann-radix2-no-padding"),
        },
        original: None,
    };
    let mut mono = [0.0f32; 16];
    let mut fft = [0.0f32; 32];
    let mut win = [0.0f32; 16];
    let mut spectra = [0u8; 4096];
    let mut metadata = [0u8; 8192];
    let mut payload = [0u8; 4096];
    let mut file = [0u8; 16384];
    let mut final_bytes = vec![0xa5u8; 16384];
    let mut m = [sem::NodeState::default(); 32];
    let mut e = m;
    let artifact = produce_stft(
        root.path(),
        &stft_doc,
        StftWriteBuffers {
            mono: &mut mono,
            fft: &mut fft,
            window: &mut win,
            spectra: &mut spectra,
            metadata: &mut metadata,
            payload: &mut payload,
            file: &mut file,
        },
        &mut sem::Scratch {
            metadata: &mut m,
            entries: &mut e,
        },
        &mut final_bytes,
        StftProductionOptions::default(),
    )
    .unwrap();
    let published = &final_bytes[..artifact.byte_length as usize];
    assert_eq!(fs::read(&artifact.path).unwrap(), published);
    assert_eq!(publish(root.path(), published).unwrap(), artifact);
    let view =
        StructuralView::open(published, Family::QacV1Candidate, env::Limits::default()).unwrap();
    let mut m2 = [sem::NodeState::default(); 32];
    let mut e2 = m2;
    let stft_view = signal::admit(
        &view,
        stft_doc.stft_entry_index(),
        &mut sem::Scratch {
            metadata: &mut m2,
            entries: &mut e2,
        },
        sem::Limits::default(),
        signal::Limits::default(),
    )
    .unwrap();
    let tx = stft_view.transform().unwrap();
    assert_eq!(tx.window_kind(), WindowKind::HannSymmetricF32);
    assert_eq!(tx.origin_basis(), OriginBasis::SourceEntryFirstSample);
    assert!(stft_view.window_coefficients_le().is_some());
}
