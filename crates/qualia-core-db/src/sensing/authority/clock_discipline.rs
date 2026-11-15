//! Clock-discipline verification, TAI/UTC leap-second table binding, sinusoidal phase
//! uncertainty bounds ($\Delta\phi_{\max} = 2\pi f u_t$, C02-C03), and explicit I/Q carrier
//! convention conversion (`z_QAC = conj(z_EMF)`, C02-C08).

use core::cmp::Ordering;
use core::f64::consts::TAU;

use crate::sensing::acquisition::{Clock, ComplexConvention};
use crate::sensing::authority::certificates::{CalibrationValidityWindow, RationalInstant};
use crate::sensing::SemanticId;

/// Domain separator for BLAKE3 leap-second table commitments.
pub const LEAP_SECOND_TABLE_DOMAIN: &[u8] = b"QUALIA-LEAP-SECOND-TABLE-v1\0";

/// Maximum entries in a zero-heap bounded TAI-UTC leap-second table.
pub const MAX_LEAP_SECOND_ENTRIES: usize = 64;

/// Clock scale defined by Shared Schema 1 (`Clock.scale`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum ClockScale {
    /// International Atomic Time (TAI), continuous SI seconds without leap seconds.
    Tai = 0,
    /// Coordinated Universal Time (UTC), requires an explicit `LeapSecondTable` for TAI conversion.
    Utc = 1,
    /// Relative / monotonic hardware or solver clock; requires an explicit `origin` (`SemanticId`).
    MonotonicRelative = 2,
}

impl ClockScale {
    pub const fn from_code(code: u32) -> Result<Self, ClockDisciplineError> {
        match code {
            0 => Ok(Self::Tai),
            1 => Ok(Self::Utc),
            2 => Ok(Self::MonotonicRelative),
            _ => Err(ClockDisciplineError::InvalidClockScale),
        }
    }
}

/// One step in an authenticated IERS/BIPM TAI-minus-UTC leap-second schedule.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LeapSecondStep {
    /// Effective UTC Unix epoch second (inclusive) when `tai_minus_utc_seconds` takes effect.
    pub effective_utc_second: i64,
    /// Integer `TAI - UTC` offset in seconds (e.g. `37` after 2017-01-01T00:00:00Z).
    pub tai_minus_utc_seconds: i32,
}

/// Authenticated, bounded TAI/UTC leap-second table with validity horizon and clock uncertainty.
/// Per C02-C03, POSIX/UTC-to-TAI mapping requires an identified leap-second table and uncertainty;
/// no floating global epoch or fixed universal UTC offset substitutes for it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LeapSecondTable {
    table_id: SemanticId,
    steps: [LeapSecondStep; MAX_LEAP_SECOND_ENTRIES],
    step_count: usize,
    valid_from_utc_sec: i64,
    valid_until_utc_sec: i64,
    table_uncertainty_seconds: f64,
    table_digest: [u8; 32],
}

impl LeapSecondTable {
    pub fn new(
        table_id: SemanticId,
        steps_slice: &[LeapSecondStep],
        valid_from_utc_sec: i64,
        valid_until_utc_sec: i64,
        table_uncertainty_seconds: f64,
    ) -> Result<Self, ClockDisciplineError> {
        if table_id.value().is_none() {
            return Err(ClockDisciplineError::InvalidIdentifier);
        }
        if steps_slice.is_empty() || steps_slice.len() > MAX_LEAP_SECOND_ENTRIES {
            return Err(ClockDisciplineError::InvalidLeapSecondTable);
        }
        if valid_from_utc_sec >= valid_until_utc_sec
            || !table_uncertainty_seconds.is_finite()
            || table_uncertainty_seconds < 0.0
        {
            return Err(ClockDisciplineError::InvalidLeapSecondTable);
        }
        let mut steps = [LeapSecondStep {
            effective_utc_second: 0,
            tai_minus_utc_seconds: 0,
        }; MAX_LEAP_SECOND_ENTRIES];
        let mut prev_effective: Option<i64> = None;
        for (idx, step) in steps_slice.iter().copied().enumerate() {
            if let Some(prev) = prev_effective {
                if step.effective_utc_second <= prev {
                    return Err(ClockDisciplineError::InvalidLeapSecondTable);
                }
            }
            prev_effective = Some(step.effective_utc_second);
            steps[idx] = step;
        }
        if steps[0].effective_utc_second > valid_from_utc_sec {
            return Err(ClockDisciplineError::InvalidLeapSecondTable);
        }

        let mut hasher = blake3::Hasher::new();
        hasher.update(LEAP_SECOND_TABLE_DOMAIN);
        hasher.update(&table_id.value().unwrap_or(0).to_le_bytes());
        hasher.update(&valid_from_utc_sec.to_le_bytes());
        hasher.update(&valid_until_utc_sec.to_le_bytes());
        hasher.update(&table_uncertainty_seconds.to_bits().to_le_bytes());
        hasher.update(&(steps_slice.len() as u32).to_le_bytes());
        for step in steps_slice {
            hasher.update(&step.effective_utc_second.to_le_bytes());
            hasher.update(&step.tai_minus_utc_seconds.to_le_bytes());
        }
        let table_digest = *hasher.finalize().as_bytes();

        Ok(Self {
            table_id,
            steps,
            step_count: steps_slice.len(),
            valid_from_utc_sec,
            valid_until_utc_sec,
            table_uncertainty_seconds,
            table_digest,
        })
    }

