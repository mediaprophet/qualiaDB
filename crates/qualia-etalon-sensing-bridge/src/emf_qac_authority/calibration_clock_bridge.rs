//! Clock-discipline, half-open rational calibration validity `[valid_from, valid_until)`,
//! single-application affine calibration (C02-C05), sinusoidal phase uncertainty bounds
//! $\Delta\phi_{\max} = 2\pi f u_t$ (C02-C03), and EMF/QAC I/Q carrier convention
//! conversion (`z_QAC = conj(z_EMF)`, C02-C08) for Etalon `ObservationEnvelope` bridges.

use core::cmp::Ordering;
use core::f64::consts::TAU;

use etalon_sense_core::{observation::ObservationEnvelope, quality::CalibrationState};
use qualia_core_db::sensing::{
    acquisition::{Clock, ComplexConvention, ExactTime, Rate},
    SemanticId,
};

/// Compute a 32-byte BLAKE3 digest over `domain || len_le64 || payload` using
/// `qualia_core_db::sensing::acquisition::Original::admit` (zero heap allocation).
pub fn blake3_domain_digest(domain: &[u8], payload: &[u8]) -> [u8; 32] {
    let mut buf = [0u8; 512];
    let d_len = domain.len().min(128);
    buf[..d_len].copy_from_slice(&domain[..d_len]);
    buf[d_len..d_len + 8].copy_from_slice(&(payload.len() as u64).to_le_bytes());
    let first_chunk = payload.len().min(buf.len() - (d_len + 8));
    buf[d_len + 8..d_len + 8 + first_chunk].copy_from_slice(&payload[..first_chunk]);
    let head_len = d_len + 8 + first_chunk;
    let head_digest = qualia_core_db::sensing::acquisition::Original::admit(&buf[..head_len], head_len)
        .map(|o| o.digest())
        .unwrap_or([0u8; 32]);
    if first_chunk == payload.len() {
        return head_digest;
    }
    let tail_digest = qualia_core_db::sensing::acquisition::Original::admit(
        &payload[first_chunk..],
        payload.len() - first_chunk,
    )
    .map(|o| o.digest())
    .unwrap_or([0u8; 32]);
    let mut combined = [0u8; 64];
    combined[..32].copy_from_slice(&head_digest);
    combined[32..64].copy_from_slice(&tail_digest);
    qualia_core_db::sensing::acquisition::Original::admit(&combined, 64)
        .map(|o| o.digest())
        .unwrap_or([0u8; 32])
}

/// Exact clock instant using reduced signed `i128` rational arithmetic (`numerator / denominator`
/// seconds from the clock epoch), bound to an explicit `clock_origin: SemanticId`,
/// `restart_epoch: u64`, and `scale: u32`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BridgeRationalInstant {
    clock_origin: SemanticId,
    restart_epoch: u64,
    scale: u32,
    numerator: i128,
    denominator: i128,
}

impl BridgeRationalInstant {
    pub fn new(
        clock_origin: SemanticId,
        restart_epoch: u64,
        scale: u32,
        numerator: i128,
        denominator: i128,
    ) -> Result<Self, CalibrationClockBridgeError> {
        if clock_origin.value().is_none() {
            return Err(CalibrationClockBridgeError::InvalidIdentifier);
        }
        if scale > 2 || denominator <= 0 {
            return Err(CalibrationClockBridgeError::InvalidClockInstant);
        }
        let abs_num = numerator
            .checked_abs()
            .ok_or(CalibrationClockBridgeError::RationalOverflow)? as u128;
        let g = gcd_u128(abs_num, denominator as u128) as i128;
        Ok(Self {
            clock_origin,
            restart_epoch,
            scale,
            numerator: numerator / g,
            denominator: denominator / g,
        })
    }

