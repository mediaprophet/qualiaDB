//! Cross-subsystem performance & real-time gate benchmark for EMF v4 & QAC v1 (`W3-08`).
//! Aligned with `evidence/R02/performance-benchmark-manifest.json` (`PERF-WL-01`..`03`, `REALTIME-WL-04`..`05`).

use std::time::Instant;

use qualia_core_db::audio::qac::profiles::{
    convert_sn3d_n3d, HoaNormalization as ProfileHoaNormalization,
};
use qualia_core_db::audio::qac::signal::stft::{
    transform_into as stft_transform_into, ProviderLimits as StftLimits,
};
use qualia_core_db::audio::qac::signal::writer::{
    PcmDocument, PcmQuantity, PcmSamples, Provenance, ProvenanceRole, SignalRelation,
    WriteBuffers, WriteOptions,
};
use qualia_core_db::audio::qac::signal::{
    admit as admit_qac_signal, Limits as QacSignalLimits, Rate, Timing,
};
use qualia_core_db::audio::qac::spatial_dsp::{
    admit_and_derive_unit_quaternion_f64, convolve_stereo_hrir_with_additional_delays_into,
    evaluate_hoa_basis_f32, gate_block_energies_bs1770, rotate_vector_active_f64,
    HoaNormalization as DspHoaNormalization, HoaOrder, QualityStatus,
};
use qualia_core_db::audio::qac::{QacStorageOptions, QacStorageView};
use qualia_core_db::bundle::field_range_planner::{FieldEntryRangePlan, FieldRangeError};
use qualia_core_db::bundle::{BundleReader, BundleWriter};
use qualia_core_db::q42::emf_profiles::{
    canonical_emf_mode, encode_field_samples, field_entry_slot, field_metadata_len,
    prepare_field_document, yee_cfl_max_dt_seconds, yee_check_cfl, yee_step_periodic,
    yee_total_energy_j, FieldChunkInput, FieldClock, FieldDocument, FieldFrame, FieldProvenance,
    FieldSample, FieldWriteLimits, Limits as EmfLimits, ProfileReader, YeeRecord, FIELD_PROFILE,
};
use qualia_core_db::q42::field_envelope::{
    Error as EnvelopeError, Family, Limits as EnvelopeLimits, StructuralView,
};
use qualia_core_db::q42::field_semantics::{Limits as SemanticLimits, NodeState, Scratch};
use qualia_core_db::q42::science_validation::passivity_conservation::{
    evaluate_qac_elastic_mode01_oracle, evaluate_qac_fluid_mode00_oracle,
    verify_yee_cfl_and_dispersion, ConservationValidationError,
};

fn build_test_emf_v4(carrier_hz: f64) -> Vec<u8> {
    let sample = FieldSample {
        electric_v_per_m: [1.25, -2.5, 0.5], magnetic_tesla: [1e-6, -2e-6, 3e-6],
        position_m: [10.0, 20.0, -5.0], tick: 0, carrier_hz: Some(carrier_hz),
        stored_poynting_w_per_m2: Some([0.1, 0.2, 0.3]), confidence: Some(0.95),
    };
    let mut payload = [0u8; 128];
    encode_field_samples(&[sample], &mut payload, EmfLimits::default()).unwrap();
    let chunks = [FieldChunkInput {
        stream_id: 1, start_tick: 0, time_count: 1, spatial_key: None, payload: &payload,
    }];
    let document = FieldDocument {
        clock: FieldClock {
            scale: 0, epoch_seconds: 1_700_000_000, epoch_nanoseconds: 0,
            ticks_per_second_num: 1000, ticks_per_second_den: 1,
        },
        frame: FieldFrame {
            origin_m: [0.0, 0.0, 0.0], rotation_xyzw: [0.0, 0.0, 0.0, 1.0], grid: None,
        },
        provenance: FieldProvenance {
            role: 1, algorithm: Some("emf-perf-gate"), algorithm_version: Some("1"),
            lineage: Some("perf-benchmark"),
        },
        temporal_step_ticks: Some(1), chunks: &chunks, align_payloads_to_4096: false,
    };
    let write_limits = FieldWriteLimits::default();
    let mut meta = vec![0u8; field_metadata_len(&document, write_limits).unwrap()];
    let mut entries = [field_entry_slot(); 1];
    let prepared = prepare_field_document(&document, &mut meta, &mut entries, write_limits).unwrap();
    let len = prepared.encoded_len().unwrap();
    let (mut staging, mut out) = (vec![0u8; len], vec![0u8; len]);
    let (mut m_nodes, mut e_nodes) = ([NodeState::default(); 16], [NodeState::default(); 4]);
    prepared
        .encode_into(&mut staging, &mut out, &mut Scratch { metadata: &mut m_nodes, entries: &mut e_nodes })
        .unwrap();
    out
}

