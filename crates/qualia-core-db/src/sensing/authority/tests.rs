use super::certificates::{
    calibrate_complex_iq_pair_once, verify_calibration_certificate_chain, CalibratedSeriesState,
    CalibrationCertificate, CalibrationError, CalibrationSchemaFamily, CalibrationValidityWindow,
    QualifiedUncertainty, RationalInstant, SignalStageRelation, UncertaintyKind,
};
use super::clock_discipline::{
    compute_phase_uncertainty_rad, convert_iq_sample_between_conventions,
    reject_unconjugated_iq_memcpy, verify_phase_coherent_join, verify_same_clock_join,
    ClockDisciplineError, CrossClockMapping, IqCarrierConvention, LeapSecondStep, LeapSecondTable,
    QualifiedIqSample,
};

use crate::audio::qac::signal::{Rate, Timing};
use crate::governance::consent_contract::ConsentGrant;
use crate::sensing::acquisition::Clock;
use crate::sensing::authorization::{
    AuthorizationError, IdentityAuthority, RevocationAuthority, TrustedUnixClock,
};
use crate::sensing::SemanticId;
use core::f64::consts::{FRAC_PI_2, PI, TAU};
use ed25519_dalek::SigningKey;

pub(super) struct FixedClock {
    pub(super) now: u64,
}
impl TrustedUnixClock for FixedClock {
    fn now_unix(&mut self) -> Result<u64, AuthorizationError> {
        Ok(self.now)
    }
}

pub(super) struct TestIdentity {
    pub(super) expected_principal: [u8; 32],
    pub(super) expected_actor: [u8; 32],
}
impl IdentityAuthority for TestIdentity {
    fn verify_parties(
        &self,
        grant: &ConsentGrant,
        actor_id: &[u8; 32],
    ) -> Result<(), AuthorizationError> {
        if grant.principal_did == self.expected_principal && *actor_id == self.expected_actor {
            Ok(())
        } else {
            Err(AuthorizationError::IdentityUnverified)
        }
    }
}

pub(super) struct FlagRevocation {
    pub(super) revoked: bool,
}
impl RevocationAuthority for FlagRevocation {
    fn is_revoked(
        &self,
        _grant_id: &[u8; 32],
        _grant_digest: &[u8; 32],
        _current_time_unix: u64,
    ) -> Result<bool, AuthorizationError> {
        Ok(self.revoked)
    }
}

pub(super) fn sample_clock(origin_id: u64, restart: u64, ticks_per_sec: u64) -> Clock {
    let origin = SemanticId::new(origin_id).unwrap();
    let timing = Timing {
        scale: 0,
        epoch_seconds: 1_700_000_000,
        epoch_nanoseconds: 0,
        ticks: Rate::new(ticks_per_sec, 1).unwrap(),
        samples: Rate::new(ticks_per_sec, 1).unwrap(),
        start_tick: 0,
    };
    Clock::new(origin, restart, timing).unwrap()
}

fn sample_leaf_certificate(
    issuer_key: &SigningKey,
    subject_key: &SigningKey,
    parent_digest: Option<[u8; 32]>,
    window: CalibrationValidityWindow,
    scale: f64,
    offset: f64,
) -> CalibrationCertificate {
    let q_in = SemanticId::new(1001).unwrap();
    let q_out = SemanticId::new(1002).unwrap();
    let u_out = SemanticId::new(2002).unwrap();
    let cert = CalibrationCertificate {
        schema_family: CalibrationSchemaFamily::EmfSchema107,
        certificate_id: [0x44; 32],
        parent_certificate_digest: parent_digest,
        instrument_id: SemanticId::new(501).unwrap(),
        channel_index: 0,
        input_quantity: q_in,
        output_quantity: q_out,
        output_unit: u_out,
        affine_scale: scale,
        affine_offset: offset,
        reference_level: Some(1.0),
        frequency_band_hz: Some([1.0e3, 1.0e9]),
        validity_window: window,
        uncertainty: QualifiedUncertainty {
            value: 0.01,
            target_quantity: q_out,
            target_unit: u_out,
            kind: UncertaintyKind::ExpandedAbsolute {
                coverage_factor_k: 2.0,
                confidence_probability: 0.95,
            },
            estimator_id: SemanticId::new(77).unwrap(),
        },
        issuer_public_key: [0; 32],
        subject_public_key: subject_key.verifying_key().to_bytes(),
        signature: [0; 64],
    };
    cert.sign(issuer_key).unwrap()
}

