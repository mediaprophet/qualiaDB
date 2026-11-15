//! Wave-3 Integration Gate W3-07 (`GATE-SENSING-PORTAL-RANGE`):
//! End-to-end verification of `sensing::acquisition::storage` (Mode `0x0104` Schema 6/7 + 13 fixtures),
//! `sensing::authority` (M02 certificates, clock discipline, custody bridge, actuation gate),
//! `render::portal_load_emf` / `portal_emf_state` / `portal_qac` (EMF v4 & QAC v1 modes),
//! and `bundle::field_range_planner` (selective range planning, CRC-32C + BLAKE3, mixed Schema 6/7).

use core::f64::consts::TAU;
use ed25519_dalek::SigningKey;
use sha2::{Digest, Sha256};
use qualia_core_db::audio::qac::{
    metadata::{OriginBasis, WindowKind},
    profiles::{AdmittedEntryKind, AdmittedEnvelope, BASE_DELAY_REFERENCE, ClockSpec, ELASTIC_VALID_ALL, ElasticMediumSpec, ElasticStressVelocityRecord, FluidMediumModel, FrameSpec, HAPTIC_VALID_ALL, HapticContactSpec, HapticRecord, HapticRole, HoaDirectionConvention, HoaNormalization, HoaOrdering, HoaProfileSpec, HrirWriteSpec, InstantSpec, ModalRecord, PerceptualScale, PerceptualWriteSpec, ProfileError, ProfileLimits, ProvenanceSpec, RationalRate, RawUltrasoundPayload, SampleStatus, StaggeringOffsetsPV, US_VALID_AXIAL_VEL, US_VALID_HARMONIC, US_VALID_IQ, US_VALID_ISPTA, US_VALID_MI, US_VALID_PR, UltrasoundAcquisitionSpec, UltrasoundBeamSummaryRecord, UltrasoundRole, UnitDirection3, evaluate_actuator_command_at, find_ultrasound_summary_entry_index, plane_wave_record, read_elastic_stress_qac, read_haptic_qac, read_hoa_temporal_qac, read_hrir_qac, read_modal_bank_qac, read_perceptual_features_qac, read_pressure_velocity_qac, read_ultrasound_qac, write_elastic_stress_qac, write_haptic_qac, write_hoa_temporal_qac, write_hrir_qac, write_modal_bank_qac, write_perceptual_features_qac, write_pressure_velocity_qac, write_ultrasound_qac},
    signal::{ExactTime as QacExactTime, Limits as QacSignalLimits, QacMixedEntry, Rate as QacRate, Sample as QacSample, Timing as QacTiming, admit_qac_mixed_entry, admit_qac_signal_with_acquisition_source, stft::{StftDocument, StftQuantity, StftWriteBuffers, StftWriteOptions}, writer::{PcmDocument, PcmQuantity, PcmSamples, Provenance as QacProvenance, ProvenanceRole as QacProvenanceRole, SignalRelation, WriteBuffers, WriteOptions}},
};
use qualia_core_db::bundle::{BundleReader, BundleWriter, FieldEntryRangePlan, FieldRangeError, verify_arrived_field_chunk};
use qualia_core_db::container_10d::crc32c::crc32c;
use qualia_core_db::governance::consent_contract::{ConsentGrant, ConsentScope};
use qualia_core_db::q42::{
    emf_container::{EMF_BRICK_CELLS, EmfCellRecord64, EmfContainerWriter, emf_brick_voxel_linear_index},
    emf_profiles::{self as profiles, FieldChunkInput, FieldClock, FieldDocument, FieldFrame, FieldProvenance, FieldSample, FieldWriteLimits, Limits as EmfProfileLimits, encode_field_samples, field_entry_slot, field_metadata_len, prepare_field_document},
    field_envelope::{self as env, Family, Limits as EnvelopeLimits, StructuralView},
    field_semantics::{Limits as SharedSemanticLimits, NodeState, Scratch},
    field_spectrum::VACUUM_SPEED_OF_LIGHT_M_PER_S,
    m64_matter::brick3d::encode_hilbert3d_u64,
};
use qualia_core_db::render::{
    portal_emf_state::{DisplayEmfCell, DisplayOpticalCell, EmfBackend, EmfDisplayStatus, LoadedEmf, RetainedEmf, carrier_hz_to_spectral_sigma, e_magnitude_from_poynting, poynting_irradiance_w_per_m2},
    portal_load_emf::{Error as EmfLoadError, PortalEmfLimits, decode_emf_scientific, decode_emf_with_selection},
    portal_qac::{Limits as PortalQacLimits, PcmWindow, RetainedQac, StftWindow},
};
use qualia_core_db::sensing::{
    SemanticId,
    acquisition::{ArtifactLink, Axis, ByteOrder, Carrier, Clock, ComponentOrder, Context, Derivation, Encoding, Error as AcqError, Event as AcqEvent, EventOrdering, Layout, Loss, Normalization, Original, Quantity, Rate as AcqRate, SampleStage, Scalar, SignalAxes, TensorView, Timing as AcqTiming, storage::{ACQUISITION_SCHEMA, DERIVATION_SCHEMA, Document as AcqDocument, Error as AcqStorageError, Input as AcqInput, Limits as AcqLimits, ORIGINAL_MODE, View as AcqView, admit as admit_acquisition, custody as acq_custody, encode as encode_acquisition}},
    authority::{ActuationGateError, AuthorityReceiptTarget, CalibratedSeriesState, CalibrationCertificate, CalibrationError, CalibrationSchemaFamily, CalibrationValidityWindow, ClockDisciplineError, CommandReplayLedger, ConsentBoundSidecarAuthority, CrossClockMapping, CustodyBridgeError, EpistemicClass, ExpectedStreamBinding, HapticSchema207Role, IqCarrierConvention, LeapSecondStep, LeapSecondTable, PassiveArtifactKind, ProvenanceRole, ProvenanceRoleRecord, QualifiedIqSample, QualifiedUncertainty, RationalInstant, ReceiptReplayWindow, SignalStageRelation, SignedAuthorityReceipt, SignedSimulatorCommandReceipt, SimulatorCommandClaims, SimulatorCommandKind, SimulatorCommandRequest, UltrasoundSchema208Role, UncertaintyKind, admit_simulator_command, calibrate_complex_iq_pair_once, compute_phase_uncertainty_rad, convert_iq_sample_between_conventions, reject_passive_artifact_for_actuation, reject_unconjugated_iq_memcpy, verify_acquisition_derivation_custody, verify_acquisition_original_custody, verify_and_consume_stream_receipt, verify_calibration_certificate_chain, verify_empirical_join_admission, verify_phase_coherent_join, verify_same_clock_join},
    authorization::{AuthorizationError, IdentityAuthority, RevocationAuthority, SensingAction, SensingModality, SensingRequest, TrustedUnixClock, authorize_sensing},
    sidecar_attestation::{SIDECAR_BYTES, SidecarReceipt, SidecarReceiptAuthority},
    signed_scope::SignedSensingScope,
};

const TENSOR_QAC: &[u8] = include_bytes!("../src/sensing/acquisition/storage/fixtures/tensor.qac");
const TENSOR_EMF: &[u8] = include_bytes!("../src/sensing/acquisition/storage/fixtures/tensor.emf");
const EVENTS_QAC: &[u8] = include_bytes!("../src/sensing/acquisition/storage/fixtures/events.qac");
const EVENTS_EMF: &[u8] = include_bytes!("../src/sensing/acquisition/storage/fixtures/events.emf");
const CORR_QAC: &[u8] = include_bytes!("../src/sensing/acquisition/storage/fixtures/correlation.qac");
const CORR_EMF: &[u8] = include_bytes!("../src/sensing/acquisition/storage/fixtures/correlation.emf");
const LINEAGE_QAC: &[u8] = include_bytes!("../src/sensing/acquisition/storage/fixtures/lineage.qac");
const LINEAGE_EMF: &[u8] = include_bytes!("../src/sensing/acquisition/storage/fixtures/lineage.emf");
const UNKNOWN_CRIT_QAC: &[u8] = include_bytes!("../src/sensing/acquisition/storage/fixtures/unknown-critical.qac");
const UNKNOWN_OPT_QAC: &[u8] = include_bytes!("../src/sensing/acquisition/storage/fixtures/unknown-optional.qac");
const BAD_REF_QAC: &[u8] = include_bytes!("../src/sensing/acquisition/storage/fixtures/bad-ref.qac");
const BAD_HASH_QAC: &[u8] = include_bytes!("../src/sensing/acquisition/storage/fixtures/bad-hash.qac");
const OVERFLOW_STRIDE_QAC: &[u8] = include_bytes!("../src/sensing/acquisition/storage/fixtures/overflow-stride.qac");

fn sha256_hex(bytes: &[u8]) -> String { Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect() }

fn build_acquisition_bytes(family: Family) -> Vec<u8> {
    let (out_bytes, src_bytes) = ([0x10u8, 0x20, 0, 0, 0, 0, 0, 0, 0, 0x40], [1u8, 2, 3, 4, 5, 6, 7, 8]);
    let (original, src_orig) = (Original::admit(&out_bytes, 10).unwrap(), Original::admit(&src_bytes, 8).unwrap());
    let layout = Layout { rank: 1, extents: [1, 0, 0, 0], strides: [8, 0, 0, 0], axes: [Axis::Sample; 4], encoding: Encoding::Shared(Scalar::F64), byte_order: ByteOrder::Little, components: ComponentOrder::Scalar };
    let ctx = Context { quantity: Quantity { kind: SemanticId::new(11).unwrap(), unit: SemanticId::new(12).unwrap() }, stage: SampleStage::ReportedPhysical, sensor: SemanticId::new(13).unwrap(), geometry: None, calibration: None, uncertainty: None };
    let axes = SignalAxes { carrier: Carrier::Unknown, baseband_hz: None, pulse_rate: None, complex_convention: None, phase_origin: None };
    let tv = TensorView::admit(original.region(2, 8).unwrap(), layout, ctx, None, axes).unwrap();
    let (inputs, losses, link) = ([src_orig], [Loss::Quantization], ArtifactLink { digest: [0x55; 32], identifier: SemanticId::new(99).unwrap() });
    let deriv = Some(Derivation::admit(original, &inputs, link, link, &losses, 8).unwrap());
    let (mut meta, mut staging, mut out, mut m, mut e) = (vec![0u8; 16_384], vec![0u8; 65_536], vec![0u8; 65_536], [NodeState::default(); 256], [NodeState::default(); 128]);
    let n = encode_acquisition(AcqInput { family, view: AcqView::Tensor(tv), derivation: deriv }, &mut meta, &mut staging, &mut Scratch { metadata: &mut m, entries: &mut e }, &mut out, Default::default(), Default::default(), Default::default()).unwrap();
    out.truncate(n);
    out
}