    /// Construct a `BridgeRationalInstant` from an `acquisition::Clock`, integer `tick`, and
    /// Schema 5 rational subtick `(subtick_num, subtick_den)` where `0 <= subtick_num < subtick_den`.
    pub fn from_clock_tick_subtick(
        clock: Clock,
        tick: i64,
        subtick_num: u64,
        subtick_den: u64,
    ) -> Result<Self, CalibrationClockBridgeError> {
        if subtick_den == 0 || subtick_num >= subtick_den {
            return Err(CalibrationClockBridgeError::InvalidClockInstant);
        }
        let base: ExactTime = clock
            .tick_time(tick)
            .map_err(|_| CalibrationClockBridgeError::InvalidClockInstant)?;
        let base_instant = Self::new(
            clock.origin(),
            clock.restart(),
            clock.timing().scale,
            base.numerator,
            base.denominator,
        )?;
        if subtick_num == 0 {
            return Ok(base_instant);
        }
        let ticks_rate: Rate = clock.timing().ticks;
        let sub_sec_num = i128::from(subtick_num)
            .checked_mul(i128::from(ticks_rate.denominator))
            .ok_or(CalibrationClockBridgeError::RationalOverflow)?;
        let sub_sec_den = i128::from(subtick_den)
            .checked_mul(i128::from(ticks_rate.numerator))
            .ok_or(CalibrationClockBridgeError::RationalOverflow)?;
        base_instant.add_rational_seconds(sub_sec_num, sub_sec_den)
    }

    /// Extract a `BridgeRationalInstant` from an `ObservationEnvelope`'s `ClockStamp` (`ticks_ns / 10^9`).
    pub fn from_observation_envelope(
        observation: &ObservationEnvelope,
    ) -> Result<Self, CalibrationClockBridgeError> {
        let stamp = observation
            .timestamp()
            .ok_or(CalibrationClockBridgeError::MissingObservationTimestamp)?;
        let origin = SemanticId::new(stamp.clock_id())
            .map_err(|_| CalibrationClockBridgeError::InvalidIdentifier)?;
        Self::new(
            origin,
            u64::from(stamp.restart_epoch()),
            0,
            i128::from(stamp.ticks_ns()),
            1_000_000_000,
        )
    }

    pub const fn clock_origin(self) -> SemanticId {
        self.clock_origin
    }
    pub const fn restart_epoch(self) -> u64 {
        self.restart_epoch
    }
    pub const fn scale(self) -> u32 {
        self.scale
    }
    pub const fn numerator(self) -> i128 {
        self.numerator
    }
    pub const fn denominator(self) -> i128 {
        self.denominator
    }

    pub fn same_clock_domain(self, other: Self) -> bool {
        self.clock_origin == other.clock_origin
            && self.restart_epoch == other.restart_epoch
            && self.scale == other.scale
            && self.clock_origin.value().is_some()
    }

    pub fn add_rational_seconds(
        self,
        delta_num: i128,
        delta_den: i128,
    ) -> Result<Self, CalibrationClockBridgeError> {
        if delta_den <= 0 {
            return Err(CalibrationClockBridgeError::InvalidClockInstant);
        }
        let g = gcd_u128(self.denominator as u128, delta_den as u128) as i128;
        let a = delta_den / g;
        let b = self.denominator / g;
        let left = self
            .numerator
            .checked_mul(a)
            .ok_or(CalibrationClockBridgeError::RationalOverflow)?;
        let right = delta_num
            .checked_mul(b)
            .ok_or(CalibrationClockBridgeError::RationalOverflow)?;
        let sum_num = left
            .checked_add(right)
            .ok_or(CalibrationClockBridgeError::RationalOverflow)?;
        let lcm_den = self
            .denominator
            .checked_mul(a)
            .ok_or(CalibrationClockBridgeError::RationalOverflow)?;
        Self::new(
            self.clock_origin,
            self.restart_epoch,
            self.scale,
            sum_num,
            lcm_den,
        )
    }