    pub const fn table_id(&self) -> SemanticId {
        self.table_id
    }

    pub const fn table_digest(&self) -> &[u8; 32] {
        &self.table_digest
    }

    pub const fn table_uncertainty_seconds(&self) -> f64 {
        self.table_uncertainty_seconds
    }

    /// Look up `(TAI - UTC)` in integer seconds at `utc_second` within `[valid_from_utc_sec, valid_until_utc_sec)`.
    pub fn tai_minus_utc_at(&self, utc_second: i64) -> Result<i32, ClockDisciplineError> {
        if utc_second < self.valid_from_utc_sec || utc_second >= self.valid_until_utc_sec {
            return Err(ClockDisciplineError::LeapTableExpiredOrOutOfRange);
        }
        let mut active = self.steps[0].tai_minus_utc_seconds;
        for step in self.steps.iter().take(self.step_count) {
            if utc_second >= step.effective_utc_second {
                active = step.tai_minus_utc_seconds;
            } else {
                break;
            }
        }
        Ok(active)
    }

    /// Convert a UTC `RationalInstant` (`scale == 1`) into a TAI `RationalInstant` (`scale == 0`),
    /// returning the converted instant and combined time uncertainty $u_t$ in seconds.
    pub fn convert_utc_instant_to_tai(
        &self,
        utc_instant: RationalInstant,
        tai_origin: SemanticId,
        source_time_uncertainty_seconds: Option<f64>,
    ) -> Result<(RationalInstant, f64), ClockDisciplineError> {
        if ClockScale::from_code(utc_instant.scale())? != ClockScale::Utc {
            return Err(ClockDisciplineError::InvalidClockScale);
        }
        let u_src = validate_known_time_uncertainty(source_time_uncertainty_seconds)?;
        let floor_utc_sec = rational_floor_seconds(utc_instant);
        let floor_i64 = i64::try_from(floor_utc_sec)
            .map_err(|_| ClockDisciplineError::LeapTableExpiredOrOutOfRange)?;
        let delta_tai = self.tai_minus_utc_at(floor_i64)?;
        let shifted = utc_instant
            .add_rational_seconds(i128::from(delta_tai), 1)
            .map_err(|_| ClockDisciplineError::RationalOverflow)?;
        let tai_instant = RationalInstant::new(
            tai_origin,
            utc_instant.restart_epoch(),
            ClockScale::Tai as u32,
            shifted.numerator(),
            shifted.denominator(),
        )
        .map_err(|_| ClockDisciplineError::InvalidIdentifier)?;
        let total_u_t = u_src + self.table_uncertainty_seconds;
        if !total_u_t.is_finite() {
            return Err(ClockDisciplineError::UnknownOrInvalidTimeUncertainty);
        }
        Ok((tai_instant, total_u_t))
    }
}