fn build_test_qac_v1_pcm(native_pcm: &[f32]) -> Vec<u8> {
    let rate = Rate::new(48_000, 1).unwrap();
    let pcm_doc = PcmDocument {
        samples: PcmSamples::F32(native_pcm),
        timing: Timing { scale: 1, epoch_seconds: 1_700_000_000, epoch_nanoseconds: 0, ticks: rate, samples: rate, start_tick: 0 },
        stream_id: 1, channels: 1, channel_order: "mono", purpose: "perf-gate",
        quantity: PcmQuantity::Pressure, relation: SignalRelation::PhysicalBaseUnits,
        units_per_stored_sample: 1.0, stored_offset: 0.0,
        provenance: Provenance { role: ProvenanceRole::Measured, algorithm: None, algorithm_version: None, instrument_uri: Some("urn:gate:mic"), lineage: None },
        original: None,
    };
    let (mut q_meta, mut q_pay, mut q_file, mut q_out) = ([0u8; 4096], [0u8; 4096], [0u8; 8192], vec![0u8; 8192]);
    let (mut q_m, mut q_e) = ([NodeState::default(); 64], [NodeState::default(); 16]);
    let q_len = pcm_doc
        .encode_into(
            WriteBuffers { metadata: &mut q_meta, payload: &mut q_pay, file: &mut q_file },
            &mut Scratch { metadata: &mut q_m, entries: &mut q_e },
            &mut q_out,
            WriteOptions::default(),
        )
        .unwrap();
    q_out.truncate(q_len);
    q_out
}

#[test]
fn perf_wl_01_small_metadata_rich_admit_and_chunk_latency() {
    let emf_bytes = build_test_emf_v4(2.4e9);
    assert!(!emf_bytes.is_empty());

    let start = Instant::now();
    let view = StructuralView::open(&emf_bytes, Family::EmfV4Candidate, EnvelopeLimits::default()).unwrap();
    let chunk_payload = view.verify_chunk(1).unwrap();
    let open_chunk_us = start.elapsed().as_micros();

    assert!(open_chunk_us < 5_000_000, "latency {open_chunk_us} us exceeded upper bound");
    assert_eq!(view.entry_count(), 1);
    assert_eq!(chunk_payload.len(), 128);

    let mut m_nodes = [NodeState::default(); 16];
    let mut e_nodes = [NodeState::default(); 4];
    let scratch_bytes = std::mem::size_of_val(&m_nodes) + std::mem::size_of_val(&e_nodes);
    assert!(scratch_bytes <= 131_072 && scratch_bytes <= 4 * 1024 * 1024);

    let mut scratch = Scratch { metadata: &mut m_nodes, entries: &mut e_nodes };
    let mut reader = ProfileReader::open(&view, &mut scratch, EmfLimits::default(), 64 << 20).unwrap();
    let admitted = reader.chunk(1).unwrap();
    assert_eq!(admitted.profile(), FIELD_PROFILE);
    let sample = admitted.field(0).unwrap().unwrap();
    assert_eq!(sample.carrier_hz(), Some(2.4e9));
    assert_eq!(sample.electric_v_per_m(), [1.25, -2.5, 0.5]);

    let pcm = [0.125f32, -0.25, 0.5, -0.75];
    let qac_bytes = build_test_qac_v1_pcm(&pcm);
    let qac_view = QacStorageView::open(&qac_bytes, QacStorageOptions::default()).unwrap();
    let mut q_m = [NodeState::default(); 32];
    let mut q_e = q_m;
    assert!(std::mem::size_of_val(&q_m) + std::mem::size_of_val(&q_e) <= 131_072);

    let sig = admit_qac_signal(
        qac_view.structure(),
        1,
        &mut Scratch { metadata: &mut q_m, entries: &mut q_e },
        SemanticLimits::default(),
        QacSignalLimits::default(),
    )
    .unwrap();
    assert_eq!(sig.as_bytes().len(), pcm.len() * 4);
}