    /// Exact comparison between two instants on the same clock domain (zero tolerance).
    pub fn checked_cmp(self, other: Self) -> Result<Ordering, CalibrationClockBridgeError> {
        if !self.same_clock_domain(other) {
            return Err(CalibrationClockBridgeError::CrossClockOriginRejected);
        }
        if self.denominator == other.denominator {
            return Ok(self.numerator.cmp(&other.numerator));
        }
        let g = gcd_u128(self.denominator as u128, other.denominator as u128) as i128;
        let lhs = self
            .numerator
            .checked_mul(other.denominator / g)
            .ok_or(CalibrationClockBridgeError::RationalOverflow)?;
        let rhs = other
            .numerator
            .checked_mul(self.denominator / g)
            .ok_or(CalibrationClockBridgeError::RationalOverflow)?;
        Ok(lhs.cmp(&rhs))
    }
}

/// Half-open validity interval `[valid_from, valid_until)` for calibration certificates
/// (EMF Schema 107, QAC Schema 202) and simulator command expiry (Schema 207/208).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BridgeCalibrationWindow {
    valid_from: BridgeRationalInstant,
    valid_until: BridgeRationalInstant,
}

impl BridgeCalibrationWindow {
    pub fn new(
        valid_from: BridgeRationalInstant,
        valid_until: BridgeRationalInstant,
    ) -> Result<Self, CalibrationClockBridgeError> {
        if valid_from.checked_cmp(valid_until)? != Ordering::Less {
            return Err(CalibrationClockBridgeError::InvalidValidityInterval);
        }
        Ok(Self {
            valid_from,
            valid_until,
        })
    }

    pub const fn valid_from(self) -> BridgeRationalInstant {
        self.valid_from
    }
    pub const fn valid_until(self) -> BridgeRationalInstant {
        self.valid_until
    }

    /// Strictly admit `instant` iff `valid_from <= instant < valid_until` (zero tolerance).
    /// Rejects at `instant == valid_until` with `CalibrationClockBridgeError::ExpiredAtOrAfterUntil`.
    pub fn verify_contains(
        self,
        instant: BridgeRationalInstant,
    ) -> Result<(), CalibrationClockBridgeError> {
        if instant.checked_cmp(self.valid_from)? == Ordering::Less {
            return Err(CalibrationClockBridgeError::NotYetValid);
        }
        if instant.checked_cmp(self.valid_until)? != Ordering::Less {
            return Err(CalibrationClockBridgeError::ExpiredAtOrAfterUntil);
        }
        Ok(())
    }

    /// Verify that an `ObservationEnvelope`'s timestamp lies inside `[valid_from, valid_until)`
    /// and that its `CalibrationState::Calibrated { artifact_id }` matches `expected_artifact_id`.
    pub fn verify_observation_calibration(
        self,
        observation: &ObservationEnvelope,
        expected_artifact_id: &[u8; 32],
    ) -> Result<BridgeRationalInstant, CalibrationClockBridgeError> {
        if *expected_artifact_id == [0u8; 32] {
            return Err(CalibrationClockBridgeError::InvalidIdentifier);
        }
        match observation.calibration().state() {
            CalibrationState::Calibrated { artifact_id } => {
                if &artifact_id.bytes() != expected_artifact_id {
                    return Err(CalibrationClockBridgeError::CalibrationArtifactMismatch);
                }
            }
            CalibrationState::Expired => {
                return Err(CalibrationClockBridgeError::ExpiredAtOrAfterUntil);
            }
            _ => return Err(CalibrationClockBridgeError::CalibrationArtifactMismatch),
        }
        let instant = BridgeRationalInstant::from_observation_envelope(observation)?;
        self.verify_contains(instant)?;
        Ok(instant)
    }
}