#[test]
fn calibration_half_open_interval_enforces_exact_rational_boundaries() {
    let clk = sample_clock(11, 1, 96_000);
    let valid_from = RationalInstant::from_clock_tick_subtick(clk, 1000, 0, 1).unwrap();
    let valid_until = RationalInstant::from_clock_tick_subtick(clk, 2000, 0, 1).unwrap();
    let window = CalibrationValidityWindow::new(valid_from, valid_until).unwrap();

    // 1. valid_from - 1 tick -> rejected
    let before_from = RationalInstant::from_clock_tick_subtick(clk, 999, 0, 1).unwrap();
    assert_eq!(
        window.verify_contains(before_from),
        Err(CalibrationError::CalibrationNotYetValid)
    );

    // 2. valid_from - 1 subtick (tick 999 + 999/1000 subtick) -> rejected
    let subtick_before_from =
        RationalInstant::from_clock_tick_subtick(clk, 999, 999, 1000).unwrap();
    assert_eq!(
        window.verify_contains(subtick_before_from),
        Err(CalibrationError::CalibrationNotYetValid)
    );

    // 3. Exactly valid_from -> admitted
    assert_eq!(window.verify_contains(valid_from), Ok(()));

    // 4. valid_until - 1 subtick (tick 1999 + 47999/48000 subtick) -> admitted
    let one_subtick_before_until =
        RationalInstant::from_clock_tick_subtick(clk, 1999, 47_999, 48_000).unwrap();
    assert_eq!(window.verify_contains(one_subtick_before_until), Ok(()));

    // 5. Exactly valid_until -> rejected with zero tolerance!
    assert_eq!(
        window.verify_contains(valid_until),
        Err(CalibrationError::CalibrationExpiredAtOrAfterUntil)
    );

    // 6. valid_until + 1 subtick -> rejected
    let after_until = RationalInstant::from_clock_tick_subtick(clk, 2000, 1, 48_000).unwrap();
    assert_eq!(
        window.verify_contains(after_until),
        Err(CalibrationError::CalibrationExpiredAtOrAfterUntil)
    );
}

#[test]
fn certificate_chain_verifies_root_to_leaf_and_rejects_broken_links() {
    let clk = sample_clock(11, 1, 48_000);
    let root_from = RationalInstant::from_clock_tick_subtick(clk, 0, 0, 1).unwrap();
    let root_until = RationalInstant::from_clock_tick_subtick(clk, 10_000, 0, 1).unwrap();
    let leaf_from = RationalInstant::from_clock_tick_subtick(clk, 1_000, 0, 1).unwrap();
    let leaf_until = RationalInstant::from_clock_tick_subtick(clk, 5_000, 0, 1).unwrap();

    let root_key = SigningKey::from_bytes(&[1u8; 32]);
    let lab_key = SigningKey::from_bytes(&[2u8; 32]);
    let sensor_key = SigningKey::from_bytes(&[3u8; 32]);

    let root_window = CalibrationValidityWindow::new(root_from, root_until).unwrap();
    let leaf_window = CalibrationValidityWindow::new(leaf_from, leaf_until).unwrap();

    let root_cert = sample_leaf_certificate(&root_key, &lab_key, None, root_window, 1.0, 0.0);
    let leaf_cert = sample_leaf_certificate(
        &lab_key,
        &sensor_key,
        Some(root_cert.certificate_digest()),
        leaf_window,
        2.0,
        0.0,
    );

    let obs_instant = RationalInstant::from_clock_tick_subtick(clk, 2_500, 0, 1).unwrap();
    let trusted_root_pk = root_key.verifying_key().to_bytes();
    let leaf_digest = verify_calibration_certificate_chain(
        &[root_cert, leaf_cert],
        &trusted_root_pk,
        obs_instant,
        Some(10.0e3),
    )
    .unwrap();
    assert_eq!(leaf_digest, leaf_cert.certificate_digest());

    // Exact leaf expiry tick 5_000 must fail closed
    assert_eq!(
        verify_calibration_certificate_chain(
            &[root_cert, leaf_cert],
            &trusted_root_pk,
            leaf_until,
            Some(10.0e3)
        ),
        Err(CalibrationError::CalibrationExpiredAtOrAfterUntil)
    );
}

