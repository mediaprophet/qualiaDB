//! Cross-subsystem integration gate for the EMF/QAC evolution programme (`E01`, `Q01..=Q05`,
//! `T01`, `T03`, `T04`, `D02`).

use qualia_core_db::audio::qac::profiles::{convert_sn3d_n3d, HoaNormalization};
use qualia_core_db::audio::qac::signal::writer::{
    PcmDocument, PcmQuantity, PcmSamples, Provenance, ProvenanceRole, SignalRelation,
    WriteBuffers, WriteOptions,
};
use qualia_core_db::audio::qac::signal::{
    admit as admit_qac_signal, Limits as QacSignalLimits, Rate, Timing,
};
use qualia_core_db::audio::qac::{QacStorageOptions, QacStorageView};
use qualia_core_db::medical::dicom_codecs::jxl::{
    reconstruct_jpeg_from_jbrd, JxlError, JxlLimits,
};
use qualia_core_db::q42::emf_profiles::{
    encode_field_samples, field_entry_slot, field_metadata_len, prepare_field_document,
    FieldChunkInput, FieldClock, FieldDocument, FieldFrame, FieldProvenance, FieldSample,
    FieldWriteLimits, Limits as EmfLimits, ProfileReader, FIELD_PROFILE,
};
use qualia_core_db::q42::field_envelope::{Family, Limits as EnvelopeLimits, StructuralView};
use qualia_core_db::q42::field_semantics::{Limits as SemanticLimits, NodeState, Scratch};
use qualia_core_db::q42::material_coupling::{
    compute_blake3_256, reject_spatial_key_only_join, MaterialBindingError,
};
use qualia_core_db::q42::science_validation::passivity_conservation::{
    evaluate_qac_fluid_mode00_oracle, verify_yee_cfl_and_dispersion,
    ConservationValidationError,
};
use qualia_core_db::q42::science_validation::stokes_mueller::{
    jones_vector_to_stokes, verify_stokes_realizability, Complex64, PolarimetricValidationError,
};
use qualia_core_db::sensing::authority::{CalibratedSeriesState, CalibrationError};
use qualia_core_db::sensing::SemanticId;

#[test]
fn emf_v4_and_qac_v1_deterministic_roundtrip_and_profile_admission() {
    let sample = FieldSample {
        electric_v_per_m: [1.25, -2.5, 0.5],
        magnetic_tesla: [1e-6, -2e-6, 3e-6],
        position_m: [10.0, 20.0, -5.0],
        tick: 0,
        carrier_hz: Some(2.4e9),
        stored_poynting_w_per_m2: Some([0.1, 0.2, 0.3]),
        confidence: Some(0.95),
    };
    let mut payload = [0u8; 128];
    encode_field_samples(&[sample], &mut payload, EmfLimits::default()).unwrap();

    let chunks = [FieldChunkInput {
        stream_id: 0,
        start_tick: 0,
        time_count: 0,
        spatial_key: None,
        payload: &payload,
    }];
    let document = FieldDocument {
        clock: FieldClock {
            scale: 0,
            epoch_seconds: 1_700_000_000,
            epoch_nanoseconds: 0,
            ticks_per_second_num: 1000,
            ticks_per_second_den: 1,
        },
        frame: FieldFrame {
            origin_m: [0.0, 0.0, 0.0],
            rotation_xyzw: [0.0, 0.0, 0.0, 1.0],
            grid: None,
        },
        provenance: FieldProvenance {
            role: 1,
            algorithm: Some("emf-qac-evolution-gate"),
            algorithm_version: Some("1"),
            lineage: Some("deterministic gate"),
        },
        temporal_step_ticks: None,
        chunks: &chunks,
        align_payloads_to_4096: false,
    };

    let write_limits = FieldWriteLimits::default();
    let mut meta = vec![0u8; field_metadata_len(&document, write_limits).unwrap()];
    let mut entries = [field_entry_slot(); 1];
    let prepared = prepare_field_document(&document, &mut meta, &mut entries, write_limits).unwrap();
    let encoded_len = prepared.encoded_len().unwrap();
    let mut staging = vec![0u8; encoded_len];
    let mut out_a = vec![0u8; encoded_len];
    let mut out_b = vec![0u8; encoded_len];
    let mut m_nodes = [NodeState::default(); 8];
    let mut e_nodes = [NodeState::default(); 2];
    let mut scratch = Scratch {
        metadata: &mut m_nodes,
        entries: &mut e_nodes,
    };
    let emf_bytes = prepared
        .encode_into(&mut staging, &mut out_a, &mut scratch)
        .unwrap()
        .to_vec();
    let emf_bytes_b = prepared
        .encode_into(&mut staging, &mut out_b, &mut scratch)
        .unwrap()
        .to_vec();
    assert_eq!(emf_bytes, emf_bytes_b);

    let emf_view =
        StructuralView::open(&emf_bytes, Family::EmfV4Candidate, EnvelopeLimits::default()).unwrap();
    let mut emf_rm = [NodeState::default(); 8];
    let mut emf_re = [NodeState::default(); 2];
    let mut emf_rscratch = Scratch {
        metadata: &mut emf_rm,
        entries: &mut emf_re,
    };
    let mut reader =
        ProfileReader::open(&emf_view, &mut emf_rscratch, EmfLimits::default(), 64 << 20).unwrap();
    let emf_admitted = reader.chunk(1).unwrap();
    assert_eq!(emf_admitted.profile(), FIELD_PROFILE);
    let rec = emf_admitted.field(0).unwrap().unwrap();
    assert_eq!(rec.electric_v_per_m(), sample.electric_v_per_m);
    assert_eq!(rec.carrier_hz(), sample.carrier_hz);

    let native_pcm = [0.25_f32, -0.5, 0.75, -0.125];
    let pcm_doc = PcmDocument {
        samples: PcmSamples::F32(&native_pcm),
        timing: Timing {
            scale: 2,
            epoch_seconds: 1_700_000_000,
            epoch_nanoseconds: 0,
            ticks: Rate::new(48_000, 1).unwrap(),
            samples: Rate::new(48_000, 1).unwrap(),
            start_tick: 0,
        },
        stream_id: 1,
        channels: 1,
        channel_order: "mono",
        purpose: "original-acquisition",
        quantity: PcmQuantity::DigitalSample,
        relation: SignalRelation::DigitalUnrelatedToPressure,
        units_per_stored_sample: 1.0,
        stored_offset: 0.0,
        provenance: Provenance {
            role: ProvenanceRole::Measured,
            algorithm: None,
            algorithm_version: None,
            instrument_uri: Some("urn:gate:mic"),
            lineage: None,
        },
        original: None,
    };
    let mut q_meta = [0u8; 4096];
    let mut q_pay = [0u8; 4096];
    let mut q_file = [0u8; 16384];
    let mut q_out = vec![0u8; 16384];
    let mut q_m = [NodeState::default(); 32];
    let mut q_e = q_m;
    let q_len = pcm_doc
        .encode_into(
            WriteBuffers {
                metadata: &mut q_meta,
                payload: &mut q_pay,
                file: &mut q_file,
            },
            &mut Scratch {
                metadata: &mut q_m,
                entries: &mut q_e,
            },
            &mut q_out,
            WriteOptions::default(),
        )
        .unwrap();
    q_out.truncate(q_len);

    let qac_view = QacStorageView::open(
        &q_out,
        QacStorageOptions {
            envelope_limits: EnvelopeLimits::default(),
        },
    )
    .unwrap();
    let mut r_m = [NodeState::default(); 32];
    let mut r_e = r_m;
    let sig = admit_qac_signal(
        qac_view.structure(),
        1,
        &mut Scratch {
            metadata: &mut r_m,
            entries: &mut r_e,
        },
        SemanticLimits::default(),
        QacSignalLimits::default(),
    )
    .unwrap();
    assert_eq!(sig.as_bytes().len(), native_pcm.len() * 4);
}