/// Reject joins across relative clocks with different origins or restart epochs even when
/// `ticks_per_second`, `epoch_seconds`, and `start_tick` match (C02-C03).
pub fn verify_same_clock_domain(
    clock_a: Clock,
    clock_b: Clock,
) -> Result<(), CalibrationClockBridgeError> {
    if clock_a.origin() != clock_b.origin() || clock_a.restart() != clock_b.restart() {
        return Err(CalibrationClockBridgeError::CrossClockOriginRejected);
    }
    clock_a
        .require_same_domain(clock_b)
        .map_err(|_| CalibrationClockBridgeError::ClockParametersMismatch)
}

/// Compute sinusoidal phase uncertainty $\Delta\phi_{\max} = 2\pi f u_t$ in radians (C02-C03)
/// from an `ObservationEnvelope`'s `uncertainty_ns` (or an explicit $u_t$ override) at carrier/analysis
/// frequency `frequency_hz`.
pub fn compute_envelope_phase_uncertainty_rad(
    observation: &ObservationEnvelope,
    frequency_hz: f64,
    additional_mapping_uncertainty_seconds: f64,
) -> Result<f64, CalibrationClockBridgeError> {
    if !frequency_hz.is_finite()
        || frequency_hz <= 0.0
        || !additional_mapping_uncertainty_seconds.is_finite()
        || additional_mapping_uncertainty_seconds < 0.0
    {
        return Err(CalibrationClockBridgeError::InvalidFrequencyOrUncertainty);
    }
    let stamp = observation
        .timestamp()
        .ok_or(CalibrationClockBridgeError::MissingObservationTimestamp)?;
    let u_t_seconds =
        (stamp.uncertainty_ns() as f64) * 1.0e-9 + additional_mapping_uncertainty_seconds;
    let delta_phi = TAU * frequency_hz * u_t_seconds;
    if !delta_phi.is_finite() {
        return Err(CalibrationClockBridgeError::InvalidFrequencyOrUncertainty);
    }
    Ok(delta_phi)
}

/// Admit a phase-coherent join across two `ObservationEnvelope`s at `frequency_hz` only when
/// their clock domains and timestamps match and total phase uncertainty
/// $\Delta\phi_{\max} = 2\pi f (u_{t,a} + u_{t,b}) \le \Delta\phi_{\text{budget}}$.
pub fn verify_phase_coherent_envelope_join(
    left: &ObservationEnvelope,
    right: &ObservationEnvelope,
    frequency_hz: f64,
    max_phase_uncertainty_rad: f64,
) -> Result<f64, CalibrationClockBridgeError> {
    if !max_phase_uncertainty_rad.is_finite() || max_phase_uncertainty_rad < 0.0 {
        return Err(CalibrationClockBridgeError::InvalidFrequencyOrUncertainty);
    }
    let t_left = BridgeRationalInstant::from_observation_envelope(left)?;
    let t_right = BridgeRationalInstant::from_observation_envelope(right)?;
    if t_left.checked_cmp(t_right)? != Ordering::Equal {
        return Err(CalibrationClockBridgeError::InstantNotSimultaneous);
    }
    let phi_left = compute_envelope_phase_uncertainty_rad(left, frequency_hz, 0.0)?;
    let phi_right = compute_envelope_phase_uncertainty_rad(right, frequency_hz, 0.0)?;
    let total_phi = phi_left + phi_right;
    if total_phi > max_phase_uncertainty_rad {
        return Err(CalibrationClockBridgeError::PhaseUncertaintyExceedsBudget);
    }
    Ok(total_phi)
}

/// Explicit I/Q complex carrier conventions (C02-C08).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BridgeIqConvention {
    /// EMF Schema 136 / Mode `0x0105` (`"I_plus_iQ_exp_minus_iwt"`):
    /// $\operatorname{Re}((I + iQ) e^{-i(2\pi f_c(t - t_{\text{LO}}) + \phi_{\text{LO}})})$.
    EmfNegativeCarrier,
    /// QAC Schema 208 / Mode `0x0107` (`"I_plus_iQ_exp_plus_iwt"`):
    /// $\operatorname{Re}((I + iQ) e^{+i(2\pi f_c(t - t_0) + \phi_0)})$.
    QacPositiveCarrier,
}