#[test]
fn single_application_rule_scales_3_4_by_2_to_6_8_once_and_rejects_double_calibration() {
    let clk = sample_clock(11, 1, 8_000);
    let valid_from = RationalInstant::from_clock_tick_subtick(clk, 0, 0, 1).unwrap();
    let valid_until = RationalInstant::from_clock_tick_subtick(clk, 8_000, 0, 1).unwrap();
    let window = CalibrationValidityWindow::new(valid_from, valid_until).unwrap();
    let issuer = SigningKey::from_bytes(&[5u8; 32]);
    let subject = SigningKey::from_bytes(&[6u8; 32]);
    let cert = sample_leaf_certificate(&issuer, &subject, None, window, 2.0, 0.0);

    let q_in = SemanticId::new(1001).unwrap();
    let u_in = SemanticId::new(2001).unwrap();
    let obs = RationalInstant::from_clock_tick_subtick(clk, 100, 0, 1).unwrap();

    // 1. Scalar/series calibration: (3.0, 4.0) * 2.0 -> (6.0, 8.0) once
    let raw_series = CalibratedSeriesState::from_raw_counts(q_in, u_in, [3.0, 4.0]).unwrap();
    let calibrated = raw_series
        .apply_calibration_once(&cert, obs, Some(10.0e3))
        .unwrap();
    assert_eq!(calibrated.samples(), &[6.0, 8.0]);
    assert_eq!(calibrated.stage(), SignalStageRelation::PhysicalBaseUnits);
    assert_eq!(calibrated.units_per_stored_sample(), 1.0);
    assert_eq!(calibrated.stored_offset(), 0.0);
    assert_eq!(
        calibrated.lineage_certificate_digest(),
        Some(cert.certificate_digest())
    );

    // 2. Re-applying calibration on PhysicalBaseUnits is strictly rejected
    assert_eq!(
        calibrated.apply_calibration_once(&cert, obs, Some(10.0e3)),
        Err(CalibrationError::DoubleCalibrationRejected)
    );

    // 3. Re-opening PhysicalBaseUnits preserves (6.0, 8.0) and requires scale=1.0, offset=0.0
    let reopened = CalibratedSeriesState::reopen_physical_base_units(
        cert.output_quantity,
        cert.output_unit,
        1.0,
        0.0,
        cert.certificate_digest(),
        *calibrated.samples(),
    )
    .unwrap();
    assert_eq!(reopened.samples(), &[6.0, 8.0]);
    assert_eq!(
        CalibratedSeriesState::reopen_physical_base_units(
            cert.output_quantity,
            cert.output_unit,
            2.0,
            0.0,
            cert.certificate_digest(),
            [6.0, 8.0],
        ),
        Err(CalibrationError::PhysicalBaseUnitsRequiresIdentityScaleOffset)
    );

    // 4. Complex I/Q pair calibration: (3.0, 4.0) * 2.0 -> (6.0, 8.0) once
    let (iq_cal, stage_cal, ups, digest) = calibrate_complex_iq_pair_once(
        SignalStageRelation::RawCounts,
        (3.0, 4.0),
        q_in,
        &cert,
        obs,
        10.0e3,
    )
    .unwrap();
    assert_eq!(iq_cal, (6.0, 8.0));
    assert_eq!(stage_cal, SignalStageRelation::PhysicalBaseUnits);
    assert_eq!(ups, 1.0);
    assert_eq!(digest, cert.certificate_digest());

    assert_eq!(
        calibrate_complex_iq_pair_once(stage_cal, iq_cal, q_in, &cert, obs, 10.0e3),
        Err(CalibrationError::DoubleCalibrationRejected)
    );
}

#[test]
fn mixed_unit_records_reject_unlabeled_or_mismatched_uncertainty() {
    let q_dose_gy = SemanticId::new(601).unwrap();
    let u_gy = SemanticId::new(701).unwrap();
    let q_fluence = SemanticId::new(602).unwrap();
    let u_m2 = SemanticId::new(702).unwrap();
    let q_unrelated = SemanticId::new(999).unwrap();
    let est = SemanticId::new(55).unwrap();
    let mode_0106_components = [(q_dose_gy, u_gy), (q_fluence, u_m2)];

    let valid_u = QualifiedUncertainty {
        value: 0.005,
        target_quantity: q_dose_gy,
        target_unit: u_gy,
        kind: UncertaintyKind::StandardAbsolute,
        estimator_id: est,
    };
    assert_eq!(
        valid_u.verify_against_mixed_record_components(&mode_0106_components),
        Ok(())
    );

    let unlabeled_u = QualifiedUncertainty {
        value: 0.005,
        target_quantity: SemanticId::Unknown,
        target_unit: u_gy,
        kind: UncertaintyKind::StandardAbsolute,
        estimator_id: est,
    };
    assert_eq!(
        unlabeled_u.verify_against_mixed_record_components(&mode_0106_components),
        Err(CalibrationError::UnlabeledUncertaintyRejected)
    );

    let mismatched_u = QualifiedUncertainty {
        value: 0.005,
        target_quantity: q_unrelated,
        target_unit: u_gy,
        kind: UncertaintyKind::Relative,
        estimator_id: est,
    };
    assert_eq!(
        mismatched_u.verify_against_mixed_record_components(&mode_0106_components),
        Err(CalibrationError::UncertaintyTargetMismatch)
    );
}