/// Explicit cross-clock mapping between two distinct clock origins (`source_origin -> target_origin`)
/// over a half-open source validity interval `[valid_from, valid_until)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CrossClockMapping {
    pub mapping_id: SemanticId,
    pub source_origin: SemanticId,
    pub source_restart: u64,
    pub source_scale: u32,
    pub target_origin: SemanticId,
    pub target_restart: u64,
    pub target_scale: u32,
    pub source_validity: CalibrationValidityWindow,
    pub offset_seconds_num: i128,
    pub offset_seconds_den: i128,
    pub mapping_time_uncertainty_seconds: f64,
}

impl CrossClockMapping {
    pub fn validate(&self) -> Result<(), ClockDisciplineError> {
        if self.mapping_id.value().is_none()
            || self.source_origin.value().is_none()
            || self.target_origin.value().is_none()
        {
            return Err(ClockDisciplineError::InvalidIdentifier);
        }
        ClockScale::from_code(self.source_scale)?;
        ClockScale::from_code(self.target_scale)?;
        if self.offset_seconds_den <= 0 {
            return Err(ClockDisciplineError::RationalOverflow);
        }
        if !self.mapping_time_uncertainty_seconds.is_finite()
            || self.mapping_time_uncertainty_seconds < 0.0
        {
            return Err(ClockDisciplineError::UnknownOrInvalidTimeUncertainty);
        }
        if self.source_validity.valid_from().clock_origin() != self.source_origin
            || self.source_validity.valid_from().restart_epoch() != self.source_restart
            || self.source_validity.valid_from().scale() != self.source_scale
        {
            return Err(ClockDisciplineError::CrossClockOriginRejected);
        }
        Ok(())
    }

    /// Map `source_instant` into the target clock domain, returning the mapped `RationalInstant`
    /// and the propagated time uncertainty $u_t = u_{t,\text{source}} + u_{t,\text{mapping}}$.
    pub fn map_instant(
        &self,
        source_instant: RationalInstant,
        source_time_uncertainty_seconds: Option<f64>,
    ) -> Result<(RationalInstant, f64), ClockDisciplineError> {
        self.validate()?;
        let u_src = validate_known_time_uncertainty(source_time_uncertainty_seconds)?;
        self.source_validity
            .verify_contains(source_instant)
            .map_err(|_| ClockDisciplineError::MappingOutsideValidityWindow)?;
        let shifted = source_instant
            .add_rational_seconds(self.offset_seconds_num, self.offset_seconds_den)
            .map_err(|_| ClockDisciplineError::RationalOverflow)?;
        let mapped = RationalInstant::new(
            self.target_origin,
            self.target_restart,
            self.target_scale,
            shifted.numerator(),
            shifted.denominator(),
        )
        .map_err(|_| ClockDisciplineError::InvalidIdentifier)?;
        let total_u = u_src + self.mapping_time_uncertainty_seconds;
        if !total_u.is_finite() {
            return Err(ClockDisciplineError::UnknownOrInvalidTimeUncertainty);
        }
        Ok((mapped, total_u))
    }
}

/// Verify that two `acquisition::Clock` instances share the exact same origin, restart epoch,
/// scale, epoch, and tick rate before admitting an unmapped same-clock join (C02-C03).
///
/// Relative clocks with different origins or restart epochs are strictly rejected even when
/// `epoch_seconds`, `epoch_nanoseconds`, `ticks`, and `samples` are numerically identical.
pub fn verify_same_clock_join(clock_a: Clock, clock_b: Clock) -> Result<(), ClockDisciplineError> {
    if clock_a.origin() != clock_b.origin() || clock_a.restart() != clock_b.restart() {
        return Err(ClockDisciplineError::CrossClockOriginRejected);
    }
    clock_a
        .require_same_domain(clock_b)
        .map_err(|_| ClockDisciplineError::ClockParametersMismatch)
}