impl BridgeIqConvention {
    pub const fn to_acquisition_convention(self) -> ComplexConvention {
        match self {
            Self::EmfNegativeCarrier => ComplexConvention::RealPlusIImagNegativeExponent,
            Self::QacPositiveCarrier => ComplexConvention::RealPlusIImagPositiveExponent,
        }
    }
}

/// Qualified complex I/Q sample for carrier-convention conversion and real-waveform verification.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BridgeIqSample {
    pub i: f64,
    pub q: f64,
    pub convention: BridgeIqConvention,
    pub carrier_hz: f64,
    pub reference_phase_rad: f64,
    pub signed_baseband_offset_hz: f64,
}

impl BridgeIqSample {
    pub fn validate(&self) -> Result<(), CalibrationClockBridgeError> {
        if !self.i.is_finite()
            || !self.q.is_finite()
            || !self.carrier_hz.is_finite()
            || self.carrier_hz <= 0.0
            || !self.reference_phase_rad.is_finite()
            || !self.signed_baseband_offset_hz.is_finite()
        {
            return Err(CalibrationClockBridgeError::NonFiniteSample);
        }
        Ok(())
    }

    pub fn reconstruct_real_at(&self, dt_seconds: f64) -> Result<f64, CalibrationClockBridgeError> {
        self.validate()?;
        if !dt_seconds.is_finite() {
            return Err(CalibrationClockBridgeError::NonFiniteSample);
        }
        let theta = TAU * self.carrier_hz * dt_seconds + self.reference_phase_rad;
        let (sin_t, cos_t) = theta.sin_cos();
        Ok(match self.convention {
            BridgeIqConvention::EmfNegativeCarrier => self.i * cos_t + self.q * sin_t,
            BridgeIqConvention::QacPositiveCarrier => self.i * cos_t - self.q * sin_t,
        })
    }
}

/// Reject raw `memcpy` / unconjugated reinterpretation of $Q$ between carrier conventions (C02-C08).
pub fn reject_unconjugated_iq_memcpy(
    source: BridgeIqConvention,
    target: BridgeIqConvention,
) -> Result<(), CalibrationClockBridgeError> {
    if source != target {
        return Err(CalibrationClockBridgeError::UnconjugatedCarrierMemcpyRejected);
    }
    Ok(())
}

/// Explicitly convert a `BridgeIqSample` between `EmfNegativeCarrier` and `QacPositiveCarrier`
/// (`z_QAC = conj(z_EMF)` and signed baseband offset negation, plus optional phase-origin shift
/// $\Delta t_0 = t_{0,\text{target}} - t_{0,\text{source}}$).
pub fn convert_iq_between_emf_and_qac(
    source: BridgeIqSample,
    target_convention: BridgeIqConvention,
    phase_origin_shift_seconds: f64,
) -> Result<BridgeIqSample, CalibrationClockBridgeError> {
    source.validate()?;
    if !phase_origin_shift_seconds.is_finite() {
        return Err(CalibrationClockBridgeError::NonFiniteSample);
    }
    let (mut i_out, mut q_out, mut offset_out) = if source.convention == target_convention {
        (source.i, source.q, source.signed_baseband_offset_hz)
    } else {
        (source.i, -source.q, -source.signed_baseband_offset_hz)
    };
    let origin_delta_theta = TAU * source.carrier_hz * phase_origin_shift_seconds;
    if origin_delta_theta != 0.0
        || (source.convention != target_convention && source.reference_phase_rad != 0.0)
    {
        let phi_eff = source.reference_phase_rad;
        let total_phase = match target_convention {
            BridgeIqConvention::EmfNegativeCarrier => -(origin_delta_theta + phi_eff),
            BridgeIqConvention::QacPositiveCarrier => origin_delta_theta + phi_eff,
        };
        let (sin_p, cos_p) = total_phase.sin_cos();
        let i_rot = i_out * cos_p - q_out * sin_p;
        let q_rot = i_out * sin_p + q_out * cos_p;
        i_out = i_rot;
        q_out = q_rot;
        if offset_out == -0.0 {
            offset_out = 0.0;
        }
        return Ok(BridgeIqSample {
            i: i_out,
            q: q_out,
            convention: target_convention,
            carrier_hz: source.carrier_hz,
            reference_phase_rad: 0.0,
            signed_baseband_offset_hz: offset_out,
        });
    }
    Ok(BridgeIqSample {
        i: i_out,
        q: q_out,
        convention: target_convention,
        carrier_hz: source.carrier_hz,
        reference_phase_rad: source.reference_phase_rad,
        signed_baseband_offset_hz: offset_out,
    })
}