#[test]
fn acquisition_storage_13_fixtures_schema6_schema7_and_custody_gate() {
    assert_eq!((ORIGINAL_MODE, ACQUISITION_SCHEMA, DERIVATION_SCHEMA), (0x0104, 6, 7));
    for (bytes, expected) in [
        (TENSOR_QAC, "3a4a54b1a4ca668fae9b2d76348d334575b995ea3cf5c2c6b7a92f6dc4bfa2db"), (TENSOR_EMF, "d18b466063e0e4527b82f77f78009034035cd1c78e1f4179434f3cd654d2f96f"),
        (EVENTS_QAC, "7a6d32f0ab279dfcbfa5edae130904443acd3870dd264199b42c96ec6a3fb3bc"), (EVENTS_EMF, "0859a5c424def50acf55cf1652f5a55e637a213b1bf09aaf10a306a13458a8d9"),
        (CORR_QAC, "3b7cd46a71eba54b62fe8b6cd1b0f9eef702b558679abadd7f0922143aed824f"), (CORR_EMF, "c02056cb99b1da5d5a82fc526a7c0c229895c9e53e1514db20e72a4f2c630423"),
        (LINEAGE_QAC, "c5420a9070d803c2995de570a1b07c101ce47fe44befe0f63d6a42ecffa002f0"), (LINEAGE_EMF, "405bd216b184478783a0cb60534f99bae5edbaf3706366ad701288b1775b7b81"),
        (UNKNOWN_CRIT_QAC, "8a85bbec594f0450f3ee0386e57864f60fb0a6c267937e7893400a483cd142c5"), (UNKNOWN_OPT_QAC, "149125527d3ac707d72fa379cf7b6cf4fcdf6887700ca66c4c174af67e146982"),
        (BAD_REF_QAC, "ad786798f632df546355513ecb47f7684df3aa5e6375e2d73c3da3fa688c085c"), (BAD_HASH_QAC, "1896f5ef1495ad66f063bc2daed74f863d4548749d996f7a259f037742e56c1d"),
        (OVERFLOW_STRIDE_QAC, "4073bab742609ed742ca006e6b9476fd36db181c7a231746a2052889088df484"),
    ] { assert_eq!(sha256_hex(bytes), expected); }
    let inspect = |bytes: &[u8], family: Family, check: fn(&AcqDocument<'_, '_>)| {
        let (structural, mut orig, mut m, mut e) = (StructuralView::open(bytes, family, EnvelopeLimits::default()).unwrap(), [None; 65], [NodeState::default(); 256], [NodeState::default(); 128]);
        check(&admit_acquisition(&structural, 1, &mut orig, &mut Scratch { metadata: &mut m, entries: &mut e }, SharedSemanticLimits::default(), AcqLimits::default()).unwrap());
    };
    let open_admit = |bytes: &[u8], family: Family, idx: u32| -> Result<(usize, u32, u32), AcqStorageError> {
        let (structural, mut orig, mut m, mut e) = (StructuralView::open(bytes, family, EnvelopeLimits::default())?, [None; 65], [NodeState::default(); 256], [NodeState::default(); 128]);
        let d = admit_acquisition(&structural, idx, &mut orig, &mut Scratch { metadata: &mut m, entries: &mut e }, SharedSemanticLimits::default(), AcqLimits::default())?;
        Ok((d.metadata_bytes.len(), d.uninterpreted_blocks, d.uninterpreted_fields))
    };
    for (family, bytes) in [(Family::QacV1Candidate, TENSOR_QAC), (Family::EmfV4Candidate, TENSOR_EMF)] {
        inspect(bytes, family, |d| { let AcqView::Tensor(tv) = d.view else { panic!() }; assert_eq!((tv.numeric([0; 4]).unwrap(), tv.numeric([1, 0, 0, 0]), d.uninterpreted_blocks, d.uninterpreted_fields), ([-0.0, 0.0], Err(AcqError::Nonfinite), 0, 0)); });
    }
    assert_eq!(open_admit(UNKNOWN_OPT_QAC, Family::QacV1Candidate, 1).unwrap().2, 1);
    for (family, bytes) in [(Family::QacV1Candidate, EVENTS_QAC), (Family::EmfV4Candidate, EVENTS_EMF)] {
        inspect(bytes, family, |d| { let AcqView::Events { view, axes } = d.view else { panic!() }; assert_eq!((view.records(), view.byte_order(), view.ordering(), view.event(0).unwrap(), view.event(1).unwrap(), axes.carrier), (2, ByteOrder::Big, EventOrdering::NondecreasingTick, AcqEvent { tick: -5, channel: 1, count: u64::MAX }, AcqEvent { tick: 12, channel: 0, count: 0 }, Carrier::Hertz(2e6))); });
    }
    for (family, bytes) in [(Family::QacV1Candidate, CORR_QAC), (Family::EmfV4Candidate, CORR_EMF)] {
        inspect(bytes, family, |d| { let AcqView::Correlation(cv) = d.view else { panic!() }; assert_eq!((cv.lag_ticks(0).unwrap(), cv.lag_ticks(1).unwrap(), cv.normalization(), cv.tensor().numeric([0, 0, 0, 0]).unwrap(), cv.tensor().numeric([1, 0, 0, 0]).unwrap()), (-1, 1, Normalization::Mean { contributing_pairs: 5 }, [1.0, -2.0], [3.5, 0.0])); });
    }
    for (family, bytes) in [(Family::QacV1Candidate, LINEAGE_QAC), (Family::EmfV4Candidate, LINEAGE_EMF)] {
        inspect(bytes, family, |d| { let lin = d.lineage.unwrap(); assert_eq!((lin.output_entry(), lin.inputs(), lin.input_entry(0).unwrap(), d.original(2).unwrap().bytes()), (1, 1, 2, &b"INPUT-original-bytes"[..])); });
        assert_eq!(open_admit(bytes, family, 2), Err(AcqStorageError::Unsupported));
    }
    for bad in [UNKNOWN_CRIT_QAC, BAD_REF_QAC, BAD_HASH_QAC, OVERFLOW_STRIDE_QAC] { assert!(open_admit(bad, Family::QacV1Candidate, 1).is_err()); }
    let fresh_qac = build_acquisition_bytes(Family::QacV1Candidate);
    inspect(&fresh_qac, Family::QacV1Candidate, |d| assert!(d.lineage.is_some()));
    let dir = std::env::temp_dir().join(format!("qualia-w307-custody-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let (mut m, mut e, mut rbuf, mut rorig) = ([NodeState::default(); 256], [NodeState::default(); 128], vec![0u8; 65_536], [None; 8]);
    let mut scratch = Scratch { metadata: &mut m, entries: &mut e };
    let published = acq_custody::publish_container(&dir, Family::QacV1Candidate, 1, &fresh_qac, &mut scratch, acq_custody::PublishOptions::default()).unwrap();
    let (n_read, art_read, doc_read) = acq_custody::read_container(&dir, Family::QacV1Candidate, &published.blake3, 1, &mut rbuf, &mut rorig, &mut scratch, acq_custody::PublishOptions::default()).unwrap();
    assert_eq!((n_read, art_read.blake3, doc_read.lineage.is_some()), (fresh_qac.len(), published.blake3, true));
    let mut tampered = fresh_qac.clone();
    tampered[0] ^= 0x01;
    std::fs::write(&published.path, &tampered).unwrap();
    let (mut rbuf2, mut rorig2) = (vec![0u8; 65_536], [None; 8]);
    assert!(matches!(acq_custody::read_container(&dir, Family::QacV1Candidate, &published.blake3, 1, &mut rbuf2, &mut rorig2, &mut scratch, acq_custody::PublishOptions::default()), Err(acq_custody::PublishError::Conflict)));
    let _ = std::fs::remove_dir_all(&dir);
}

struct FixedClock(u64);
impl TrustedUnixClock for FixedClock { fn now_unix(&mut self) -> Result<u64, AuthorizationError> { Ok(self.0) } }
struct TestIdentity([u8; 32], [u8; 32]);
impl IdentityAuthority for TestIdentity { fn verify_parties(&self, g: &ConsentGrant, actor: &[u8; 32]) -> Result<(), AuthorizationError> { (g.principal_did == self.0 && *actor == self.1).then_some(()).ok_or(AuthorizationError::IdentityUnverified) } }
struct FlagRevocation(bool);
impl RevocationAuthority for FlagRevocation { fn is_revoked(&self, _: &[u8; 32], _: &[u8; 32], _: u64) -> Result<bool, AuthorizationError> { Ok(self.0) } }
fn sample_clock(origin_id: u64, restart: u64, rate_hz: u64) -> Clock {
    let r = AcqRate::new(rate_hz, 1).unwrap();
    Clock::new(SemanticId::new(origin_id).unwrap(), restart, AcqTiming { scale: 0, epoch_seconds: 1_700_000_000, epoch_nanoseconds: 0, ticks: r, samples: r, start_tick: 0 }).unwrap()
}

#[test]
fn sensing_authority_m02_certificates_clocks_custody_and_actuation_gate() {
    let clk = sample_clock(11, 1, 48_000);
    let (r_from, r_until) = (RationalInstant::from_clock_tick_subtick(clk, 0, 0, 1).unwrap(), RationalInstant::from_clock_tick_subtick(clk, 10_000, 0, 1).unwrap());
    let (l_from, l_until) = (RationalInstant::from_clock_tick_subtick(clk, 1_000, 0, 1).unwrap(), RationalInstant::from_clock_tick_subtick(clk, 5_000, 0, 1).unwrap());
    let leaf_win = CalibrationValidityWindow::new(l_from, l_until).unwrap();
    assert_eq!((leaf_win.verify_contains(l_from), leaf_win.verify_contains(l_until)), (Ok(()), Err(CalibrationError::CalibrationExpiredAtOrAfterUntil)));
    let (root_k, lab_k, sens_k) = (SigningKey::from_bytes(&[1; 32]), SigningKey::from_bytes(&[2; 32]), SigningKey::from_bytes(&[3; 32]));
    let (q_in, q_out, u_out) = (SemanticId::new(1001).unwrap(), SemanticId::new(1002).unwrap(), SemanticId::new(2002).unwrap());
    let mk_cert = |iss: &SigningKey, sub: &SigningKey, parent: Option<[u8; 32]>, win, scale| CalibrationCertificate {
        schema_family: CalibrationSchemaFamily::EmfSchema107, certificate_id: [0x44; 32], parent_certificate_digest: parent, instrument_id: SemanticId::new(501).unwrap(), channel_index: 0, input_quantity: q_in, output_quantity: q_out, output_unit: u_out, affine_scale: scale, affine_offset: 0.0, reference_level: Some(1.0), frequency_band_hz: Some([1.0e3, 1.0e9]), validity_window: win, uncertainty: QualifiedUncertainty { value: 0.01, target_quantity: q_out, target_unit: u_out, kind: UncertaintyKind::ExpandedAbsolute { coverage_factor_k: 2.0, confidence_probability: 0.95 }, estimator_id: SemanticId::new(77).unwrap() }, issuer_public_key: [0; 32], subject_public_key: sub.verifying_key().to_bytes(), signature: [0; 64],
    }.sign(iss).unwrap();
    let root_cert = mk_cert(&root_k, &lab_k, None, CalibrationValidityWindow::new(r_from, r_until).unwrap(), 1.0);
    let leaf_cert = mk_cert(&lab_k, &sens_k, Some(root_cert.certificate_digest()), leaf_win, 2.0);
    let (obs, root_pk) = (RationalInstant::from_clock_tick_subtick(clk, 2_500, 0, 1).unwrap(), root_k.verifying_key().to_bytes());
    assert_eq!(verify_calibration_certificate_chain(&[root_cert, leaf_cert], &root_pk, obs, Some(10.0e3)).unwrap(), leaf_cert.certificate_digest());
    assert_eq!(verify_calibration_certificate_chain(&[root_cert, leaf_cert], &root_pk, l_until, Some(10.0e3)), Err(CalibrationError::CalibrationExpiredAtOrAfterUntil));
    let raw_s = CalibratedSeriesState::from_raw_counts(q_in, SemanticId::new(2001).unwrap(), [3.0, 4.0]).unwrap();
    let cal_s = raw_s.apply_calibration_once(&leaf_cert, obs, Some(10.0e3)).unwrap();
    assert_eq!((cal_s.samples(), cal_s.stage(), cal_s.apply_calibration_once(&leaf_cert, obs, Some(10.0e3))), (&[6.0, 8.0], SignalStageRelation::PhysicalBaseUnits, Err(CalibrationError::DoubleCalibrationRejected)));
    let (iq_cal, stg, _, _) = calibrate_complex_iq_pair_once(SignalStageRelation::RawCounts, (3.0, 4.0), q_in, &leaf_cert, obs, 10.0e3).unwrap();
    assert_eq!((iq_cal, calibrate_complex_iq_pair_once(stg, iq_cal, q_in, &leaf_cert, obs, 10.0e3)), ((6.0, 8.0), Err(CalibrationError::DoubleCalibrationRejected)));
    assert_eq!(leaf_cert.uncertainty.verify_against_mixed_record_components(&[(q_out, u_out), (q_in, u_out)]), Ok(()));
    let clk_b = sample_clock(12, 1, 48_000);
    assert_eq!(verify_same_clock_join(clk, clk_b), Err(ClockDisciplineError::CrossClockOriginRejected));
    let (t_a, t_b) = (RationalInstant::from_clock_tick_subtick(clk, 48_000, 0, 1).unwrap(), RationalInstant::from_clock_tick_subtick(clk_b, 48_000, 0, 1).unwrap());
    let map = CrossClockMapping { mapping_id: SemanticId::new(88).unwrap(), source_origin: clk_b.origin(), source_restart: 1, source_scale: 0, target_origin: clk.origin(), target_restart: 1, target_scale: 0, source_validity: CalibrationValidityWindow::new(RationalInstant::from_clock_tick_subtick(clk_b, 0, 0, 1).unwrap(), RationalInstant::from_clock_tick_subtick(clk_b, 96_000, 0, 1).unwrap()).unwrap(), offset_seconds_num: 0, offset_seconds_den: 1, mapping_time_uncertainty_seconds: 2.0e-9 };
    let (mapped_b, u_t) = map.map_instant(t_b, Some(3.0e-9)).unwrap();
    assert!((verify_phase_coherent_join(t_a, mapped_b, 2.0e6, Some(u_t), 0.10).unwrap() - TAU * 2.0e6 * 5.0e-9).abs() <= 1e-15);
    assert_eq!(compute_phase_uncertainty_rad(2.0e6, None), Err(ClockDisciplineError::UnknownOrInvalidTimeUncertainty));
    let leap = LeapSecondTable::new(SemanticId::new(300).unwrap(), &[LeapSecondStep { effective_utc_second: 1_483_228_800, tai_minus_utc_seconds: 37 }], 1_500_000_000, 1_800_000_000, 1.0e-9).unwrap();
    let (tai_t, _) = leap.convert_utc_instant_to_tai(RationalInstant::new(SemanticId::new(301).unwrap(), 1, 1, 1_700_000_000, 1).unwrap(), SemanticId::new(302).unwrap(), Some(2.0e-9)).unwrap();
    assert_eq!((tai_t.numerator(), reject_unconjugated_iq_memcpy(IqCarrierConvention::EmfNegativeCarrier, IqCarrierConvention::QacPositiveCarrier)), (1_700_000_037, Err(ClockDisciplineError::UnconjugatedCarrierMemcpyRejected)));
    let emf_q = QualifiedIqSample { i: 0.0, q: 1.0, convention: IqCarrierConvention::EmfNegativeCarrier, carrier_hz: 10_000.0, reference_phase_rad: 0.0, signed_baseband_offset_hz: 1_000.0 };
    let qac_q = convert_iq_sample_between_conventions(emf_q, IqCarrierConvention::QacPositiveCarrier, 0.0).unwrap();
    assert_eq!((qac_q.i, qac_q.q, qac_q.signed_baseband_offset_hz), (0.0, -1.0, -1_000.0));

    let (prin_k, act_k, sig_k) = (SigningKey::from_bytes(&[10; 32]), SigningKey::from_bytes(&[20; 32]), SigningKey::from_bytes(&[30; 32]));
    let (prin_did, act_did, now) = (prin_k.verifying_key().to_bytes(), act_k.verifying_key().to_bytes(), 1_700_000_100);
    let grant = ConsentGrant::new_signed([0xAA; 32], &prin_k, act_did, ConsentScope(ConsentScope::VITALS), 9001, now - 100, now + 3600, 42);
    let req_acq = SensingRequest::new(act_did, 7001, SensingModality::Sdr, SensingAction::Acquire, 9001).unwrap();
    let scope_acq = SignedSensingScope::sign(&grant, &prin_k, req_acq).unwrap();
    let (id_auth, mut uclk, rev_ok) = (TestIdentity(prin_did, act_did), FixedClock(now), FlagRevocation(false));
    let reval = authorize_sensing(&grant, req_acq, &id_auth, &scope_acq, &rev_ok, &mut uclk).unwrap().revalidate(&grant, &id_auth, &scope_acq, &rev_ok, &mut uclk).unwrap();
    let (orig, idx_comm) = (Original::admit(b"mode-0x0104-payload", 1024).unwrap(), [0xCC; 32]);
    let rcpt = SignedAuthorityReceipt::issue(&reval, grant.expires_at, AuthorityReceiptTarget::ProvenanceSchema4, orig.digest(), idx_comm, 1, &sig_k).unwrap();
    let mut replay_win = ReceiptReplayWindow::new();
    verify_acquisition_original_custody(orig, idx_comm, rcpt.receipt_digest(), AuthorityReceiptTarget::ProvenanceSchema4, &rcpt, req_acq, &grant, &id_auth, &scope_acq, &rev_ok, &mut uclk, &mut replay_win).unwrap();
    assert_eq!(verify_acquisition_original_custody(orig, idx_comm, rcpt.receipt_digest(), AuthorityReceiptTarget::ProvenanceSchema4, &rcpt, req_acq, &grant, &id_auth, &scope_acq, &rev_ok, &mut uclk, &mut replay_win), Err(CustodyBridgeError::ReplayDetected));
    let req_exp = SensingRequest::new(act_did, 7001, SensingModality::Sdr, SensingAction::Export, 9001).unwrap();
    let mut fresh_win = ReceiptReplayWindow::new();
    assert_eq!(verify_and_consume_stream_receipt(&rcpt, &ExpectedStreamBinding { expected_target: AuthorityReceiptTarget::ProvenanceSchema4, stored_receipt_digest: rcpt.receipt_digest(), original_blake3: orig.digest(), index_commitment: idx_comm, requested_operation: req_exp }, &grant, &id_auth, &scope_acq, &rev_ok, &mut uclk, &mut fresh_win), Err(CustodyBridgeError::ActionEscalation { granted: SensingAction::Acquire, requested: SensingAction::Export }));

    let req_proc = SensingRequest::new(act_did, 7001, SensingModality::Sdr, SensingAction::Process, 9001).unwrap();
    let scope_proc = SignedSensingScope::sign(&grant, &prin_k, req_proc).unwrap();
    let reval_p = authorize_sensing(&grant, req_proc, &id_auth, &scope_proc, &rev_ok, &mut uclk).unwrap().revalidate(&grant, &id_auth, &scope_proc, &rev_ok, &mut uclk).unwrap();
    let (deriv_out, inputs, losses, link) = (Original::admit(b"derived-bytes", 1024).unwrap(), [orig], [Loss::NoneDeclared], ArtifactLink { digest: [0x11; 32], identifier: SemanticId::new(1).unwrap() });
    let deriv = Derivation::admit(deriv_out, &inputs, link, link, &losses, 8).unwrap();
    let d_rcpt = SignedAuthorityReceipt::issue(&reval_p, grant.expires_at, AuthorityReceiptTarget::AcquisitionDerivationSchema7, deriv_out.digest(), idx_comm, 2, &sig_k).unwrap();
    verify_acquisition_derivation_custody(deriv, idx_comm, d_rcpt.receipt_digest(), &d_rcpt, req_proc, &grant, &id_auth, &scope_proc, &rev_ok, &mut uclk, &mut fresh_win).unwrap();
    let (sc_auth, sc_bytes) = (ConsentBoundSidecarAuthority::new(&d_rcpt, [0x77; 32]).unwrap(), [0u8; SIDECAR_BYTES]);
    let sc_rcpt = SidecarReceipt::sign(&sc_bytes, b"mode-0x0104-payload", 1024, d_rcpt.receipt_digest(), [0x77; 32], 0, None, &sig_k).unwrap();
    assert!(sc_auth.permits(&sc_rcpt, &sc_bytes, b"mode-0x0104-payload"));
    assert_eq!(verify_empirical_join_admission(ProvenanceRole::Measured, EpistemicClass::Empirical, ProvenanceRole::Derived, EpistemicClass::Empirical), Ok(()));
    assert_eq!(verify_empirical_join_admission(ProvenanceRole::Measured, EpistemicClass::Empirical, ProvenanceRole::Measured, EpistemicClass::Hypothesis), Err(ActuationGateError::NonEmpiricalClassRejected(EpistemicClass::Hypothesis)));
    assert_eq!(reject_passive_artifact_for_actuation(PassiveArtifactKind::IonizingDoseGyOrSv), Err(ActuationGateError::PassiveObservationCannotAuthorizeActuation(PassiveArtifactKind::IonizingDoseGyOrSv)));
    assert_eq!((SimulatorCommandKind::from_haptic_role(HapticSchema207Role::Measurement), SimulatorCommandKind::from_ultrasound_role(UltrasoundSchema208Role::TransmitCommand)), (Err(ActuationGateError::NonCommandDomainRoleRejected), Ok(SimulatorCommandKind::UltrasoundTransmitCommandSchema208)));
    assert_eq!(ProvenanceRoleRecord { role: ProvenanceRole::Measured, source_digest: None, algorithm_id: SemanticId::Unknown, algorithm_version: SemanticId::Unknown, instrument_uri: SemanticId::new(10).unwrap(), authority_receipt_digest: None }.validate(), Ok(()));
    let cmd_claims = SimulatorCommandClaims { command_id: [0xC1; 32], command_nonce: 1, command_kind: SimulatorCommandKind::HapticActuatorCommandSchema207, provenance_role: ProvenanceRole::Command, grant_id: grant.grant_id, grant_digest: grant.compute_digest(), actor_id: act_did, source_id: 5001, device_profile_digest: [0xE1; 32], contact_or_array_frame_id: 7, validity_window: leaf_win };
    let (cmd_rcpt, cmd_pk, mut cmd_ledger) = (SignedSimulatorCommandReceipt::sign(cmd_claims, &sig_k).unwrap(), sig_k.verifying_key().to_bytes(), CommandReplayLedger::new());
    let cmd_req = SimulatorCommandRequest { requested_kind: SimulatorCommandKind::HapticActuatorCommandSchema207, descriptor_provenance_role: ProvenanceRole::Command, stored_authority_receipt_digest: cmd_rcpt.receipt_digest(), device_profile_digest: [0xE1; 32], contact_or_array_frame_id: 7, actor_id: act_did, source_id: 5001, execution_instant: obs };
    assert_eq!(admit_simulator_command(&SimulatorCommandRequest { execution_instant: l_until, ..cmd_req }, &cmd_rcpt, &cmd_pk, &grant, &rev_ok, &mut uclk, &mut cmd_ledger), Err(ActuationGateError::CommandWindow(CalibrationError::CalibrationExpiredAtOrAfterUntil)));
    assert_eq!(admit_simulator_command(&cmd_req, &cmd_rcpt, &cmd_pk, &grant, &rev_ok, &mut uclk, &mut cmd_ledger).unwrap().contact_or_array_frame_id(), 7);
    assert_eq!(admit_simulator_command(&cmd_req, &cmd_rcpt, &cmd_pk, &grant, &rev_ok, &mut uclk, &mut cmd_ledger), Err(ActuationGateError::CommandReplayDetected));
}

#[derive(Clone)]
struct Tf { key: u16, tag: u8, count: u32, bytes: Vec<u8> }
struct Tb { id: u32, schema: u16, fields: Vec<Tf> }
fn tf(key: u16, tag: u8, bytes: Vec<u8>) -> Tf { let w = match tag { 1 | 8 | 9 => 4, 2 | 3 | 5 => 8, _ => 1 }; Tf { key, tag, count: (bytes.len() / w) as u32, bytes } }
fn tu32(k: u16, v: u32) -> Tf { tf(k, 1, v.to_le_bytes().to_vec()) }
fn tref(k: u16, v: u32) -> Tf { tf(k, 8, v.to_le_bytes().to_vec()) }
fn tchunk(k: u16, v: u32) -> Tf { tf(k, 9, v.to_le_bytes().to_vec()) }
fn tu64(k: u16, v: u64) -> Tf { tf(k, 2, v.to_le_bytes().to_vec()) }
fn ti64(k: u16, v: i64) -> Tf { tf(k, 3, v.to_le_bytes().to_vec()) }
fn tf64(k: u16, v: f64) -> Tf { tf(k, 5, v.to_le_bytes().to_vec()) }
fn tstr(k: u16, s: &str) -> Tf { tf(k, 6, s.as_bytes().to_vec()) }
fn tf64s(k: u16, xs: &[f64]) -> Tf { tf(k, 5, xs.iter().flat_map(|v| v.to_le_bytes()).collect()) }
fn tu32s(k: u16, xs: &[u32]) -> Tf { tf(k, 1, xs.iter().flat_map(|v| v.to_le_bytes()).collect()) }
fn tu64s(k: u16, xs: &[u64]) -> Tf { tf(k, 2, xs.iter().flat_map(|v| v.to_le_bytes()).collect()) }
fn ti64s(k: u16, xs: &[i64]) -> Tf { tf(k, 3, xs.iter().flat_map(|v| v.to_le_bytes()).collect()) }
fn put_f64(b: &mut [u8], kv: &[(usize, f64)]) { for &(o, v) in kv { b[o..o + 8].copy_from_slice(&v.to_le_bytes()); } }
fn put_u32(b: &mut [u8], kv: &[(usize, u32)]) { for &(o, v) in kv { b[o..o + 4].copy_from_slice(&v.to_le_bytes()); } }

fn seal_env(family: Family, mut blocks: Vec<Tb>, mut entries: Vec<env::EntryInput<'_>>) -> Vec<u8> {
    blocks.sort_by_key(|b| b.id);
    for b in &mut blocks { b.fields.sort_by_key(|f| f.key); }
    let f_in: Vec<Vec<_>> = blocks.iter().map(|b| b.fields.iter().map(|f| env::FieldInput { key: f.key, flags: 1, value: env::FieldValue::Raw { type_tag: f.tag, element_count: f.count, bytes: &f.bytes } }).collect()).collect();
    let m_blocks: Vec<_> = blocks.iter().zip(&f_in).map(|(b, f)| env::BlockInput { id: b.id, schema_type: b.schema, revision: 1, flags: 1, fields: f }).collect();
    let meta_in = env::MetadataInput { blocks: &m_blocks };
    let mut meta = vec![0; meta_in.encoded_len(EnvelopeLimits::default()).unwrap()];
    meta_in.encode_into(&mut meta, EnvelopeLimits::default()).unwrap();
    entries.sort_by_key(|e| (e.stream_id, e.start_tick, e.spatial_key, e.mode, e.descriptor_id));
    let env_in = env::EnvelopeInput { family, flags: 0, metadata: &meta, entries: &entries };
    let mut out = vec![0; env_in.encoded_len(EnvelopeLimits::default()).unwrap()];
    env_in.encode_into(&mut out, EnvelopeLimits::default()).unwrap();
    out
}
fn seal_emf(blocks: Vec<Tb>, entries: Vec<env::EntryInput<'_>>) -> Vec<u8> { seal_env(Family::EmfV4Candidate, blocks, entries) }
fn base_emf_blocks(grid: Option<([f64; 3], [u32; 3])>) -> Vec<Tb> {
    let mut ff = vec![tu32(1, 0), tf64s(3, &[0.0; 3]), tf64s(4, &[0.0, 0.0, 0.0, 1.0]), tstr(8, "right-handed-X-forward-Y-left-Z-up")];
    if let Some((sp, d)) = grid { ff.extend([tf64s(5, &sp), tu32s(6, &d), tu32(7, 0)]); }
    vec![Tb { id: 1, schema: 1, fields: vec![tu32(1, 0), ti64(2, 0), tu32(3, 0), tu64(4, 1000), tu64(5, 1)] }, Tb { id: 2, schema: 2, fields: ff }, Tb { id: 3, schema: 4, fields: vec![tu32(1, 1)] }]
}
fn ext_blk(id: u32, mode: u32, prof: &str, dims: [u64; 3], extra: Vec<Tf>) -> Tb {
    let mut f = vec![tu32(1, mode), tu32(2, 1), tref(3, 1), tref(4, 2), tref(5, 3), tstr(6, prof), tstr(7, "linear"), tu64s(8, &dims), tu32s(9, &[dims[0] as u32, dims[1] as u32, dims[2] as u32]), tu32(10, 0), ti64(11, 0)];
    f.extend(extra);
    Tb { id, schema: 103, fields: f }
}
fn desc_blk(id: u32, qty: &str, unit: &str, ext: Option<u32>, mask: Option<(u32, u64)>) -> Tb {
    let mut f = vec![tref(2, 2), tref(3, 3), tstr(7, qty), tstr(8, unit), tstr(12, "qualia://emf/v4")];
    if let Some((m_idx, vn)) = mask { f.extend([tu64(15, vn), tchunk(16, m_idx)]); }
    if let Some(eid) = ext { f.push(tref(18, eid)); }
    Tb { id, schema: 3, fields: f }
}
fn entry_u(mode: u16, desc: u32, rc: u64, stride: u32, payload: &[u8]) -> env::EntryInput<'_> {
    env::EntryInput { mode, revision: 1, flags: 0, stream_id: 0, frame_id: 2, start_tick: 0, time_count: 0, spatial_key: 0, record_count: rc, stride, codec: 0, descriptor_id: desc, payload }
}

#[test]
fn render_portal_emf_v4_modes_and_portal_qac_v1_modes_gate() {
    let mut b0 = base_emf_blocks(None);
    b0.extend([ext_blk(4, 0x0100, profiles::FIELD_PROFILE, [1, 1, 1], vec![]), desc_blk(5, "electric_field", "V/m", Some(4), None)]);
    let mut p0 = [0u8; 128];
    put_f64(&mut p0, &[(0, 3.0), (8, 4.0), (80, VACUUM_SPEED_OF_LIGHT_M_PER_S / 532.0e-9)]);
    put_u32(&mut p0, &[(120, 1)]);
    let emf0 = seal_emf(b0, vec![entry_u(0x0100, 5, 1, 128, &p0)]);
    let l0 = decode_emf_scientific(&emf0).unwrap();
    assert_eq!((l0.display_mode(), l0.display_cells()[0].magnitude_v_per_m), (0x0100, 5.0));
    for legacy_ver in 1u16..=3 {
        let mut legacy = emf0.clone();
        legacy[8..10].copy_from_slice(&legacy_ver.to_le_bytes());
        assert_eq!(decode_emf_with_selection(&legacy, None, PortalEmfLimits::default()).err(), Some(EmfLoadError::Storage(env::Error::Version)));
    }
    let mut b1 = base_emf_blocks(None);
    b1.extend([Tb { id: 6, schema: 135, fields: vec![tstr(1, "deterministic_transverse_v1"), tstr(2, "I_Q_U_V_v1"), tstr(3, "W sr^-1 m^-2"), tstr(4, "time_averaged")] }, ext_blk(4, 0x0101, profiles::RAY_PROFILE, [1, 1, 1], vec![tref(16, 6)]), desc_blk(5, "radiance", "W sr^-1 m^-2", Some(4), None)]);
    let mut p1 = [0u8; 128];
    put_f64(&mut p1, &[(24, 1.0), (48, 10.0), (56, 6.0), (80, 5.0e14), (96, 2.0)]);
    assert_eq!(decode_emf_scientific(&seal_emf(b1, vec![entry_u(0x0101, 5, 1, 128, &p1)])).unwrap().display_mode(), 0x0101);
    let mut b2 = base_emf_blocks(Some(([1.0, 1.0, 1.0], [1, 1, 1])));
    b2.extend([Tb { id: 6, schema: 133, fields: vec![tstr(1, "isotropic_nondispersive"), tf64(2, 2.25), tf64(3, 1.0), tf64(4, 0.0)] }, ext_blk(4, 0x0102, "yee_cartesian_v1", [1, 1, 1], vec![tref(14, 6)]), desc_blk(5, "electric_field", "V/m", Some(4), None)]);
    let mut p2 = [0u8; 128];
    put_f64(&mut p2, &[(0, 3.0), (32, 4.0), (88, 2.25), (96, 1.0)]);
    let mut e2 = entry_u(0x0102, 5, 1, 128, &p2);
    e2.flags = 4;
    let l2 = decode_emf_scientific(&seal_emf(b2, vec![e2])).unwrap();
    assert!((l2.optical_cells()[0].stokes_i - 12.0).abs() < 1e-9 && poynting_irradiance_w_per_m2([1.0, 0.0, 0.0], None, None) > 0.0);
    let mut b3 = base_emf_blocks(None);
    b3.extend([Tb { id: 7, schema: 132, fields: vec![tstr(1, "frequency_hz"), tstr(2, "bin_integral"), tstr(3, "bin_integrated_irradiance"), tstr(4, "W m^-2"), tf64s(5, &[8.0e8, 1.25e9]), tstr(6, "explicit"), tstr(8, "none")] }, ext_blk(4, 0x0103, profiles::SPECTRUM_PROFILE, [1, 1, 1], vec![tref(12, 7), tu32(19, 1)]), desc_blk(5, "bin_integrated_irradiance", "W m^-2", Some(4), None)]);
    let mut p3 = [0u8; 128];
    put_u32(&mut p3, &[(36, 7)]);
    put_f64(&mut p3, &[(64, 2.5)]);
    let l3 = decode_emf_scientific(&seal_emf(b3, vec![entry_u(0x0103, 5, 1, 128, &p3)])).unwrap();
    assert!((l3.display_cells()[0].magnitude_v_per_m - e_magnitude_from_poynting(2.5)).abs() < 1e-9);
    let mut b4 = base_emf_blocks(None);
    b4.extend([Tb { id: 6, schema: 135, fields: vec![tstr(1, "deterministic_transverse_v1"), tstr(2, "I_Q_U_V_v1"), tstr(3, "sr^-1"), tstr(4, "time_averaged")] }, Tb { id: 7, schema: 134, fields: vec![tstr(1, "e02-v1"), tstr(2, "local"), tf(3, 7, vec![0xAA; 32]), tstr(4, "si-right-handed"), tstr(8, "empirical")] }, ext_blk(4, 0x0104, "mueller_scattering_v1", [1, 1, 1], vec![tref(16, 6), tref(17, 7)]), desc_blk(5, "mueller_matrix", "sr^-1", Some(4), None), desc_blk(8, "calibration_metadata", "1", None, None)]);
    let mut p4 = [0u8; 256];
    put_f64(&mut p4, &[(24, 1.0), (48, 1.0), (72, VACUUM_SPEED_OF_LIGHT_M_PER_S / 200.0e-9), (128, 10.0)]);
    assert_eq!(decode_emf_scientific(&seal_emf(b4, vec![entry_u(0x0104, 5, 1, 256, &p4), entry_u(0x0104, 8, 1, 64, &[0x42; 64])])).unwrap().display_mode(), 0x0104);
    let mut b5 = base_emf_blocks(None);
    b5.extend([Tb { id: 6, schema: 107, fields: vec![tstr(1, "s"), tf(2, 7, vec![0xBB; 32]), tstr(3, "V"), tf64(4, 2.0), tf64(5, 0.5)] }, Tb { id: 7, schema: 136, fields: vec![tref(1, 1), tu64(2, 1000), tu64(3, 1), tstr(4, "complex_baseband"), tstr(5, "V"), tref(6, 6), tstr(7, "I_plus_iQ_exp_minus_iwt")] }, ext_blk(4, 0x0105, "calibrated_iq_stft_v1", [1, 1, 1], vec![tref(15, 7), tref(30, 6)]), desc_blk(5, "complex_baseband_voltage", "V", Some(4), None)]);
    let mut p5 = vec![0u8; 128];
    put_u32(&mut p5, &[(8, 1), (12, 1), (48, 1)]);
    put_f64(&mut p5, &[(16, 1.0e8), (32, 1.0), (64, 3.0), (72, 4.0)]);
    let crc5 = crc32c(&p5);
    put_u32(&mut p5, &[(60, crc5)]);
    assert_eq!(decode_emf_scientific(&seal_emf(b5, vec![entry_u(0x0105, 5, 1, 128, &p5)])).unwrap().display_mode(), 0x0105);
    let mut p6 = [0u8; 128];
    put_f64(&mut p6, &[(32, 60.0e3 * 1.602_176_634e-19), (56, 1.0e15)]);
    put_u32(&mut p6, &[(108, 0b1001)]);
    let crc6 = crc32c(&p6[..124]);
    put_u32(&mut p6, &[(124, crc6)]);
    for attach_mask in [true, false] {
        let mut b6 = base_emf_blocks(None);
        b6.extend([Tb { id: 6, schema: 134, fields: vec![tstr(1, "e02"), tstr(2, "loc"), tf(3, 7, vec![0xAA; 32]), tstr(4, "si-right-handed"), tstr(8, "empirical"), tstr(9, "photon"), ti64s(10, &[0, 100]), tref(12, 1)] }, Tb { id: 7, schema: 107, fields: vec![tstr(1, "s"), tf(2, 7, vec![0xBB; 32]), tstr(3, "J"), tf64(4, 1.0), tf64(5, 0.0)] }, ext_blk(4, 6, "ionizing_radiation_v1", [1, 1, 1], vec![tref(17, 6), tref(30, 7)]), ext_blk(10, 0x0106, profiles::VALIDITY_PROFILE, [1, 1, 1], vec![]), Tb { id: 11, schema: 3, fields: vec![tref(3, 3), tstr(7, "validity"), tstr(8, "1"), tstr(12, profiles::VALIDITY_MODEL_URI), tref(18, 10)] }, desc_blk(5, "spectral_fluence", "J", Some(4), attach_mask.then_some((2, 1)))]);
        let mut me = entry_u(0x0106, 11, 1, 1, &[1u8]);
        me.frame_id = 0;
        let res = decode_emf_with_selection(&seal_emf(b6, vec![entry_u(6, 5, 1, 128, &p6), me]), None, PortalEmfLimits::default());
        assert_eq!((res.is_ok(), !attach_mask && res.err() == Some(EmfLoadError::OrphanValidityMask)), (attach_mask, !attach_mask));
    }
    let mut b7 = base_emf_blocks(None);
    b7.extend([Tb { id: 6, schema: 134, fields: vec![tstr(1, "e02"), tstr(2, "loc"), tf(3, 7, vec![0xAA; 32]), tstr(4, "si-right-handed"), tstr(8, "empirical")] }, Tb { id: 7, schema: 107, fields: vec![tstr(1, "s"), tf(2, 7, vec![0xBB; 32]), tstr(3, "V"), tf64(4, 1.0), tf64(5, 0.0)] }, Tb { id: 8, schema: 137, fields: vec![tstr(1, "FMA-5"), tstr(2, "FMA_7088"), tstr(3, "current_dipole"), tref(4, 3)] }, ext_blk(4, 0x0107, "bioelectric_dipole_v1", [1, 1, 1], vec![tref(17, 6), tref(22, 8), tref(30, 7)]), desc_blk(5, "bioelectric_potential", "V", Some(4), None)]);
    let mut p7 = [0u8; 128];
    put_f64(&mut p7, &[(40, 1.0e-9), (48, 2.5e-3), (64, 40.0)]);
    put_u32(&mut p7, &[(96, 0b11)]);
    let crc7 = crc32c(&p7[..124]);
    put_u32(&mut p7, &[(124, crc7)]);
    assert_eq!(decode_emf_scientific(&seal_emf(b7, vec![entry_u(0x0107, 5, 1, 128, &p7)])).unwrap().display_mode(), 0x0107);
    let mut w64 = EmfContainerWriter::new([0.0; 3], 1.0, [1, 1, 1], 0.0);
    let mut cells = [EmfCellRecord64::default(); EMF_BRICK_CELLS];
    cells[emf_brick_voxel_linear_index(1, 2, 3)] = EmfCellRecord64 { e_field: [3.0, 4.0, 0.0], b_field: [0.0, 0.0, 1.0e-6], wavelength_nm: 550.0, stokes_q: 0.5, coherence_phase: 0.8, confidence_alpha: 1.0, ..Default::default() };
    w64.add_brick_chunk(encode_hilbert3d_u64(0, 0, 0, 0), &cells, 0, 1, 0).unwrap();
    let loaded64 = LoadedEmf::from_emf64(&w64.to_bytes().unwrap()).unwrap();
    let mut retained_emf = RetainedEmf::default();
    retained_emf.accept(loaded64, EmfBackend::WebGpu, |_| Ok(())).unwrap();
    assert_eq!((retained_emf.status(), carrier_hz_to_spectral_sigma(0.0), DisplayEmfCell::default().valid, DisplayOpticalCell::default().valid), (EmfDisplayStatus::Uploaded, 0.0, false, false));

    let pcm_vals = [0.5f32, -0.5, 0.25, -0.25, 0.75, -0.75];
    let stft_doc = StftDocument {
        samples: PcmSamples::F32(&pcm_vals), timing: QacTiming { scale: 2, epoch_seconds: 10, epoch_nanoseconds: 0, ticks: QacRate::new(96_000, 1).unwrap(), samples: QacRate::new(48_000, 1).unwrap(), start_tick: 0 },
        stream_id: 3, channels: 1, channel_order: "mic-A", fft_size: 4, hop: 2, pcm_purpose: "original-acquisition", stft_purpose: "hann-complex-stft",
        quantity: StftQuantity::DigitalSample, relation: SignalRelation::DigitalUnrelatedToPressure,
        pcm_provenance: QacProvenance { role: QacProvenanceRole::Measured, algorithm: None, algorithm_version: None, instrument_uri: Some("urn:inst:1"), lineage: None },
        stft_provenance: QacProvenance { role: QacProvenanceRole::Derived, algorithm: Some("urn:etalon:signal:hann-stft-f32"), algorithm_version: Some("1"), instrument_uri: None, lineage: Some("hann") },
        original: None,
    };
    let (mut mono, mut fft, mut win, mut spec, mut meta, mut pay, mut file, mut out) = ([0.0f32; 16], [0.0f32; 32], [0.0f32; 16], [0u8; 4096], [0u8; 8192], [0u8; 4096], [0u8; 16384], vec![0u8; 16384]);
    let (mut m, mut e) = ([NodeState::default(); 32], [NodeState::default(); 32]);
    let n = stft_doc.encode_into(StftWriteBuffers { mono: &mut mono, fft: &mut fft, window: &mut win, spectra: &mut spec, metadata: &mut meta, payload: &mut pay, file: &mut file }, &mut Scratch { metadata: &mut m, entries: &mut e }, &mut out, StftWriteOptions::default()).unwrap();
    let qac_stft_bytes = out[..n].to_vec();
    let ret_pcm = RetainedQac::open(qac_stft_bytes.clone().into_boxed_slice(), stft_doc.pcm_entry_index(), PortalQacLimits::default()).unwrap();
    let pcm_w: PcmWindow<'_> = ret_pcm.pcm_window(0, 4, 0).unwrap();
    assert_eq!(pcm_w.sample(0).unwrap(), QacSample::F32(0.5));
    let ret_stft = RetainedQac::open(qac_stft_bytes.clone().into_boxed_slice(), stft_doc.stft_entry_index(), PortalQacLimits::default()).unwrap();
    assert_eq!((ret_stft.metadata().transform.unwrap().window_kind(), ret_stft.metadata().transform.unwrap().origin_basis()), (WindowKind::HannSymmetricF32, OriginBasis::SourceEntryFirstSample));
    let stft_w: StftWindow<'_> = ret_stft.stft_window(0, 1, 0).unwrap();
    assert_eq!((stft_w.bins(), stft_w.frequency_offset(0).unwrap().numerator), (4, 0));
    for bad_magic in [*b"UA4Q\0\0\0\0", *b"Q4AU\0\0\0\0"] {
        let mut legacy_qac = qac_stft_bytes.clone();
        legacy_qac[..8].copy_from_slice(&bad_magic);
        assert!(RetainedQac::open(legacy_qac.into_boxed_slice(), 1, PortalQacLimits::default()).is_err());
    }
    let qclk = ClockSpec { scale: 0, epoch_seconds: 1_700_000_000, epoch_nanoseconds: 0, ticks_per_second: RationalRate::new(48_000, 1).unwrap() };
    let qprov = ProvenanceSpec { role: 0, source_digests: &[], algorithm: Some("urn:qac:test"), algorithm_version: Some("1"), instrument_uri: Some("urn:qac:inst"), authority_receipt_digest: None, lineage: Some("gate") };
    let (rho, bulk) = (1.2041_f64, 142_000.0_f64);
    let pv_rec = plane_wave_record([0.0; 3], 0.0, 2.0, UnitDirection3::new([1.0, 0.0, 0.0]).unwrap(), 500.0, rho, bulk);
    let pv_qac = write_pressure_velocity_qac(&[pv_rec], &FluidMediumModel::FluidLinear { density_kg_m3: rho, bulk_modulus_pa: bulk, temperature_k: Some(293.15), ambient_pressure_pa: Some(101_325.0), flow_m_s: None, matter_digest: None, material_table_digest: None, material_table_epoch: None, material_ids: vec![1] }, &StaggeringOffsetsPV::collocated(), &qclk, RationalRate::new(48_000, 1).unwrap(), &FrameSpec::local_default(), &qprov, None, ProfileLimits::default()).unwrap();
    assert_eq!((AdmittedEnvelope::open_alloc(&pv_qac, ProfileLimits::default()).unwrap().inspect_entry_summary(1).unwrap().kind, read_pressure_velocity_qac(&pv_qac, 1, ProfileLimits::default()).unwrap().records.len()), (AdmittedEntryKind::PressureVelocity, 1));
    let el_qac = write_elastic_stress_qac(&[ElasticStressVelocityRecord { velocity_m_s: [0.01, -0.02, 0.03], material_ref: 0, stress_xx_yy_zz_xy_yz_xz_pa: [100.0; 6], displacement_m: [0.0; 3], validity_bits: ELASTIC_VALID_ALL }], &ElasticMediumSpec::steel(), None, &qclk, RationalRate::new(100_000, 1).unwrap(), &FrameSpec::local_default(), &qprov, None, ProfileLimits::default()).unwrap();
    assert_eq!((AdmittedEnvelope::open_alloc(&el_qac, ProfileLimits::default()).unwrap().inspect_entry_summary(1).unwrap().kind, read_elastic_stress_qac(&el_qac, 1, ProfileLimits::default()).unwrap().records.len()), (AdmittedEntryKind::ElasticStressVelocity, 1));
    let hoa_qac = write_hoa_temporal_qac(&[&[1.0f32, 0.0, 0.0, 1.0]], &HoaProfileSpec { order: 1, ordering: HoaOrdering::Acn, normalization: HoaNormalization::Sn3d, direction_convention: HoaDirectionConvention::ListenerToSource, units: "Pa", sample_rate: RationalRate::new(48_000, 1).unwrap() }, &qclk, &FrameSpec::local_default(), &qprov, None, ProfileLimits::default()).unwrap();
    assert_eq!((AdmittedEnvelope::open_alloc(&hoa_qac, ProfileLimits::default()).unwrap().inspect_entry_summary(1).unwrap().kind, read_hoa_temporal_qac(&hoa_qac, 1, ProfileLimits::default()).unwrap().chunk.frame_count()), (AdmittedEntryKind::HoaSpatial, 1));
    let hrir_qac = write_hrir_qac(&[1.0f32, 0.0, 0.5, 0.0], &HrirWriteSpec { representation: "HRIR", taps: 2, receivers: 2, measurements: 1, impulse_sample_rate: RationalRate::new(48_000, 1).unwrap(), delay_reference: BASE_DELAY_REFERENCE, sofa_convention: Some("SimpleFreeFieldHRIR"), units: "1", delays_samples: Some(&[1.0, 2.0]), source_positions_xyz: Some(&[[1.0, 0.0, 0.0]]), receiver_positions_xyz: Some(&[[0.0, 0.09, 0.0], [0.0, -0.09, 0.0]]), measurement_cadence: None }, &FrameSpec::local_default(), &qprov, ProfileLimits::default()).unwrap();
    assert_eq!((AdmittedEnvelope::open_alloc(&hrir_qac, ProfileLimits::default()).unwrap().inspect_entry_summary(1).unwrap().kind, read_hrir_qac(&hrir_qac, 1, ProfileLimits::default()).unwrap().chunk.taps()), (AdmittedEntryKind::HrirImpulse, 2));
    let modal_qac = write_modal_bank_qac(&[ModalRecord { frequency_hz: 440.0, amplitude_pa: 1.0, decay_rate_s1: 2.302585, phase_rad: 0.0, mode_shape_index: 0, material_ref: 0, excitation_index: 0 }], &[1.0f32, -0.5], 2, &[2.0f32], Some(&[SampleStatus::Valid]), &qclk, RationalRate::new(48_000, 1).unwrap(), &qprov, ProfileLimits::default()).unwrap();
    assert_eq!((AdmittedEnvelope::open_alloc(&modal_qac, ProfileLimits::default()).unwrap().inspect_entry_summary(1).unwrap().kind, read_modal_bank_qac(&modal_qac, 1, ProfileLimits::default()).unwrap().records.len()), (AdmittedEntryKind::ModalResonance, 1));
    let lufs_qac = write_perceptual_features_qac(&[&[-14.2f32]], &PerceptualWriteSpec { algorithm_uri: "urn:qac:lufs", algorithm_version: "1", scale: PerceptualScale::Lufs, feature_count: 1, feature_names: "lufs", centers_hz: Some(&[1000.0]), filter_coefficients: None, reference_pressure_pa: None, channel_weights: Some(&[1.0]), gating_profile: Some("ITU-R BS.1770-5"), frame_rate: RationalRate::new(100, 1).unwrap() }, &qclk, &qprov, None, ProfileLimits::default()).unwrap();
    assert_eq!((AdmittedEnvelope::open_alloc(&lufs_qac, ProfileLimits::default()).unwrap().inspect_entry_summary(1).unwrap().kind, read_perceptual_features_qac(&lufs_qac, 1, ProfileLimits::default()).unwrap().scale), (AdmittedEntryKind::PerceptualFeatures, PerceptualScale::Lufs));
    let hap_qac = write_haptic_qac(&[HapticRecord { contact_position_m: [0.0; 3], contact_area_m2: 1.0e-4, force_local_xyz_n: [0.0, 0.0, 2.5], normal_indentation_m: 0.002, displacement_local_xyz_m: [0.0, 0.0, -0.002], carrier_hz: 200.0, validity_bits: HAPTIC_VALID_ALL, contact_ref: 4 }], &HapticContactSpec { role: HapticRole::Measurement { receptor_model_uri: None }, contact_frame: FrameSpec::local_default(), contact_area_m2: Some(1.0e-4) }, None, &qclk, RationalRate::new(1000, 1).unwrap(), &FrameSpec::local_default(), &qprov, None, ProfileLimits::default()).unwrap();
    assert_eq!(AdmittedEnvelope::open_alloc(&hap_qac, ProfileLimits::default()).unwrap().inspect_entry_summary(1).unwrap().kind, AdmittedEntryKind::HapticSample);
    let hap_doc = read_haptic_qac(&hap_qac, 1, ProfileLimits::default()).unwrap();
    assert!(matches!(evaluate_actuator_command_at(&hap_doc, &InstantSpec::new(1, 10, 0, 1).unwrap(), &qclk, &|_, _| true), Err(ProfileError::ActuationAuthority(_))));
    let us_qac = write_ultrasound_qac(&[UltrasoundBeamSummaryRecord { position_m: [0.0, 0.0, 0.02], center_frequency_hz: 2.5e6, peak_rarefactional_pressure_pa: 3.8e6, intensity_ispta_w_m2: 12_000.0, mechanical_index: 2.4, thermal_index: 0.0, complex_envelope_iq: [1.25, -0.75], axial_velocity_m_s: 0.5, harmonic_ratio_linear: 0.15, validity_bits: US_VALID_PR | US_VALID_ISPTA | US_VALID_MI | US_VALID_IQ | US_VALID_AXIAL_VEL | US_VALID_HARMONIC, acquisition_ref: 4 }], &UltrasoundAcquisitionSpec { role: UltrasoundRole::BeamSummary, center_frequency_hz: Some(2.5e6), element_positions_m: Some(&[[0.0; 3]]), channel_map: Some(&[0]), transmit_delays_s: Some(&[0.0]), demodulation_phase_rad: Some(0.0), prf: Some(RationalRate::new(4000, 1).unwrap()), derating_model_uri: Some("urn:fda"), exposure_context: None, thermal_model_uri: None, instrument_digest: Some([0x33; 32]), authority_receipt_digest: None, target_frame: None, calibration: None }, Some(&RawUltrasoundPayload::RawRfF32 { channels: 1, sample_rate: RationalRate::new(40_000_000, 1).unwrap(), samples_interleaved: &[0.25f32; 8] }), &qclk, RationalRate::new(60, 1).unwrap(), &FrameSpec::local_default(), &qprov, None, ProfileLimits::default()).unwrap();
    let us_idx = find_ultrasound_summary_entry_index(&us_qac, ProfileLimits::default()).unwrap();
    assert_eq!((AdmittedEnvelope::open_alloc(&us_qac, ProfileLimits::default()).unwrap().inspect_entry_summary(us_idx).unwrap().kind, read_ultrasound_qac(&us_qac, us_idx, ProfileLimits::default()).unwrap().records.len()), (AdmittedEntryKind::UltrasoundSummary, 1));
}

#[test]
fn bundle_field_range_planner_and_mixed_domain_schema6_schema7_gate() {
    let build_emf = |carrier_hz: f64| {
        let samples = [FieldSample { electric_v_per_m: [1.5, -0.5, 0.25], magnetic_tesla: [0.0, 5.0e-9, 0.0], position_m: [1.0, 2.0, 3.0], tick: 10, carrier_hz: Some(carrier_hz), stored_poynting_w_per_m2: None, confidence: Some(0.95) }];
        let mut pbuf = [0u8; 128];
        let pay = encode_field_samples(&samples, &mut pbuf, EmfProfileLimits::default()).unwrap();
        let chunks = [FieldChunkInput { stream_id: 1, start_tick: 10, time_count: 1, spatial_key: None, payload: pay }];
        let doc = FieldDocument { clock: FieldClock { scale: 1, epoch_seconds: 1_700_000_000, epoch_nanoseconds: 0, ticks_per_second_num: 1000, ticks_per_second_den: 1 }, frame: FieldFrame { origin_m: [0.0; 3], rotation_xyzw: [0.0, 0.0, 0.0, 1.0], grid: None }, provenance: FieldProvenance { role: 0, algorithm: Some("maxwell_probe"), algorithm_version: Some("1.0"), lineage: None }, temporal_step_ticks: Some(1), chunks: &chunks, align_payloads_to_4096: false };
        let lim = FieldWriteLimits::default();
        let (mut meta, mut slots) = (vec![0u8; field_metadata_len(&doc, lim).unwrap()], [field_entry_slot(); 1]);
        let prep = prepare_field_document(&doc, &mut meta, &mut slots, lim).unwrap();
        let len = prep.encoded_len().unwrap();
        let (mut stg, mut out, mut m, mut e) = (vec![0u8; len], vec![0u8; len], [NodeState::default(); 64], [NodeState::default(); 16]);
        prep.encode_into(&mut stg, &mut out, &mut Scratch { metadata: &mut m, entries: &mut e }).unwrap();
        out
    };
    let build_qac = |val: f32, with_orig: bool| {
        let samples = [val, -val, 0.25, -0.25];
        let rate = QacRate::new(48_000, 1).unwrap();
        let doc = PcmDocument { samples: PcmSamples::F32(&samples), timing: QacTiming { scale: 1, epoch_seconds: 0, epoch_nanoseconds: 0, ticks: rate, samples: rate, start_tick: 0 }, stream_id: 1, channels: 1, channel_order: "mono", purpose: "test-pcm", quantity: PcmQuantity::Pressure, relation: SignalRelation::PhysicalBaseUnits, units_per_stored_sample: 1.0, stored_offset: 0.0, provenance: QacProvenance { role: QacProvenanceRole::Measured, algorithm: None, algorithm_version: None, instrument_uri: Some("urn:qualia:mic:1"), lineage: None }, original: with_orig.then_some(&b"raw-adc-capture"[..]) };
        let (mut meta, mut pay, mut stg, mut out) = (vec![0u8; 4096], vec![0u8; 4096], vec![0u8; 8192], vec![0u8; 8192]);
        let (mut m, mut e) = ([NodeState::default(); 64], [NodeState::default(); 16]);
        let n = doc.encode_into(WriteBuffers { metadata: &mut meta, payload: &mut pay, file: &mut stg }, &mut Scratch { metadata: &mut m, entries: &mut e }, &mut out, WriteOptions::default()).unwrap();
        out.truncate(n);
        out
    };
    let (emf_a, emf_b, qac_a, qac_mixed, acq_emf, acq_qac) = (build_emf(2.4e9), build_emf(5.0e9), build_qac(0.5, false), build_qac(0.5, true), build_acquisition_bytes(Family::EmfV4Candidate), build_acquisition_bytes(Family::QacV1Candidate));
    let mut w = BundleWriter::new();
    for (name, kind, data) in [("a.emf", "emf", emf_a.clone()), ("b.emf", "emf", emf_b.clone()), ("voice.qac", "qac", qac_a.clone()), ("voice_mixed.qac", "qac", qac_mixed.clone()), ("acq.emf", "acq", acq_emf.clone()), ("acq.qac", "acq", acq_qac.clone())] {
        w.add_file(name, kind, data, None).unwrap();
    }
    let hmc = w.build().unwrap();
    let reader = BundleReader::parse(&hmc).unwrap();
    let emf_plan = FieldEntryRangePlan::from_prefix(reader.entry("a.emf").unwrap(), &emf_a, EnvelopeLimits::default()).unwrap();
    let ca = emf_plan.plan_chunk_range(1).unwrap();
    let (pay_a, meta_a) = (&hmc[ca.hmc_data_range.start as usize..ca.hmc_data_range.end as usize], &hmc[ca.hmc_meta_range.start as usize..ca.hmc_meta_range.end as usize]);
    let va = emf_plan.verify_arrived_chunk(&ca, pay_a, meta_a).unwrap();
    assert_eq!((va.is_whole_volume_verified(), va.claim_whole_volume_receipt()), (false, Err(FieldRangeError::PartialRangeCannotClaimWholeVolume)));
    let (mut m, mut e) = ([NodeState::default(); 64], [NodeState::default(); 16]);
    let emf_chunk = va.inspect_emf_profile(&mut Scratch { metadata: &mut m, entries: &mut e }, EmfProfileLimits::default(), 1 << 20).unwrap();
    assert_eq!(emf_chunk.field(0).unwrap().unwrap().carrier_hz(), Some(2.4e9));
    let qac_plan = FieldEntryRangePlan::from_prefix(reader.entry("voice.qac").unwrap(), &qac_a, EnvelopeLimits::default()).unwrap();
    let q_ranges = qac_plan.plan_chunk_ranges(&[1]).unwrap();
    let vq = verify_arrived_field_chunk(&q_ranges[0], &hmc[q_ranges[0].hmc_data_range.start as usize..q_ranges[0].hmc_data_range.end as usize], &hmc[q_ranges[0].hmc_meta_range.start as usize..q_ranges[0].hmc_meta_range.end as usize]).unwrap();
    let sig = vq.inspect_qac_signal(&mut Scratch { metadata: &mut m, entries: &mut e }, SharedSemanticLimits::default(), QacSignalLimits::default()).unwrap();
    assert_eq!((sig.records(), sig.time_at(0).unwrap(), sig.time_at(1).unwrap()), (4, QacExactTime { numerator: 0, denominator: 1 }, QacExactTime { numerator: 1, denominator: 48_000 }));
    let mixed_plan = FieldEntryRangePlan::from_prefix(reader.entry("voice_mixed.qac").unwrap(), &qac_mixed, EnvelopeLimits::default()).unwrap();
    let mixed_ranges = mixed_plan.plan_chunk_ranges(&[1, 2]).unwrap();
    assert!(mixed_ranges[0].hmc_data_range.end <= mixed_ranges[1].hmc_data_range.start);
    let qac_view = StructuralView::open(&qac_mixed, Family::QacV1Candidate, EnvelopeLimits::default()).unwrap();
    let QacMixedEntry::Signal(orig_v) = admit_qac_mixed_entry(&qac_view, 1, &mut Scratch { metadata: &mut m, entries: &mut e }, SharedSemanticLimits::default(), QacSignalLimits::default(), AcqLimits::default()).unwrap() else { panic!() };
    assert_eq!((orig_v.mode(), orig_v.as_bytes()), (0x0104, &b"raw-adc-capture"[..]));
    let raw_h = *blake3::hash(b"raw-adc-capture").as_bytes();
    let qac_s6 = seal_env(Family::QacV1Candidate, vec![
        Tb { id: 1, schema: 1, fields: vec![tu32(1, 0), ti64(2, 0), tu32(3, 0), tu64(4, 48000), tu64(5, 1)] },
        Tb { id: 2, schema: 4, fields: vec![tu32(1, 2), tf(2, 7, raw_h.to_vec()), tstr(3, "pcm-dec"), tstr(7, "derive")] },
        Tb { id: 3, schema: 205, fields: vec![tstr(1, "urn:qualia:qac:pcm-preservation:1"), tstr(2, "1"), tu64s(14, &[2, 1]), tstr(15, "sample")] },
        Tb { id: 4, schema: 210, fields: vec![tstr(1, "ch0"), tf64(2, 1.0), tf64(3, 0.0), tstr(4, "audio"), tstr(5, "digital-unrelated-to-pressure")] },
        Tb { id: 5, schema: 3, fields: vec![tref(1, 1), tref(3, 2), tstr(7, "digital_sample"), tstr(8, "1"), tu32(9, 1), tstr(10, "time-channel"), tu32(11, 1), tstr(12, "urn:qualia:qac:pcm-preservation:1"), tu64(13, 48000), tu64(14, 1), tchunk(17, 1), tf(18, 8, [3u32.to_le_bytes(), 4u32.to_le_bytes()].concat())] },
        Tb { id: 10, schema: 3, fields: vec![tref(3, 11), tstr(7, "original_bytes"), tstr(8, "1"), tu32(9, 1), tstr(10, "byte"), tu32(11, 7), tstr(12, "urn:qualia:acquisition:original:1"), tref(18, 12)] },
        Tb { id: 11, schema: 4, fields: vec![tu32(1, 4)] },
        Tb { id: 12, schema: 6, fields: vec![tu32(1, 1), tchunk(2, 1), tf(3, 7, raw_h.to_vec()), tu64(4, 0), tu64(5, 15), tu32(8, 0), tu32(10, 7), tu32(11, 1), tu32(12, 0), tu32(13, 1), tu32(14, 15), tu64(15, 1), tu32(16, 0), tu64(17, 0), tu32(32, 0)] },
    ], vec![
        env::EntryInput { mode: 0x0104, revision: 1, flags: 0, stream_id: 0, frame_id: 0, start_tick: 0, time_count: 0, spatial_key: 0, record_count: 15, stride: 1, codec: 0, descriptor_id: 10, payload: b"raw-adc-capture" },
        env::EntryInput { mode: 0x0100, revision: 1, flags: 2, stream_id: 1, frame_id: 0, start_tick: 0, time_count: 2, spatial_key: 0, record_count: 2, stride: 4, codec: 0, descriptor_id: 5, payload: &[0, 0, 128, 63, 0, 0, 0, 64] },
    ]);
    let qac_s6_view = StructuralView::open(&qac_s6, Family::QacV1Candidate, EnvelopeLimits::default()).unwrap();
    let sig_src = admit_qac_signal_with_acquisition_source(&qac_s6_view, 2, &mut Scratch { metadata: &mut m, entries: &mut e }, SharedSemanticLimits::default(), QacSignalLimits::default(), AcqLimits::default()).unwrap();
    assert_eq!((sig_src.signal.records(), sig_src.acquisition_source.original_bytes()), (2, &b"raw-adc-capture"[..]));
    for (name, bytes) in [("acq.emf", &acq_emf), ("acq.qac", &acq_qac)] {
        let plan = FieldEntryRangePlan::from_prefix(reader.entry(name).unwrap(), bytes, EnvelopeLimits::default()).unwrap();
        let ranges = plan.plan_chunk_ranges(&[1, 2]).unwrap();
        let meta = &hmc[ranges[0].hmc_meta_range.start as usize..ranges[0].hmc_meta_range.end as usize];
        let v1 = plan.verify_arrived_chunk(&ranges[0], &hmc[ranges[0].hmc_data_range.start as usize..ranges[0].hmc_data_range.end as usize], meta).unwrap();
        let v2 = plan.verify_arrived_chunk(&ranges[1], &hmc[ranges[1].hmc_data_range.start as usize..ranges[1].hmc_data_range.end as usize], meta).unwrap();
        assert_eq!(plan.claim_whole_volume_receipt(std::slice::from_ref(&v1), Some(bytes)), Err(FieldRangeError::PartialRangeCannotClaimWholeVolume));
        let assembled = v1.assemble_envelope_with_companions(&[&v2]).unwrap();
        let (mut originals, mut sm, mut se) = ([None; 8], [NodeState::default(); 256], [NodeState::default(); 128]);
        let doc = v1.inspect_acquisition_in_envelope(&assembled, &mut originals, &mut Scratch { metadata: &mut sm, entries: &mut se }, SharedSemanticLimits::default(), AcqLimits::default()).unwrap();
        let AcqView::Tensor(tv) = doc.view else { panic!("expected tensor view") };
        let lin = doc.lineage.expect("Schema 7 derivation lineage");
        assert_eq!((tv.numeric([0, 0, 0, 0]).unwrap(), lin.output_entry(), lin.input_entry(0).unwrap()), ([2.0, 0.0], 1, 2));
    }
    let plan_b = FieldEntryRangePlan::from_prefix(reader.entry("b.emf").unwrap(), &emf_b, EnvelopeLimits::default()).unwrap();
    let cb = plan_b.plan_chunk_range(1).unwrap();
    let pay_b = &hmc[cb.hmc_data_range.start as usize..cb.hmc_data_range.end as usize];
    assert_eq!((emf_plan.verify_arrived_chunk(&cb, pay_b, meta_a), verify_arrived_field_chunk(&ca, pay_b, meta_a)), (Err(FieldRangeError::CrossContainerSubstitution), Err(FieldRangeError::PayloadBlake3Mismatch)));
    let mut forged = ca.clone();
    forged.mode = 0x0104;
    assert_eq!(verify_arrived_field_chunk(&forged, pay_a, meta_a), Err(FieldRangeError::CrossModeReinterpretation { expected: 0, got: 0x0104 }));
    assert_eq!(va.assemble_envelope_with_companions(&[&vq]), Err(FieldRangeError::CrossContainerSubstitution));
}