#[test]
fn hoa_normalization_science_authority_and_jbrd_fail_closed_gates() {
    let sn3d = [1.0_f32, 0.5, -0.5, 0.25];
    let mut n3d = [0.0_f32; 4];
    convert_sn3d_n3d(1, &sn3d, HoaNormalization::Sn3d, HoaNormalization::N3d, &mut n3d).unwrap();
    let mut back = [0.0_f32; 4];
    convert_sn3d_n3d(1, &n3d, HoaNormalization::N3d, HoaNormalization::Sn3d, &mut back).unwrap();
    for (&orig, &rt) in sn3d.iter().zip(back.iter()) {
        assert!((orig - rt).abs() <= 1e-6);
    }

    assert_eq!(
        reject_spatial_key_only_join(0x1234_5678),
        Err(MaterialBindingError::SpatialKeyAloneInsufficient)
    );
    assert_eq!(compute_blake3_256(b"gate"), compute_blake3_256(b"gate"));

    let q_id = SemanticId::new(101).unwrap();
    let u_id = SemanticId::new(202).unwrap();
    assert!(CalibratedSeriesState::from_raw_counts(q_id, u_id, [10.0, 20.0]).is_ok());
    assert_eq!(
        CalibratedSeriesState::from_raw_counts(q_id, u_id, [f64::NAN, 1.0]),
        Err(CalibrationError::NonFiniteSample)
    );

    assert!(verify_yee_cfl_and_dispersion(
        [1e-3, 1e-3, 1e-3],
        1.5e-12,
        1.0,
        1.0,
        [100.0, 0.0, 0.0]
    )
    .is_ok());
    assert!(matches!(
        verify_yee_cfl_and_dispersion([1e-3, 1e-3, 1e-3], 1.0, 1.0, 1.0, [1.0, 0.0, 0.0]),
        Err(ConservationValidationError::CflStabilityViolation { .. })
    ));

    let mode00 = evaluate_qac_fluid_mode00_oracle(1.225, 142_000.0, 2.0, [0.005, 0.0, 0.0], true)
        .unwrap();
    assert!(mode00.sound_speed_m_per_s > 300.0);
    assert_eq!(
        evaluate_qac_fluid_mode00_oracle(1.225, 142_000.0, 2.0, [0.005, 0.0, 0.0], false),
        Err(ConservationValidationError::NonCollocatedPressureVelocityRejected)
    );

    let stokes = jones_vector_to_stokes(Complex64::ONE, Complex64::I);
    let verified = verify_stokes_realizability(stokes).unwrap();
    assert!((verified.degree_of_polarization - 1.0).abs() <= 1e-12);
    assert!(matches!(
        verify_stokes_realizability([1.0, 1.0, 1.0, 0.0]),
        Err(PolarimetricValidationError::OverPolarizedState { .. })
    ));

    assert!(matches!(
        reconstruct_jpeg_from_jbrd(&[0u8; 32], &[], JxlLimits::default()),
        Err(JxlError::CorruptStream(_))
    ));
}