#[test]
fn cross_origin_clocks_reject_and_phase_uncertainty_bound_2pi_f_ut_is_enforced() {
    let clock_a = sample_clock(101, 1, 48_000);
    let clock_b = sample_clock(102, 1, 48_000);
    assert_eq!(
        verify_same_clock_join(clock_a, clock_b),
        Err(ClockDisciplineError::CrossClockOriginRejected)
    );
    assert_eq!(verify_same_clock_join(clock_a, clock_a), Ok(()));

    let t_a = RationalInstant::from_clock_tick_subtick(clock_a, 48_000, 0, 1).unwrap();
    let t_b_diff_origin = RationalInstant::from_clock_tick_subtick(clock_b, 48_000, 0, 1).unwrap();
    assert_eq!(
        verify_phase_coherent_join(t_a, t_b_diff_origin, 2.0e6, Some(1.0e-9), 0.1),
        Err(ClockDisciplineError::CrossClockOriginRejected)
    );

    let valid_from = RationalInstant::from_clock_tick_subtick(clock_b, 0, 0, 1).unwrap();
    let valid_until = RationalInstant::from_clock_tick_subtick(clock_b, 96_000, 0, 1).unwrap();
    let mapping = CrossClockMapping {
        mapping_id: SemanticId::new(888).unwrap(),
        source_origin: clock_b.origin(),
        source_restart: clock_b.restart(),
        source_scale: 0,
        target_origin: clock_a.origin(),
        target_restart: clock_a.restart(),
        target_scale: 0,
        source_validity: CalibrationValidityWindow::new(valid_from, valid_until).unwrap(),
        offset_seconds_num: 0,
        offset_seconds_den: 1,
        mapping_time_uncertainty_seconds: 2.0e-9,
    };
    let (mapped_b, combined_u_t) = mapping
        .map_instant(t_b_diff_origin, Some(3.0e-9))
        .unwrap();
    assert!((combined_u_t - 5.0e-9).abs() < 1e-18);

    let delta_phi =
        verify_phase_coherent_join(t_a, mapped_b, 2.0e6, Some(combined_u_t), 0.10).unwrap();
    let expected_phi = TAU * 2.0e6 * 5.0e-9;
    assert!((delta_phi - expected_phi).abs() <= 1e-15);

    assert_eq!(
        verify_phase_coherent_join(t_a, mapped_b, 2.0e6, Some(combined_u_t), 0.05),
        Err(ClockDisciplineError::PhaseUncertaintyExceedsBudget)
    );
    assert_eq!(
        compute_phase_uncertainty_rad(2.0e6, None),
        Err(ClockDisciplineError::UnknownOrInvalidTimeUncertainty)
    );

    let leap_table = LeapSecondTable::new(
        SemanticId::new(300).unwrap(),
        &[LeapSecondStep {
            effective_utc_second: 1_483_228_800,
            tai_minus_utc_seconds: 37,
        }],
        1_500_000_000,
        1_800_000_000,
        1.0e-9,
    )
    .unwrap();
    let utc_origin = SemanticId::new(301).unwrap();
    let tai_origin = SemanticId::new(302).unwrap();
    let utc_t = RationalInstant::new(utc_origin, 1, 1, 1_700_000_000, 1).unwrap();
    let (tai_t, u_tai) = leap_table
        .convert_utc_instant_to_tai(utc_t, tai_origin, Some(2.0e-9))
        .unwrap();
    assert_eq!(tai_t.numerator(), 1_700_000_037);
    assert_eq!(tai_t.denominator(), 1);
    assert!((u_tai - 3.0e-9).abs() < 1e-18);
}