#[test]
fn perf_wl_02_medium_multistream_range_and_throughput() {
    let (emf_bytes, qac_bytes) = (build_test_emf_v4(5.8e9), build_test_qac_v1_pcm(&[0.5, -0.5, 0.25, -0.25]));
    let mut w = BundleWriter::new();
    w.add_file("field.emf", "emf", emf_bytes.clone(), None).unwrap();
    w.add_file("voice.qac", "qac", qac_bytes.clone(), None).unwrap();
    let hmc = w.build().unwrap();

    let reader = BundleReader::parse(&hmc).unwrap();
    let start = Instant::now();
    let emf_plan = FieldEntryRangePlan::from_prefix(reader.entry("field.emf").unwrap(), &emf_bytes, EnvelopeLimits::default()).unwrap();
    assert_eq!(emf_plan.family(), Family::EmfV4Candidate);
    assert_eq!(emf_plan.entry_count(), 1);

    let c1 = emf_plan.plan_chunk_range(1).unwrap();
    assert_eq!(canonical_emf_mode(c1.mode), Some(0x0100));
    assert_eq!(c1.data_length, 128);

    let payload = &hmc[c1.hmc_data_range.start as usize..c1.hmc_data_range.end as usize];
    let metadata = &hmc[c1.hmc_meta_range.start as usize..c1.hmc_meta_range.end as usize];

    let verified_emf = emf_plan.verify_arrived_chunk(&c1, payload, metadata).unwrap();
    let range_verify_us = start.elapsed().as_micros();
    assert!(range_verify_us < 5_000_000, "range verify {range_verify_us} us exceeded bound");
    assert!(!verified_emf.is_whole_volume_verified());
    assert_eq!(verified_emf.claim_whole_volume_receipt(), Err(FieldRangeError::PartialRangeCannotClaimWholeVolume));

    let t_start = Instant::now();
    for _ in 0..1000 {
        let v = emf_plan.verify_arrived_chunk(&c1, payload, metadata).unwrap();
        assert_eq!(v.payload().len(), 128);
    }
    let elapsed_us = t_start.elapsed().as_micros();
    assert!(elapsed_us < 5_000_000, "throughput loop {elapsed_us} us exceeded bound");

    let (mut m, mut e) = ([NodeState::default(); 64], [NodeState::default(); 16]);
    let chunk = verified_emf
        .inspect_emf_profile(&mut Scratch { metadata: &mut m, entries: &mut e }, EmfLimits::default(), 64 << 20)
        .unwrap();
    assert_eq!(chunk.profile(), FIELD_PROFILE);

    let qac_plan = FieldEntryRangePlan::from_prefix(reader.entry("voice.qac").unwrap(), &qac_bytes, EnvelopeLimits::default()).unwrap();
    let qc1 = qac_plan.plan_chunk_range(1).unwrap();
    let q_pay = &hmc[qc1.hmc_data_range.start as usize..qc1.hmc_data_range.end as usize];
    let q_meta = &hmc[qc1.hmc_meta_range.start as usize..qc1.hmc_meta_range.end as usize];
    let verified_qac = qac_plan.verify_arrived_chunk(&qc1, q_pay, q_meta).unwrap();

    let (mut qm, mut qe) = ([NodeState::default(); 64], [NodeState::default(); 16]);
    let sig = verified_qac
        .inspect_qac_signal(&mut Scratch { metadata: &mut qm, entries: &mut qe }, SemanticLimits::default(), QacSignalLimits::default())
        .unwrap();
    assert_eq!(sig.channels(), 1);
}

#[test]
fn perf_wl_03_max_envelope_boundary_and_over_limit_rejection() {
    let limits = EnvelopeLimits::default();
    assert_eq!(limits.file_bytes, 268_435_456);
    assert_eq!(limits.metadata_bytes, 4_194_304);
    assert_eq!(limits.entries, 4096);
    assert_eq!(limits.blocks, 4096);
    assert_eq!(limits.chunk_bytes, 16_777_216);
    assert_eq!(limits.aggregate_decoded_bytes, 268_435_456);

    let emf_bytes = build_test_emf_v4(1.0e9);
    assert!(emf_bytes.len() > 128);

    let mut over_limit = limits;
    over_limit.file_bytes = 100;
    let start = Instant::now();
    let err = StructuralView::open(&emf_bytes, Family::EmfV4Candidate, over_limit).unwrap_err();
    let reject_us = start.elapsed().as_micros();
    assert_eq!(err, EnvelopeError::Limit);
    assert!(reject_us < 500_000, "over-limit latency {reject_us} us exceeded bound");

    let mut over_meta = limits;
    over_meta.metadata_bytes = 10;
    assert_eq!(StructuralView::open(&emf_bytes, Family::EmfV4Candidate, over_meta).unwrap_err(), EnvelopeError::Limit);

    let mut over_entries = limits;
    over_entries.entries = 0;
    assert_eq!(StructuralView::open(&emf_bytes, Family::EmfV4Candidate, over_entries).unwrap_err(), EnvelopeError::Limit);

    assert_eq!(StructuralView::open(&emf_bytes, Family::QacV1Candidate, limits).unwrap_err(), EnvelopeError::Family);

    let mut corrupt_magic = emf_bytes.clone();
    corrupt_magic[0] = b'X';
    assert_eq!(StructuralView::open(&corrupt_magic, Family::EmfV4Candidate, limits).unwrap_err(), EnvelopeError::Family);
}

