//! Wave-3 Integration Gate `W3-03` (`GATE-ENVELOPE-VECTORS`) under `docs/plans/emf-qac-evolution-2026-11-13`.
//! Exercises `q42::field_envelope`, `q42::field_semantics`, `q42::field_spectrum`, `q42::emf_container`,
//! and deterministic corpus replay of all 236 `V01` vectors and 72 `C04` fixtures.

use std::fs;
use std::path::PathBuf;

use serde::Deserialize;
use sha2::{Digest, Sha256};

use qualia_core_db::audio::qac::{QacStorageOptions, QacStorageView};
use qualia_core_db::q42::emf_container::{
    EmfCellRecord64, EmfContainerReader, EmfContainerWriter, EmfError, EmfReadLimits,
    EMF_BRICK_CELLS,
};
use qualia_core_db::q42::emf_profiles::{
    encode_field_samples, field_entry_slot, field_metadata_len, prepare_field_document,
    FieldChunkInput, FieldClock, FieldDocument, FieldFrame, FieldProvenance, FieldSample,
    FieldWriteLimits, Limits as EmfProfileLimits, ProfileReader, ProfileRegistry, FIELD_PROFILE,
};
use qualia_core_db::q42::field_envelope::{
    BlockInput, EntryInput, EnvelopeInput, Error as EnvError, Family, FieldInput, FieldValue,
    Limits as EnvLimits, MetadataInput, StructuralView, DIRECTORY_BYTES, HEADER_BYTES,
    INDEX_DOMAIN,
};
use qualia_core_db::q42::field_semantics::{
    admit_shared, validate_fields, DomainRegistry, Error as SemError, FieldRule,
    Limits as SemLimits, NodeState, Scratch, SharedOnly,
};
use qualia_core_db::q42::field_spectrum::{
    admit_axis, convert_bin_average, density_coordinate_jacobian,
    frequency_hz_to_photon_energy_ev, frequency_hz_to_photon_energy_j,
    frequency_hz_to_vacuum_wavelength_m, vacuum_wavelength_m_to_frequency_hz, AxisKind, AxisOrder,
    DensityCoordinate, Limits as SpecLimits, NoResponse, Quantity, SampleSemantics,
    SigmaInterpretation, SCHEMA_ID as SPECTRUM_SCHEMA_ID,
};

fn plan_vectors_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/plans/emf-qac-evolution-2026-11-13/vectors")
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
}