/// Compute the sinusoidal phase uncertainty bound $\Delta\phi_{\max} = 2\pi f u_t$ in radians
/// (C02-C03) for a carrier or analysis frequency $f > 0$ Hz and time uncertainty $u_t \ge 0$ s.
///
/// Fails closed if $u_t$ is `None` (unknown), negative, or non-finite, or if $f \le 0$ or non-finite.
pub fn compute_phase_uncertainty_rad(
    frequency_hz: f64,
    time_uncertainty_seconds: Option<f64>,
) -> Result<f64, ClockDisciplineError> {
    if !frequency_hz.is_finite() || frequency_hz <= 0.0 {
        return Err(ClockDisciplineError::InvalidCarrierFrequency);
    }
    let u_t = validate_known_time_uncertainty(time_uncertainty_seconds)?;
    let delta_phi = TAU * frequency_hz * u_t;
    if !delta_phi.is_finite() {
        return Err(ClockDisciplineError::PhaseUncertaintyOverflow);
    }
    Ok(delta_phi)
}

/// Admit a phase-coherent join across RF I/Q (EMF Mode 0x0105) or ultrasound/acoustic array
/// streams (QAC Mode 0x0107) only when the clock domain matches (or is explicitly mapped),
/// time uncertainty $u_t$ is known, and $\Delta\phi_{\max} = 2\pi f u_t \le \Delta\phi_{\text{budget}}$.
pub fn verify_phase_coherent_join(
    instant_a: RationalInstant,
    instant_b: RationalInstant,
    frequency_hz: f64,
    time_uncertainty_seconds: Option<f64>,
    max_allowed_phase_uncertainty_rad: f64,
) -> Result<f64, ClockDisciplineError> {
    if !instant_a.same_clock_domain(instant_b) {
        return Err(ClockDisciplineError::CrossClockOriginRejected);
    }
    if instant_a
        .checked_cmp(instant_b)
        .map_err(|_| ClockDisciplineError::CrossClockOriginRejected)?
        != Ordering::Equal
    {
        return Err(ClockDisciplineError::InstantNotSimultaneous);
    }
    if !max_allowed_phase_uncertainty_rad.is_finite() || max_allowed_phase_uncertainty_rad < 0.0 {
        return Err(ClockDisciplineError::InvalidPhaseCoherenceBudget);
    }
    let delta_phi = compute_phase_uncertainty_rad(frequency_hz, time_uncertainty_seconds)?;
    if delta_phi > max_allowed_phase_uncertainty_rad {
        return Err(ClockDisciplineError::PhaseUncertaintyExceedsBudget);
    }
    Ok(delta_phi)
}

/// Explicit I/Q complex carrier reconstruction conventions (C02-C08, CROSS-A04).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IqCarrierConvention {
    /// EMF Schema 136 / Mode 0x0105 (`"I_plus_iQ_exp_minus_iwt"`):
    /// $x_{\text{RF}}(t) = \operatorname{Re}\{(I + iQ) e^{-i(2\pi f_c(t - t_{\text{LO}}) + \phi_{\text{LO}})}\}$
    /// $= I \cos(\theta) + Q \sin(\theta)$.
    EmfNegativeCarrier,
    /// QAC Schema 208 / Mode 0x0107 (`"I_plus_iQ_exp_plus_iwt"`):
    /// $p(t) = \operatorname{Re}\{(I + iQ) e^{+i(2\pi f_c(t - t_0) + \phi_0)}\}$
    /// $= I \cos(\theta) - Q \sin(\theta)$.
    QacPositiveCarrier,
}

impl IqCarrierConvention {
    pub fn from_wire_str(s: &str) -> Result<Self, ClockDisciplineError> {
        match s {
            "I_plus_iQ_exp_minus_iwt" => Ok(Self::EmfNegativeCarrier),
            "I_plus_iQ_exp_plus_iwt" => Ok(Self::QacPositiveCarrier),
            _ => Err(ClockDisciplineError::UnknownIqCarrierConvention),
        }
    }

    pub const fn as_wire_str(self) -> &'static str {
        match self {
            Self::EmfNegativeCarrier => "I_plus_iQ_exp_minus_iwt",
            Self::QacPositiveCarrier => "I_plus_iQ_exp_plus_iwt",
        }
    }

    pub const fn from_acquisition_convention(conv: ComplexConvention) -> Self {
        match conv {
            ComplexConvention::RealPlusIImagNegativeExponent => Self::EmfNegativeCarrier,
            ComplexConvention::RealPlusIImagPositiveExponent => Self::QacPositiveCarrier,
        }
    }
}