#[test]
fn realtime_wl_04_audio_dsp_48khz_realtime_callback() {
    const SAMPLE_RATE: u32 = 48000;
    const BLOCK_FRAMES: usize = 512;
    const FFT_SIZE: usize = 1024;
    const HOP_SIZE: usize = 512;
    const HRIR_TAPS: usize = 256;

    let mut hoa_coeffs = [0.0f32; 16];
    let (mut hrir_left, mut hrir_right) = ([0.0f32; HRIR_TAPS], [0.0f32; HRIR_TAPS]);
    hrir_left[0] = 1.0;
    hrir_right[2] = 0.5;

    let mut mono_in = [0.0f32; BLOCK_FRAMES];
    for (i, sample) in mono_in.iter_mut().enumerate() {
        let t = (i as f32) / (SAMPLE_RATE as f32);
        *sample = (core::f32::consts::TAU * 440.0 * t).sin() * 0.5;
    }

    let (mut out_l, mut out_r) = ([0.0f32; BLOCK_FRAMES + HRIR_TAPS], [0.0f32; BLOCK_FRAMES + HRIR_TAPS]);
    let (mut stft_mono, mut stft_fft) = ([0.0f32; FFT_SIZE], [0.0f32; FFT_SIZE * 2]);
    let (mut stft_window, mut stft_spectra) = ([0.0f32; FFT_SIZE], [0u8; FFT_SIZE * 8]);
    let mut pcm_le = [0u8; BLOCK_FRAMES * 2 * 4];
    for (i, chunk) in pcm_le.chunks_exact_mut(4).enumerate() {
        let s = if i < BLOCK_FRAMES { mono_in[i] } else { 0.0 };
        chunk.copy_from_slice(&s.to_le_bytes());
    }

    let total_scratch_bytes = std::mem::size_of_val(&hoa_coeffs)
        + std::mem::size_of_val(&hrir_left) + std::mem::size_of_val(&hrir_right)
        + std::mem::size_of_val(&mono_in) + std::mem::size_of_val(&out_l) + std::mem::size_of_val(&out_r)
        + std::mem::size_of_val(&stft_mono) + std::mem::size_of_val(&stft_fft)
        + std::mem::size_of_val(&stft_window) + std::mem::size_of_val(&stft_spectra) + std::mem::size_of_val(&pcm_le);
    assert!(total_scratch_bytes <= 262_144, "scratch {total_scratch_bytes} exceeded 256 KiB");

    let start = Instant::now();
    for _ in 0..10 {
        let count = evaluate_hoa_basis_f32(HoaOrder::Order3, DspHoaNormalization::Sn3d, [1.0, 0.0, 0.0], &mut hoa_coeffs).unwrap();
        assert_eq!(count, 16);
        assert!((hoa_coeffs[0] - 1.0).abs() <= 1e-6);

        let q_z90 = [0.0, 0.0, core::f64::consts::FRAC_1_SQRT_2, core::f64::consts::FRAC_1_SQRT_2];
        let (q_unit, n2) = admit_and_derive_unit_quaternion_f64(q_z90).unwrap();
        assert!((n2 - 1.0).abs() <= 1e-12);
        let v_rot = rotate_vector_active_f64(q_unit, [1.0, 0.0, 0.0]).unwrap();
        assert!(v_rot[0].abs() <= 1e-12 && (v_rot[1] - 1.0).abs() <= 1e-12);

        let (nl, nr) = convolve_stereo_hrir_with_additional_delays_into(&mono_in, &hrir_left, &hrir_right, [0, 0], &mut out_l, &mut out_r).unwrap();
        assert!(nl >= BLOCK_FRAMES && nr >= BLOCK_FRAMES);
        assert!(out_l[0].is_finite() && out_r[0].is_finite());

        let gated = gate_block_energies_bs1770(&[1.0e-2, 3.162e-4, 3.162e-8]).unwrap();
        assert_eq!(gated.status, QualityStatus::Valid);
        assert!(gated.integrated_lufs.is_some());

        let plan = stft_transform_into(
            &pcm_le, 1, FFT_SIZE, HOP_SIZE, &mut stft_mono, &mut stft_fft, &mut stft_window, &mut stft_spectra, StftLimits::default(),
        )
        .unwrap();
        assert_eq!(plan.frames(), 1);
        assert_eq!(plan.output_bytes(), FFT_SIZE * 8);
    }
    let cb_elapsed_us = start.elapsed().as_micros();
    assert!(cb_elapsed_us < 5_000_000, "audio DSP callback {cb_elapsed_us} us exceeded bound");

    let sn3d = [1.0f32, 0.5, -0.5, 0.25];
    let mut n3d = [0.0f32; 4];
    convert_sn3d_n3d(1, &sn3d, ProfileHoaNormalization::Sn3d, ProfileHoaNormalization::N3d, &mut n3d).unwrap();
    let mut back = [0.0f32; 4];
    convert_sn3d_n3d(1, &n3d, ProfileHoaNormalization::N3d, ProfileHoaNormalization::Sn3d, &mut back).unwrap();
    for (&orig, &rt) in sn3d.iter().zip(back.iter()) {
        assert!((orig - rt).abs() <= 1e-6);
    }
}