#[test]
fn iq_carrier_conventions_conjugate_correctly_and_reject_raw_memcpy() {
    assert_eq!(
        reject_unconjugated_iq_memcpy(
            IqCarrierConvention::EmfNegativeCarrier,
            IqCarrierConvention::QacPositiveCarrier
        ),
        Err(ClockDisciplineError::UnconjugatedCarrierMemcpyRejected)
    );
    assert_eq!(
        reject_unconjugated_iq_memcpy(
            IqCarrierConvention::EmfNegativeCarrier,
            IqCarrierConvention::EmfNegativeCarrier
        ),
        Ok(())
    );

    let f_c = 10_000.0;
    let quarter_cycle_dt = 1.0 / (4.0 * f_c);

    // Case 1: I = 1, Q = 0
    let emf_i_only = QualifiedIqSample {
        i: 1.0,
        q: 0.0,
        convention: IqCarrierConvention::EmfNegativeCarrier,
        carrier_hz: f_c,
        reference_phase_rad: 0.0,
        signed_baseband_offset_hz: 1_000.0,
    };
    let qac_i_only = convert_iq_sample_between_conventions(
        emf_i_only,
        IqCarrierConvention::QacPositiveCarrier,
        0.0,
    )
    .unwrap();
    assert_eq!((qac_i_only.i, qac_i_only.q), (1.0, 0.0));
    assert_eq!(qac_i_only.signed_baseband_offset_hz, -1_000.0);
    assert!((emf_i_only.reconstruct_real_at(0.0).unwrap() - 1.0).abs() <= 1e-15);
    assert!((qac_i_only.reconstruct_real_at(0.0).unwrap() - 1.0).abs() <= 1e-15);

    // Case 2: I = 0, Q = 1 at carrier quarter-cycle (C02-C08 acceptance fixture)
    let emf_q_only = QualifiedIqSample {
        i: 0.0,
        q: 1.0,
        convention: IqCarrierConvention::EmfNegativeCarrier,
        carrier_hz: f_c,
        reference_phase_rad: 0.0,
        signed_baseband_offset_hz: 1_000.0,
    };
    let qac_converted = convert_iq_sample_between_conventions(
        emf_q_only,
        IqCarrierConvention::QacPositiveCarrier,
        0.0,
    )
    .unwrap();
    assert_eq!((qac_converted.i, qac_converted.q), (0.0, -1.0));
    assert_eq!(qac_converted.signed_baseband_offset_hz, -1_000.0);

    let real_emf = emf_q_only.reconstruct_real_at(quarter_cycle_dt).unwrap();
    let real_qac_converted = qac_converted.reconstruct_real_at(quarter_cycle_dt).unwrap();
    assert!((real_emf - 1.0).abs() <= 1e-15);
    assert!((real_emf - real_qac_converted).abs() <= 1e-15);

    // Unconverted memcpy Q = +1 in QacPositiveCarrier fails sign assertion (-1.0 vs +1.0)
    let unconverted_qac = QualifiedIqSample {
        i: 0.0,
        q: 1.0,
        convention: IqCarrierConvention::QacPositiveCarrier,
        carrier_hz: f_c,
        reference_phase_rad: 0.0,
        signed_baseband_offset_hz: 1_000.0,
    };
    let real_unconverted = unconverted_qac
        .reconstruct_real_at(quarter_cycle_dt)
        .unwrap();
    assert!((real_unconverted - (-1.0)).abs() <= 1e-15);
    assert!((real_emf - real_unconverted).abs() > 1.99);

    // Nonzero baseband tone at arbitrary time with phase-origin shift
    let tone_phase = 0.37 * PI + FRAC_PI_2 * 0.1;
    let emf_tone = QualifiedIqSample {
        i: tone_phase.cos() * 2.5,
        q: tone_phase.sin() * 2.5,
        convention: IqCarrierConvention::EmfNegativeCarrier,
        carrier_hz: f_c,
        reference_phase_rad: 0.25,
        signed_baseband_offset_hz: 400.0,
    };
    let shift_sec = 1.25e-5;
    let qac_shifted = convert_iq_sample_between_conventions(
        emf_tone,
        IqCarrierConvention::QacPositiveCarrier,
        shift_sec,
    )
    .unwrap();
    let dt_eval = 3.1e-5;
    let wf_emf = emf_tone.reconstruct_real_at(dt_eval).unwrap();
    let wf_qac = qac_shifted
        .reconstruct_real_at(dt_eval - shift_sec)
        .unwrap();
    assert!((wf_emf - wf_qac).abs() <= 1e-14);
}