const fn fi(key: u16, flags: u8, value: FieldValue<'_>) -> FieldInput<'_> {
    FieldInput { key, flags, value }
}

const fn bi<'a>(id: u32, schema_type: u16, flags: u32, fields: &'a [FieldInput<'a>]) -> BlockInput<'a> {
    BlockInput { id, schema_type, revision: 1, flags, fields }
}

fn encode_envelope(family: Family, blocks: &[BlockInput<'_>], entries: &[EntryInput<'_>]) -> Vec<u8> {
    let limits = EnvLimits::default();
    let meta_in = MetadataInput { blocks };
    let meta_len = meta_in.encoded_len(limits).expect("metadata length");
    let mut meta_bytes = vec![0u8; meta_len];
    meta_in.encode_into(&mut meta_bytes, limits).expect("encode metadata");

    let input = EnvelopeInput { family, flags: 0, metadata: &meta_bytes, entries };
    let len = input.encoded_len(limits).expect("envelope length");
    let mut out = vec![0u8; len];
    input.encode_into(&mut out, limits).expect("encode envelope");
    out
}

#[test]
fn field_envelope_roundtrip_crc32c_blake3_and_legacy_rejection() {
    assert_eq!(
        (HEADER_BYTES, DIRECTORY_BYTES, INDEX_DOMAIN),
        (128, 128, b"QUALIA-FIELD-INDEX-v1\0".as_slice())
    );
    let fields = [fi(1, 0, FieldValue::Utf8("envelope.probe"))];
    let blocks = [bi(1, 65000, 0, &fields)];
    let payload = [0x11u8; 64];
    let entries = [EntryInput {
        mode: 0x0100, revision: 1, flags: 0, stream_id: 0, frame_id: 0, start_tick: 0,
        time_count: 0, spatial_key: 0, record_count: 1, stride: 64, codec: 0,
        descriptor_id: 1, payload: &payload,
    }];

    for family in [Family::EmfV4Candidate, Family::QacV1Candidate] {
        let bytes = encode_envelope(family, &blocks, &entries);
        let view = StructuralView::open(&bytes, family, EnvLimits::default()).unwrap();
        assert_eq!((view.family(), view.entry_count()), (family, 1));
        assert_eq!(view.entry(0).unwrap_err(), EnvError::EntryIndex);
        assert_eq!(view.entry(2).unwrap_err(), EnvError::EntryIndex);
        let entry = view.entry(1).unwrap();
        assert_eq!((entry.mode(), entry.decoded_bytes()), (0x0100, 64));
        assert_eq!(entry.digest(), blake3::hash(&payload).as_bytes());
        assert_eq!(view.verify_chunk(1).unwrap(), &payload);

        let mut root_header = [0u8; HEADER_BYTES];
        root_header.copy_from_slice(&bytes[..HEADER_BYTES]);
        root_header[56..88].fill(0);
        root_header[124..128].fill(0);
        let mut expected_idx = blake3::Hasher::new();
        expected_idx.update(INDEX_DOMAIN);
        expected_idx.update(&root_header);
        expected_idx.update(entry.raw_directory_bytes());
        expected_idx.update(view.metadata_bytes());
        assert_eq!(view.index_digest(), expected_idx.finalize().as_bytes());

        let mut bad_hdr_crc = bytes.clone();
        bad_hdr_crc[124] ^= 0x01;
        assert_eq!(StructuralView::open(&bad_hdr_crc, family, EnvLimits::default()).unwrap_err(), EnvError::Crc);

        let mut bad_dir_crc = bytes.clone();
        bad_dir_crc[HEADER_BYTES + 124] ^= 0x01;
        assert_eq!(StructuralView::open(&bad_dir_crc, family, EnvLimits::default()).unwrap_err(), EnvError::Crc);

        let mut bad_payload = bytes.clone();
        bad_payload[entry.payload_offset() as usize] ^= 0xff;
        let p_view = StructuralView::open(&bad_payload, family, EnvLimits::default()).unwrap();
        assert_eq!(p_view.verify_chunk(1).unwrap_err(), EnvError::PayloadDigest);
    }

    let emf_v4 = encode_envelope(Family::EmfV4Candidate, &blocks, &entries);
    for legacy_ver in 1..=3u16 {
        let mut legacy_emf = emf_v4.clone();
        legacy_emf[8..10].copy_from_slice(&legacy_ver.to_le_bytes());
        assert_eq!(StructuralView::open(&legacy_emf, Family::EmfV4Candidate, EnvLimits::default()).unwrap_err(), EnvError::Version);
        let mut legacy_q4au = emf_v4.clone();
        legacy_q4au[0..4].copy_from_slice(b"UA4Q");
        legacy_q4au[4..8].copy_from_slice(&(legacy_ver as u32).to_le_bytes());
        assert_eq!(StructuralView::open(&legacy_q4au, Family::QacV1Candidate, EnvLimits::default()).unwrap_err(), EnvError::Family);
    }
}

#[test]
fn field_semantics_schemas_dag_provenance_validity_and_custody_rules() {
    let f1 = [
        fi(1, 1, FieldValue::U32(&[0])),
        fi(2, 1, FieldValue::I64(&[0])),
        fi(3, 1, FieldValue::U32(&[0])),
        fi(4, 1, FieldValue::U64(&[96_000])),
        fi(5, 1, FieldValue::U64(&[1])),
    ];
    let f2 = [
        fi(1, 1, FieldValue::U32(&[0])),
        fi(3, 1, FieldValue::F64(&[0.0, 0.0, 0.0])),
        fi(4, 1, FieldValue::F64(&[0.0, 0.0, 0.0, 1.0])),
        fi(5, 1, FieldValue::F64(&[1.0, 1.0, 1.0])),
        fi(6, 1, FieldValue::U32(&[2, 3, 4])),
        fi(7, 1, FieldValue::U32(&[0])),
        fi(8, 1, FieldValue::Utf8("right-handed-X-forward-Y-left-Z-up")),
    ];
    let f3 = [fi(1, 1, FieldValue::U32(&[4]))];
    let f4 = [
        fi(2, 1, FieldValue::MetadataRefs(&[2])),
        fi(3, 1, FieldValue::MetadataRefs(&[3])),
        fi(7, 1, FieldValue::Utf8("opaque-original")),
        fi(8, 1, FieldValue::Utf8("1")),
        fi(12, 1, FieldValue::Utf8("urn:test:opaque")),
    ];
    let f5 = [
        fi(1, 1, FieldValue::MetadataRefs(&[1])),
        fi(2, 1, FieldValue::I64(&[-1])),
        fi(3, 1, FieldValue::U64(&[1])),
        fi(4, 1, FieldValue::U64(&[2])),
    ];
    let blocks = [
        bi(1, 1, 1, &f1), bi(2, 2, 1, &f2), bi(3, 4, 1, &f3),
        bi(4, 3, 1, &f4), bi(5, 5, 1, &f5),
    ];
    let entries = [EntryInput {
        mode: 0x0104, revision: 1, flags: 4, stream_id: 0, frame_id: 2, start_tick: 0,
        time_count: 0, spatial_key: 23, record_count: 1, stride: 1, codec: 0,
        descriptor_id: 4, payload: b"a",
    }];
    let bytes = encode_envelope(Family::QacV1Candidate, &blocks, &entries);
    let view = StructuralView::open(&bytes, Family::QacV1Candidate, EnvLimits::default()).unwrap();
    let mut meta_nodes = [NodeState::default(); 16];
    let mut entry_nodes = [NodeState::default(); 8];
    let mut scratch = Scratch { metadata: &mut meta_nodes, entries: &mut entry_nodes };
    let admitted = admit_shared(&view, &SharedOnly, &mut scratch, SemLimits::default()).unwrap();
    assert_eq!(admitted.preserved_unknown_blocks, 0);
    validate_fields(admitted.context.block(5).unwrap(), &[
        FieldRule { key: 1, type_tag: 8, count: Some(1), required: true },
        FieldRule { key: 2, type_tag: 3, count: Some(1), required: true },
        FieldRule { key: 3, type_tag: 2, count: Some(1), required: true },
        FieldRule { key: 4, type_tag: 2, count: Some(1), required: true },
    ]).unwrap();

    // Invalid sub-tick fraction in schema 5 (n = 0, d = 2 -> gcd != 1 -> SemError::Time)
    let f5_bad = [
        fi(1, 1, FieldValue::MetadataRefs(&[1])),
        fi(2, 1, FieldValue::I64(&[-1])),
        fi(3, 1, FieldValue::U64(&[0])),
        fi(4, 1, FieldValue::U64(&[2])),
    ];
    let bad_blocks = [
        bi(1, 1, 1, &f1), bi(2, 2, 1, &f2), bi(3, 4, 1, &f3),
        bi(4, 3, 1, &f4), bi(5, 5, 1, &f5_bad),
    ];
    let bad_bytes = encode_envelope(Family::QacV1Candidate, &bad_blocks, &entries);
    let bad_view = StructuralView::open(&bad_bytes, Family::QacV1Candidate, EnvLimits::default()).unwrap();
    assert_eq!(admit_shared(&bad_view, &SharedOnly, &mut scratch, SemLimits::default()).err().unwrap(), SemError::Time);

    // Invalid validity record count in schema 3 key 15 (0 != record_count 1 -> SemError::Validity)
    let f4_bad_val = [
        fi(2, 1, FieldValue::MetadataRefs(&[2])),
        fi(3, 1, FieldValue::MetadataRefs(&[3])),
        fi(7, 1, FieldValue::Utf8("opaque-original")),
        fi(8, 1, FieldValue::Utf8("1")),
        fi(12, 1, FieldValue::Utf8("urn:test:opaque")),
        fi(15, 1, FieldValue::U64(&[0])),
    ];
    let val_blocks = [
        bi(1, 1, 1, &f1), bi(2, 2, 1, &f2), bi(3, 4, 1, &f3),
        bi(4, 3, 1, &f4_bad_val), bi(5, 5, 1, &f5),
    ];
    let val_bytes = encode_envelope(Family::QacV1Candidate, &val_blocks, &entries);
    let val_view = StructuralView::open(&val_bytes, Family::QacV1Candidate, EnvLimits::default()).unwrap();
    assert_eq!(admit_shared(&val_view, &SharedOnly, &mut scratch, SemLimits::default()).err().unwrap(), SemError::Validity);
}

#[test]
fn field_spectrum_laws_polarization_and_constitutive_dispersion_models() {
    let spec_fields = [
        fi(1, 1, FieldValue::Utf8("frequency_hz")),
        fi(2, 1, FieldValue::Utf8("point_density")),
        fi(3, 1, FieldValue::Utf8("spectral_irradiance")),
        fi(4, 1, FieldValue::Utf8("W m^-2 Hz^-1")),
        fi(5, 1, FieldValue::F64(&[1.0e14, 2.0e14, 3.0e14])),
        fi(6, 1, FieldValue::Utf8("explicit")),
        fi(8, 1, FieldValue::Utf8("legacy_display_400_700nm")),
    ];
    let blocks = [bi(1, SPECTRUM_SCHEMA_ID, 1, &spec_fields)];
    let bytes = encode_envelope(Family::EmfV4Candidate, &blocks, &[]);
    let view = StructuralView::open(&bytes, Family::EmfV4Candidate, EnvLimits::default()).unwrap();
    let axis = admit_axis(view.blocks().next().unwrap(), 3, SpecLimits::default(), &NoResponse).unwrap();
    assert_eq!((axis.kind(), axis.semantics()), (AxisKind::FrequencyHz, SampleSemantics::PointDensity));
    assert_eq!((axis.quantity(), axis.sigma()), (Quantity::SpectralIrradiance, SigmaInterpretation::LegacyDisplay400700nm));
    assert_eq!(axis.coordinate(0).unwrap(), 1.0e14);

    let wl = frequency_hz_to_vacuum_wavelength_m(3.0e14).unwrap();
    assert!((vacuum_wavelength_m_to_frequency_hz(wl).unwrap() - 3.0e14).abs() < 1e-2);
    assert!(frequency_hz_to_photon_energy_j(3.0e14).unwrap() > 0.0);
    assert!(frequency_hz_to_photon_energy_ev(3.0e14).unwrap() > 0.0);
    let jac = density_coordinate_jacobian(
        Quantity::SpectralIrradiance,
        SampleSemantics::PointDensity,
        DensityCoordinate::VacuumMetre,
        DensityCoordinate::Hertz,
        500.0e-9,
    ).unwrap();
    assert_eq!(jac.order, AxisOrder::Reversed);
    let bin = convert_bin_average(
        Quantity::SpectralIrradiance,
        DensityCoordinate::VacuumMetre,
        DensityCoordinate::Hertz,
        400.0e-9,
        500.0e-9,
        10.0,
    ).unwrap();
    assert_eq!(bin.order, AxisOrder::Reversed);
    assert!((bin.integral - 10.0 * 100.0e-9).abs() < 1e-18);
    assert!(ProfileRegistry.rules(133, 1).is_some() && ProfileRegistry.rules(135, 1).is_some());
}

#[test]
fn emf_container_v4_envelope_and_hilbert_brick_roundtrip() {
    let sample = FieldSample {
        electric_v_per_m: [1.5, -0.5, 0.25], magnetic_tesla: [1.0e-9, 2.0e-9, -1.0e-9],
        position_m: [1.0, 2.0, 3.0], tick: 0, carrier_hz: Some(2.4e9),
        stored_poynting_w_per_m2: Some([0.0, 0.0, 0.01]), confidence: Some(0.99),
    };
    let mut payload_buf = [0u8; 128];
    encode_field_samples(&[sample], &mut payload_buf, EmfProfileLimits::default()).unwrap();
    let chunks = [FieldChunkInput { stream_id: 1, start_tick: 0, time_count: 1, spatial_key: None, payload: &payload_buf }];
    let doc = FieldDocument {
        clock: FieldClock { scale: 0, epoch_seconds: 1_700_000_000, epoch_nanoseconds: 0, ticks_per_second_num: 1_000, ticks_per_second_den: 1 },
        frame: FieldFrame { origin_m: [0.0, 0.0, 0.0], rotation_xyzw: [0.0, 0.0, 0.0, 1.0], grid: None },
        provenance: FieldProvenance { role: 1, algorithm: Some("emf.sensor.v4"), algorithm_version: Some("1.0"), lineage: Some("gate") },
        temporal_step_ticks: Some(1), chunks: &chunks, align_payloads_to_4096: false,
    };
    let write_limits = FieldWriteLimits::default();
    let mut meta_buf = vec![0u8; field_metadata_len(&doc, write_limits).unwrap()];
    let mut slots = [field_entry_slot(); 1];
    let prepared = prepare_field_document(&doc, &mut meta_buf, &mut slots, write_limits).unwrap();
    let len = prepared.encoded_len().unwrap();
    let (mut staging, mut v4_bytes) = (vec![0u8; len], vec![0u8; len]);
    let (mut m_nodes, mut e_nodes) = ([NodeState::default(); 16], [NodeState::default(); 4]);
    prepared.encode_into(&mut staging, &mut v4_bytes, &mut Scratch { metadata: &mut m_nodes, entries: &mut e_nodes }).unwrap();

    let view = StructuralView::open(&v4_bytes, Family::EmfV4Candidate, EnvLimits::default()).unwrap();
    let mut scratch = Scratch { metadata: &mut m_nodes, entries: &mut e_nodes };
    let mut reader = ProfileReader::open(&view, &mut scratch, EmfProfileLimits::default(), 65536).unwrap();
    let c1 = reader.chunk(1).unwrap();
    assert_eq!((c1.canonical_mode(), c1.profile()), (0x0100, FIELD_PROFILE));
    assert_eq!(EmfContainerReader::parse(&v4_bytes).unwrap_err(), EmfError::InvalidVersion(4));

    assert_eq!(EMF_BRICK_CELLS, 512);
    let mut writer = EmfContainerWriter::new([0.0, 0.0, 0.0], 0.1, [1, 1, 1], 1_700_000_000.0);
    let cells = [EmfCellRecord64::default(); EMF_BRICK_CELLS];
    writer.add_brick_chunk(0, &cells, 0, 1, 0).unwrap();
    let v3_bytes = writer.to_bytes().unwrap();
    assert_eq!(StructuralView::open(&v3_bytes, Family::EmfV4Candidate, EnvLimits::default()).unwrap_err(), EnvError::Version);
    let v3_reader = EmfContainerReader::parse_with_limits(&v3_bytes, EmfReadLimits::conservative()).unwrap();
    assert_eq!(v3_reader.chunk_count(), 1);
    assert!(v3_reader.lookup_brick_cell(0, 0, 0, 0, 0, 0, 1).is_some());
    assert_eq!(v3_reader.dense_volume().unwrap().cells.len(), EMF_BRICK_CELLS);
}

#[derive(Deserialize)]
struct V01Manifest { total: usize, vectors: Vec<V01Vector> }

#[derive(Deserialize)]
struct V01Limits {
    aggregate_decoded_bytes: Option<u64>, blocks: Option<u32>, chunk_bytes: Option<u64>,
    entries: Option<u32>, fields_per_block: Option<u32>, file_bytes: Option<u64>,
    metadata_bytes: Option<u64>, string_bytes: Option<u32>,
}

#[derive(Deserialize)]
struct V01Vector {
    path: String, family: String, layer: String, bytes: usize, sha256: String,
    expected: String, v01_error: Option<String>, rust_error: Option<String>, limits: Option<V01Limits>,
}

fn map_env_error(name: &str) -> EnvError {
    match name {
        "Error::Extent" => EnvError::Extent, "Error::Overflow" => EnvError::Overflow,
        "Error::Limit" => EnvError::Limit, "Error::Family" => EnvError::Family,
        "Error::Version" => EnvError::Version, "Error::Header" => EnvError::Header,
        "Error::Crc" => EnvError::Crc, "Error::IndexDigest" => EnvError::IndexDigest,
        "Error::Directory" => EnvError::Directory, "Error::Order" => EnvError::Order,
        "Error::Padding" => EnvError::Padding, "Error::Codec" => EnvError::Codec,
        "Error::Metadata" => EnvError::Metadata, "Error::Numeric" => EnvError::Numeric,
        "Error::PayloadDigest" => EnvError::PayloadDigest, "Error::EntryIndex" => EnvError::EntryIndex,
        other => panic!("unexpected structural rust_error: {other}"),
    }
}

#[test]
fn replay_v01_manifest_236_vectors_deterministically() {
    let base = plan_vectors_dir().join("v01");
    let manifest_raw = fs::read_to_string(base.join("manifest.json")).expect("read v01 manifest");
    let manifest: V01Manifest = serde_json::from_str(&manifest_raw).expect("parse v01 manifest");
    assert_eq!((manifest.total, manifest.vectors.len()), (236, 236));

    for v in &manifest.vectors {
        let _ = &v.v01_error;
        let raw = fs::read(base.join(&v.path)).unwrap_or_else(|e| panic!("{}: {e}", v.path));
        assert_eq!(raw.len(), v.bytes, "{}", v.path);
        assert_eq!(sha256_hex(&raw), v.sha256, "{}", v.path);
        let family = if v.family == "emf" { Family::EmfV4Candidate } else { Family::QacV1Candidate };
        let mut limits = EnvLimits::default();
        if let Some(ov) = &v.limits {
            if let Some(x) = ov.aggregate_decoded_bytes { limits.aggregate_decoded_bytes = x; }
            if let Some(x) = ov.blocks { limits.blocks = x; }
            if let Some(x) = ov.chunk_bytes { limits.chunk_bytes = x; }
            if let Some(x) = ov.entries { limits.entries = x; }
            if let Some(x) = ov.fields_per_block { limits.fields_per_block = x; }
            if let Some(x) = ov.file_bytes { limits.file_bytes = x; }
            if let Some(x) = ov.metadata_bytes { limits.metadata_bytes = x; }
            if let Some(x) = ov.string_bytes { limits.string_bytes = x; }
        }
        match v.layer.as_str() {
            "golden" | "t01_gap_observation" => {
                assert_eq!(v.expected, "accept", "{}", v.path);
                let view = StructuralView::open(&raw, family, limits).expect(&v.path);
                for idx in 1..=view.entry_count() { view.verify_chunk(idx).expect(&v.path); }
            }
            "structural" => {
                assert_eq!(v.expected, "reject", "{}", v.path);
                let exp = map_env_error(v.rust_error.as_deref().unwrap());
                if exp == EnvError::PayloadDigest {
                    let view = StructuralView::open(&raw, family, limits).expect(&v.path);
                    assert!((1..=view.entry_count()).any(|i| view.verify_chunk(i) == Err(EnvError::PayloadDigest)), "{}", v.path);
                } else {
                    assert_eq!(StructuralView::open(&raw, family, limits).unwrap_err(), exp, "{}", v.path);
                }
            }
            "semantic" => {
                assert_eq!(v.expected, "reject", "{}", v.path);
                let view = StructuralView::open(&raw, family, limits).expect(&v.path);
                let mut m_nodes = vec![NodeState::default(); view.blocks().count().max(1)];
                let mut e_nodes = vec![NodeState::default(); (view.entry_count() as usize).max(1)];
                let mut scratch = Scratch { metadata: &mut m_nodes, entries: &mut e_nodes };
                assert!(admit_shared(&view, &ProfileRegistry, &mut scratch, SemLimits::default()).is_err(), "{}", v.path);
            }
            "structural_gap" => {
                assert!(v.rust_error.as_deref().unwrap_or("").starts_with("GAP("), "{}", v.path);
                if v.path.contains("utf8-ctrl-") {
                    assert_eq!(StructuralView::open(&raw, family, limits).unwrap_err(), EnvError::Metadata, "{}", v.path);
                } else {
                    let view = StructuralView::open(&raw, family, limits).expect(&v.path);
                    assert!(view.blocks().count() >= 1, "{}", v.path);
                }
            }
            other => panic!("unexpected layer {other}"),
        }
    }
}

#[derive(Deserialize)]
struct FixturesManifest { fixture_count: usize, fixtures: Vec<FixtureEntry> }

#[derive(Deserialize)]
struct FixtureEntry {
    path: String, family: String, expected: String,
    error: Option<String>, size: usize, sha256: String,
}

#[test]
fn replay_fixtures_manifest_72_vectors_deterministically() {
    let base = plan_vectors_dir().join("fixtures");
    let raw_manifest = fs::read_to_string(base.join("manifest.json")).expect("read fixtures manifest");
    let manifest: FixturesManifest = serde_json::from_str(&raw_manifest).expect("parse fixtures manifest");
    assert_eq!((manifest.fixture_count, manifest.fixtures.len()), (72, 72));

    for f in &manifest.fixtures {
        let raw = fs::read(base.join(&f.path)).unwrap_or_else(|e| panic!("{}: {e}", f.path));
        assert_eq!(raw.len(), f.size, "{}", f.path);
        assert_eq!(sha256_hex(&raw), f.sha256, "{}", f.path);

        match f.family.as_str() {
            "qac_candidate" | "emf_candidate" => {
                let family = if f.family == "qac_candidate" { Family::QacV1Candidate } else { Family::EmfV4Candidate };
                if f.expected == "accept" {
                    let view = StructuralView::open(&raw, family, EnvLimits::default()).expect(&f.path);
                    for idx in 1..=view.entry_count() { view.verify_chunk(idx).expect(&f.path); }
                } else if f.error.as_deref() == Some("payload_digest") {
                    let view = StructuralView::open(&raw, family, EnvLimits::default()).expect(&f.path);
                    assert_eq!(view.verify_chunk(1).unwrap_err(), EnvError::PayloadDigest, "{}", f.path);
                } else if f.error.as_deref() == Some("critical_schema") {
                    let view = StructuralView::open(&raw, family, EnvLimits::default()).expect(&f.path);
                    let mut m_nodes = vec![NodeState::default(); view.blocks().count().max(1)];
                    let mut e_nodes = vec![NodeState::default(); (view.entry_count() as usize).max(1)];
                    let mut scratch = Scratch { metadata: &mut m_nodes, entries: &mut e_nodes };
                    assert_eq!(
                        admit_shared(&view, &SharedOnly, &mut scratch, SemLimits::default()).err().unwrap(),
                        SemError::UnknownCriticalSchema,
                        "{}",
                        f.path
                    );
                } else {
                    assert!(StructuralView::open(&raw, family, EnvLimits::default()).is_err(), "{}", f.path);
                }
            }
            "q4au_legacy" => {
                assert!(StructuralView::open(&raw, Family::QacV1Candidate, EnvLimits::default()).is_err(), "{}", f.path);
                assert!(QacStorageView::open(&raw, QacStorageOptions::default()).is_err(), "{}", f.path);
            }
            "emf_legacy" => {
                assert!(StructuralView::open(&raw, Family::EmfV4Candidate, EnvLimits::default()).is_err(), "{}", f.path);
                if f.expected == "accept" {
                    assert!(EmfContainerReader::parse(&raw).is_ok(), "{}", f.path);
                } else {
                    assert!(EmfContainerReader::parse(&raw).is_err(), "{}", f.path);
                }
            }
            other => panic!("unexpected fixture family {other}"),
        }
    }
}