/// Complex baseband sample with explicit carrier convention, carrier frequency, reference phase,
/// and signed baseband frequency offset.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QualifiedIqSample {
    pub i: f64,
    pub q: f64,
    pub convention: IqCarrierConvention,
    pub carrier_hz: f64,
    pub reference_phase_rad: f64,
    pub signed_baseband_offset_hz: f64,
}

impl QualifiedIqSample {
    pub fn validate(&self) -> Result<(), ClockDisciplineError> {
        if !self.i.is_finite()
            || !self.q.is_finite()
            || !self.carrier_hz.is_finite()
            || self.carrier_hz <= 0.0
            || !self.reference_phase_rad.is_finite()
            || !self.signed_baseband_offset_hz.is_finite()
        {
            return Err(ClockDisciplineError::NonFiniteIqParameter);
        }
        Ok(())
    }

    /// Reconstruct the instantaneous real physical waveform value at elapsed time `dt_seconds`
    /// from the sample's phase origin (`theta = 2 * pi * carrier_hz * dt_seconds + reference_phase_rad`):
    /// - `EmfNegativeCarrier`: $\operatorname{Re}((I + iQ) e^{-i\theta}) = I \cos\theta + Q \sin\theta$.
    /// - `QacPositiveCarrier`: $\operatorname{Re}((I + iQ) e^{+i\theta}) = I \cos\theta - Q \sin\theta$.
    pub fn reconstruct_real_at(&self, dt_seconds: f64) -> Result<f64, ClockDisciplineError> {
        self.validate()?;
        if !dt_seconds.is_finite() {
            return Err(ClockDisciplineError::NonFiniteIqParameter);
        }
        let theta = TAU * self.carrier_hz * dt_seconds + self.reference_phase_rad;
        let (sin_t, cos_t) = theta.sin_cos();
        let real = match self.convention {
            IqCarrierConvention::EmfNegativeCarrier => self.i * cos_t + self.q * sin_t,
            IqCarrierConvention::QacPositiveCarrier => self.i * cos_t - self.q * sin_t,
        };
        Ok(real)
    }
}

/// Guard against direct raw `memcpy` / re-labeling of I/Q samples across conventions (C02-C08).
///
/// Attempting to treat an `EmfNegativeCarrier` `(I, Q)` sample as `QacPositiveCarrier` (or vice
/// versa) without conjugation is rejected with `ClockDisciplineError::UnconjugatedCarrierMemcpyRejected`.
pub fn reject_unconjugated_iq_memcpy(
    source_convention: IqCarrierConvention,
    target_convention: IqCarrierConvention,
) -> Result<(), ClockDisciplineError> {
    if source_convention != target_convention {
        return Err(ClockDisciplineError::UnconjugatedCarrierMemcpyRejected);
    }
    Ok(())
}