/// Signal encoding stage relation (Schema 210 key 5 and C02-C05).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BridgeCalibrationStage {
    DigitalUnrelatedToPressure,
    RawCounts,
    PhysicalBaseUnits,
}

/// Apply affine calibration `physical = scale * raw + offset` once on `RawCounts` stage (C02-C05),
/// returning `(calibrated_pair, BridgeCalibrationStage::PhysicalBaseUnits, 1.0, 0.0)`.
/// Rejects double-application when `stage == PhysicalBaseUnits`.
pub fn apply_single_calibration_pair(
    stage: BridgeCalibrationStage,
    samples: (f64, f64),
    scale: f64,
    offset: f64,
    is_complex_iq: bool,
) -> Result<((f64, f64), BridgeCalibrationStage, f64, f64), CalibrationClockBridgeError> {
    match stage {
        BridgeCalibrationStage::PhysicalBaseUnits => {
            return Err(CalibrationClockBridgeError::DoubleCalibrationRejected);
        }
        BridgeCalibrationStage::DigitalUnrelatedToPressure => {
            return Err(CalibrationClockBridgeError::DigitalUnrelatedCannotCalibrate);
        }
        BridgeCalibrationStage::RawCounts => {}
    }
    if !scale.is_finite() || scale <= 0.0 || !offset.is_finite() {
        return Err(CalibrationClockBridgeError::InvalidAffineCalibration);
    }
    if is_complex_iq && offset != 0.0 {
        return Err(CalibrationClockBridgeError::NonZeroScalarComplexOffsetRejected);
    }
    if !samples.0.is_finite() || !samples.1.is_finite() {
        return Err(CalibrationClockBridgeError::NonFiniteSample);
    }
    let y0 = samples.0 * scale + offset;
    let y1 = samples.1 * scale + offset;
    if !y0.is_finite() || !y1.is_finite() {
        return Err(CalibrationClockBridgeError::NonFiniteSample);
    }
    Ok((
        (y0, y1),
        BridgeCalibrationStage::PhysicalBaseUnits,
        1.0,
        0.0,
    ))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CalibrationClockBridgeError {
    InvalidIdentifier,
    InvalidClockInstant,
    MissingObservationTimestamp,
    RationalOverflow,
    CrossClockOriginRejected,
    ClockParametersMismatch,
    InvalidValidityInterval,
    NotYetValid,
    ExpiredAtOrAfterUntil,
    CalibrationArtifactMismatch,
    InvalidFrequencyOrUncertainty,
    InstantNotSimultaneous,
    PhaseUncertaintyExceedsBudget,
    NonFiniteSample,
    UnconjugatedCarrierMemcpyRejected,
    InvalidAffineCalibration,
    NonZeroScalarComplexOffsetRejected,
    DoubleCalibrationRejected,
    DigitalUnrelatedCannotCalibrate,
}

const fn gcd_u128(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        let r = a % b;
        a = b;
        b = r;
    }
    if a == 0 {
        1
    } else {
        a
    }
}