#[test]
fn realtime_wl_05_fdtd_stepping_and_conservation() {
    let spacing = [0.01, 0.01, 0.01];
    let dt_cfl = yee_cfl_max_dt_seconds(spacing, 1.0, 1.0).unwrap();
    let dt = 0.1 * dt_cfl;
    assert!(yee_check_cfl(dt, spacing, 1.0, 1.0).unwrap() < 0.500001);
    assert_eq!(yee_check_cfl(1.1 * dt_cfl, spacing, 1.0, 1.0), Err(qualia_core_db::q42::emf_profiles::Error::Time));

    let (cfl_max, omega_num) = verify_yee_cfl_and_dispersion(spacing, dt, 1.0, 1.0, [40.0, 0.0, 0.0]).unwrap();
    assert!((cfl_max - dt_cfl).abs() <= 1e-15);
    assert!(omega_num > 0.0);

    let dims = [4u32, 4, 4];
    let mut cells = vec![YeeRecord::vacuum([0.0; 3], [0.0; 3], 6); 64];
    for x in 0..4 {
        for y in 0..4 {
            for z in 0..4 {
                let idx = x + 4 * (y + 4 * z);
                let phase = core::f64::consts::TAU * (x as f64) / 4.0;
                cells[idx].electric_field_v_per_m = [0.0, 0.0, phase.sin()];
            }
        }
    }
    let e0 = yee_total_energy_j(&cells, spacing).unwrap();
    assert!(e0 > 0.0);

    let start = Instant::now();
    for _ in 0..20 {
        yee_step_periodic(&mut cells, dims, spacing, dt).unwrap();
    }
    let step_elapsed_us = start.elapsed().as_micros();
    assert!(step_elapsed_us < 5_000_000, "Yee stepping {step_elapsed_us} us exceeded bound");

    let e20 = yee_total_energy_j(&cells, spacing).unwrap();
    assert!(((e20 - e0) / e0).abs() < 0.08, "energy drift exceeded tolerance");
    assert_eq!(yee_step_periodic(&mut cells, dims, spacing, 1.1 * dt_cfl), Err(qualia_core_db::q42::emf_profiles::Error::Time));

    let mode00 = evaluate_qac_fluid_mode00_oracle(1.225, 142_000.0, 2.0, [0.005, 0.0, 0.0], true).unwrap();
    assert!(mode00.sound_speed_m_per_s > 300.0);
    assert!(mode00.instantaneous_energy_density_j_per_m3 > 0.0);
    assert_eq!(
        evaluate_qac_fluid_mode00_oracle(1.225, 142_000.0, 2.0, [0.005, 0.0, 0.0], false),
        Err(ConservationValidationError::NonCollocatedPressureVelocityRejected)
    );

    let mode01 = evaluate_qac_elastic_mode01_oracle(2700.0, 7.0e10, 2.6e10, [1e-5, -1e-5, 0.0, 0.0, 0.0, 0.0]).unwrap();
    assert!(mode01.compressional_speed_cp_m_per_s > mode01.shear_speed_cs_m_per_s);
    assert!(mode01.mechanical_pressure_pa.abs() <= 1e-5);

    for mode in 0x0100..=0x0107 {
        assert_eq!(canonical_emf_mode(mode), Some(mode));
    }
    for mode in 0..=7 {
        assert_eq!(canonical_emf_mode(mode), Some(0x0100 + mode));
    }
    assert_eq!(canonical_emf_mode(0x0108), None);
    assert_eq!(canonical_emf_mode(8), None);
}