/// Explicitly convert a `QualifiedIqSample` to `target_convention` and optional `target_phase_origin_shift_seconds`
/// ($\Delta t_0 = t_{0,\text{target}} - t_{0,\text{source}}$) per C02-C08:
///
/// When `source.convention != target_convention`:
/// 1. Conjugates the complex phasor: $z_{\text{conj}} = \overline{z_{\text{src}}} = (I, -Q)$.
/// 2. Negates the reference phase and signed baseband offset so that the physical RF/acoustic
///    frequency $f_{\text{phys}}$ and real reconstructed waveform remain invariant:
///    - With $\phi_{\text{ref}} = 0$ and $\Delta t_0 = 0$: `z_QAC = conj(z_EMF) = (I, -Q)` and
///      `signed_baseband_offset_qac = -signed_baseband_offset_emf`.
/// 3. If `target_phase_origin_shift_seconds != 0.0`, folds the phase-origin shift $\Delta\theta = 2\pi f_c \Delta t_0$
///    into the converted complex phasor so that evaluating at $t - t_{0,\text{target}}$ matches
///    evaluating the source at $t - t_{0,\text{source}}$.
pub fn convert_iq_sample_between_conventions(
    source: QualifiedIqSample,
    target_convention: IqCarrierConvention,
    target_phase_origin_shift_seconds: f64,
) -> Result<QualifiedIqSample, ClockDisciplineError> {
    source.validate()?;
    if !target_phase_origin_shift_seconds.is_finite() {
        return Err(ClockDisciplineError::NonFiniteIqParameter);
    }

    let (mut i_out, mut q_out, mut offset_out) = if source.convention == target_convention {
        (source.i, source.q, source.signed_baseband_offset_hz)
    } else {
        (source.i, -source.q, -source.signed_baseband_offset_hz)
    };

    // Fold source reference_phase_rad and any phase-origin shift into the phasor so the returned
    // sample has reference_phase_rad == 0.0 in the target frame if desired, or preserve reference_phase_rad
    // when target_phase_origin_shift_seconds == 0.0 and reference_phase_rad == 0.0.
    let origin_delta_theta = TAU * source.carrier_hz * target_phase_origin_shift_seconds;
    if origin_delta_theta != 0.0 || (source.convention != target_convention && source.reference_phase_rad != 0.0) {
        // Under target_convention, rotate phasor so Re(z_out * exp(s_target * i * 2pi f_c (t - t_target)))
        // equals Re(z_src * exp(s_src * i * (2pi f_c (t - t_src) + phi_src))).
        let total_phase = match target_convention {
            IqCarrierConvention::EmfNegativeCarrier => {
                // Re((I' + iQ') exp(-i 2pi f_c (t - t_target))) = I' cos(psi) + Q' sin(psi)
                // Equals I_out cos(psi + delta) + Q_out sin(psi + delta) when (I' + iQ') = (I_out + iQ_out) exp(-i delta).
                let phi_eff = if source.convention == target_convention {
                    source.reference_phase_rad
                } else {
                    -source.reference_phase_rad
                };
                -(origin_delta_theta + phi_eff)
            }
            IqCarrierConvention::QacPositiveCarrier => {
                // Re((I' + iQ') exp(+i 2pi f_c (t - t_target))) = I' cos(psi) - Q' sin(psi)
                // Equals I_out cos(psi + delta) - Q_out sin(psi + delta) when (I' + iQ') = (I_out + iQ_out) exp(+i delta).
                let phi_eff = if source.convention == target_convention {
                    source.reference_phase_rad
                } else {
                    source.reference_phase_rad
                };
                origin_delta_theta + phi_eff
            }
        };
        let (sin_p, cos_p) = total_phase.sin_cos();
        let i_rot = i_out * cos_p - q_out * sin_p;
        let q_rot = i_out * sin_p + q_out * cos_p;
        i_out = i_rot;
        q_out = q_rot;
        if offset_out == -0.0 {
            offset_out = 0.0;
        }
        return Ok(QualifiedIqSample {
            i: i_out,
            q: q_out,
            convention: target_convention,
            carrier_hz: source.carrier_hz,
            reference_phase_rad: 0.0,
            signed_baseband_offset_hz: offset_out,
        });
    }

    Ok(QualifiedIqSample {
        i: i_out,
        q: q_out,
        convention: target_convention,
        carrier_hz: source.carrier_hz,
        reference_phase_rad: source.reference_phase_rad,
        signed_baseband_offset_hz: offset_out,
    })
}

fn validate_known_time_uncertainty(u_t: Option<f64>) -> Result<f64, ClockDisciplineError> {
    match u_t {
        Some(v) if v.is_finite() && v >= 0.0 => Ok(v),
        _ => Err(ClockDisciplineError::UnknownOrInvalidTimeUncertainty),
    }
}

fn rational_floor_seconds(instant: RationalInstant) -> i128 {
    let n = instant.numerator();
    let d = instant.denominator();
    if n >= 0 {
        n / d
    } else {
        (n - d + 1) / d
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClockDisciplineError {
    InvalidIdentifier,
    InvalidClockScale,
    InvalidLeapSecondTable,
    LeapTableExpiredOrOutOfRange,
    RationalOverflow,
    UnknownOrInvalidTimeUncertainty,
    CrossClockOriginRejected,
    ClockParametersMismatch,
    MappingOutsideValidityWindow,
    InstantNotSimultaneous,
    InvalidCarrierFrequency,
    InvalidPhaseCoherenceBudget,
    PhaseUncertaintyOverflow,
    PhaseUncertaintyExceedsBudget,
    UnknownIqCarrierConvention,
    NonFiniteIqParameter,
    UnconjugatedCarrierMemcpyRejected,
}
