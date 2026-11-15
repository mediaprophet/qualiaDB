//! Q07 Mode `0x0107` (`ultrasound_beam_summary`, 64 B) & Mode `0x0105` (`raw_RF` / `baseband_IQ`, `Schema 208`).
//!
//! Preserves raw transducer channel acquisition, array geometry, element delays, calibration,
//! and derived beam summaries (`MI`, `ISPTA`, context-qualified `TI`, Doppler axial velocity,
//! and positive-carrier IQ `Re{(I + iQ) exp(+i(2*pi*fc*t + phi0))}`).

use super::types::*;
pub use super::ultrasound_codec::*;

/// Frozen wire stride in bytes for `mode 0x0107` ultrasound beam summary records.
pub const ULTRASOUND_SUMMARY_STRIDE: u32 = 64;

/// Bitmask constants for `UltrasoundBeamSummaryRecord::validity_bits` (`0..=6`).
pub const US_VALID_PR: u32 = 1 << 0;
pub const US_VALID_ISPTA: u32 = 1 << 1;
pub const US_VALID_MI: u32 = 1 << 2;
pub const US_VALID_TI: u32 = 1 << 3;
pub const US_VALID_IQ: u32 = 1 << 4;
pub const US_VALID_AXIAL_VEL: u32 = 1 << 5;
pub const US_VALID_HARMONIC: u32 = 1 << 6;
pub const US_VALID_ALL: u32 = US_VALID_PR
    | US_VALID_ISPTA
    | US_VALID_MI
    | US_VALID_TI
    | US_VALID_IQ
    | US_VALID_AXIAL_VEL
    | US_VALID_HARMONIC;

/// Explicit role of an ultrasound entry recorded in `Schema 208` key 1 (`role`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UltrasoundRole {
    RawRf,
    BasebandIq,
    BeamSummary,
    TransmitCommand,
}

impl UltrasoundRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RawRf => "raw-RF",
            Self::BasebandIq => "baseband-IQ",
            Self::BeamSummary => "beam-summary",
            Self::TransmitCommand => "transmit-command",
        }
    }

    pub fn parse(s: &str) -> Result<Self, ProfileError> {
        match s {
            "raw-RF" => Ok(Self::RawRf),
            "baseband-IQ" => Ok(Self::BasebandIq),
            "beam-summary" => Ok(Self::BeamSummary),
            "transmit-command" => Ok(Self::TransmitCommand),
            _ => Err(ProfileError::InvalidMetadata("unknown Schema 208 ultrasound role")),
        }
    }
}

/// Optional linked raw RF or baseband-IQ transducer acquisition channels (`mode 0x0105`).
#[derive(Debug, Clone)]
pub enum RawUltrasoundPayload<'a> {
    RawRfF32 {
        channels: u32,
        sample_rate: RationalRate,
        samples_interleaved: &'a [f32],
    },
    BasebandIqComplexF32 {
        channels: u32,
        sample_rate: RationalRate,
        iq_interleaved: &'a [[f32; 2]],
    },
}

/// Complete `Schema 208` acquisition, array geometry, derating, and thermal-context specification.
#[derive(Debug, Clone)]
pub struct UltrasoundAcquisitionSpec<'a> {
    pub role: UltrasoundRole,
    pub center_frequency_hz: Option<f64>,
    pub element_positions_m: Option<&'a [[f32; 3]]>,
    pub channel_map: Option<&'a [u32]>,
    pub transmit_delays_s: Option<&'a [f64]>,
    pub demodulation_phase_rad: Option<f64>,
    pub prf: Option<RationalRate>,
    pub derating_model_uri: Option<&'a str>,
    pub exposure_context: Option<&'a str>,
    pub thermal_model_uri: Option<&'a str>,
    pub instrument_digest: Option<[u8; 32]>,
    pub authority_receipt_digest: Option<[u8; 32]>,
    pub target_frame: Option<FrameSpec<'a>>,
    pub calibration: Option<CalibrationSpec<'a>>,
}

/// Canonical 64-byte Q07 `mode 0x0107` ultrasound beam summary record.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UltrasoundBeamSummaryRecord {
    pub position_m: [f32; 3],
    pub center_frequency_hz: f32,
    pub peak_rarefactional_pressure_pa: f32,
    pub intensity_ispta_w_m2: f32,
    pub mechanical_index: f32,
    pub thermal_index: f32,
    pub complex_envelope_iq: [f32; 2],
    pub axial_velocity_m_s: f32,
    pub harmonic_ratio_linear: f32,
    pub validity_bits: u32,
    pub acquisition_ref: u32,
}

impl UltrasoundBeamSummaryRecord {
    pub fn validate(&self, externally_valid: bool) -> Result<(), ProfileError> {
        for v in self.position_m {
            if !v.is_finite() {
                return Err(ProfileError::NonFiniteFloat);
            }
            require_zero_when_masked(externally_valid, v)?;
        }
        if !self.center_frequency_hz.is_finite() {
            return Err(ProfileError::NonFiniteFloat);
        }
        if externally_valid {
            if self.center_frequency_hz <= 0.0 {
                return Err(ProfileError::InvalidMetadata("ultrasound center_frequency_hz must be > 0"));
            }
        } else if self.center_frequency_hz.to_bits() != 0 || self.validity_bits != 0 {
            return Err(ProfileError::MaskedLaneNonZero);
        }

        let floats = [
            self.peak_rarefactional_pressure_pa,
            self.intensity_ispta_w_m2,
            self.mechanical_index,
            self.thermal_index,
            self.complex_envelope_iq[0],
            self.complex_envelope_iq[1],
            self.axial_velocity_m_s,
            self.harmonic_ratio_linear,
        ];
        if floats.iter().any(|v| !v.is_finite()) {
            return Err(ProfileError::NonFiniteFloat);
        }
        if self.validity_bits & !US_VALID_ALL != 0 {
            return Err(ProfileError::InvalidValidityBits);
        }
        if self.acquisition_ref == 0 {
            return Err(ProfileError::InvalidMetadata("acquisition_ref must be nonzero Schema 208 ID"));
        }

        let b = self.validity_bits;
        let check_nonneg = |bit: u32, val: f32, msg: &'static str| -> Result<(), ProfileError> {
            let valid = externally_valid && (b & bit != 0);
            require_zero_when_masked(valid, val)?;
            if valid && val < 0.0 {
                return Err(ProfileError::InvalidMetadata(msg));
            }
            Ok(())
        };
        check_nonneg(US_VALID_PR, self.peak_rarefactional_pressure_pa, "negative peak rarefactional pressure")?;
        check_nonneg(US_VALID_ISPTA, self.intensity_ispta_w_m2, "negative ISPTA")?;
        check_nonneg(US_VALID_MI, self.mechanical_index, "negative mechanical_index")?;
        check_nonneg(US_VALID_TI, self.thermal_index, "negative thermal_index")?;
        let iq_valid = externally_valid && (b & US_VALID_IQ != 0);
        require_zero_when_masked(iq_valid, self.complex_envelope_iq[0])?;
        require_zero_when_masked(iq_valid, self.complex_envelope_iq[1])?;
        let vel_valid = externally_valid && (b & US_VALID_AXIAL_VEL != 0);
        require_zero_when_masked(vel_valid, self.axial_velocity_m_s)?;
        check_nonneg(US_VALID_HARMONIC, self.harmonic_ratio_linear, "negative harmonic_ratio_linear")?;
        Ok(())
    }

    pub fn decode(bytes: &[u8], externally_valid: bool) -> Result<Self, ProfileError> {
        if bytes.len() != ULTRASOUND_SUMMARY_STRIDE as usize {
            return Err(ProfileError::InvalidPayloadLength);
        }
        ensure_zero_bytes(&bytes[56..64])?;
        let rec = Self {
            position_m: read_f32_3(bytes, 0)?,
            center_frequency_hz: read_f32_finite(bytes, 12)?,
            peak_rarefactional_pressure_pa: read_f32_finite(bytes, 16)?,
            intensity_ispta_w_m2: read_f32_finite(bytes, 20)?,
            mechanical_index: read_f32_finite(bytes, 24)?,
            thermal_index: read_f32_finite(bytes, 28)?,
            complex_envelope_iq: read_f32_2(bytes, 32)?,
            axial_velocity_m_s: read_f32_finite(bytes, 40)?,
            harmonic_ratio_linear: read_f32_finite(bytes, 44)?,
            validity_bits: read_u32(bytes, 48)?,
            acquisition_ref: read_u32(bytes, 52)?,
        };
        rec.validate(externally_valid)?;
        Ok(rec)
    }

    pub fn encode_into(&self, out: &mut [u8], externally_valid: bool) -> Result<(), ProfileError> {
        if out.len() != ULTRASOUND_SUMMARY_STRIDE as usize {
            return Err(ProfileError::InvalidPayloadLength);
        }
        self.validate(externally_valid)?;
        out.fill(0);
        write_f32_slice(out, 0, &self.position_m)?;
        write_f32_finite(out, 12, self.center_frequency_hz)?;
        write_f32_finite(out, 16, self.peak_rarefactional_pressure_pa)?;
        write_f32_finite(out, 20, self.intensity_ispta_w_m2)?;
        write_f32_finite(out, 24, self.mechanical_index)?;
        write_f32_finite(out, 28, self.thermal_index)?;
        write_f32_slice(out, 32, &self.complex_envelope_iq)?;
        write_f32_finite(out, 40, self.axial_velocity_m_s)?;
        write_f32_finite(out, 44, self.harmonic_ratio_linear)?;
        write_u32(out, 48, self.validity_bits);
        write_u32(out, 52, self.acquisition_ref);
        Ok(())
    }
}

/// Zero-heap borrowed view over verified `mode 0x0107` ultrasound beam summary records.
pub struct UltrasoundChunkView<'a> {
    bytes: &'a [u8],
    mask: Option<&'a [u8]>,
}

impl<'a> UltrasoundChunkView<'a> {
    pub(crate) fn new(bytes: &'a [u8], mask: Option<&'a [u8]>) -> Self {
        Self { bytes, mask }
    }

    pub fn len(&self) -> usize {
        self.bytes.len() / ULTRASOUND_SUMMARY_STRIDE as usize
    }

    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    pub fn get(&self, index: usize) -> Result<UltrasoundBeamSummaryRecord, ProfileError> {
        let start = index
            .checked_mul(ULTRASOUND_SUMMARY_STRIDE as usize)
            .ok_or(ProfileError::Overflow)?;
        let end = start
            .checked_add(ULTRASOUND_SUMMARY_STRIDE as usize)
            .ok_or(ProfileError::Overflow)?;
        let slice = self
            .bytes
            .get(start..end)
            .ok_or(ProfileError::OutOfBoundsIndex("ultrasound record index"))?;
        let ext_valid = self.mask.map_or(true, |m| m[index] != 0);
        UltrasoundBeamSummaryRecord::decode(slice, ext_valid)
    }
}

// ---------------------------------------------------------------------------
// Acoustic physics helpers: FDA MI, positive-carrier IQ, Doppler, ToF delays
// ---------------------------------------------------------------------------

/// Standard derated Mechanical Index `MI = p_{r,derated}(MPa) / sqrt(f_c(MHz))`.
pub fn mechanical_index_derated(
    peak_rarefactional_pa: f64,
    center_frequency_hz: f64,
    depth_cm: f64,
    derating_db_per_cm_mhz: f64,
) -> Result<f64, ProfileError> {
    if !peak_rarefactional_pa.is_finite()
        || !center_frequency_hz.is_finite()
        || !depth_cm.is_finite()
        || !derating_db_per_cm_mhz.is_finite()
    {
        return Err(ProfileError::NonFiniteFloat);
    }
    if peak_rarefactional_pa < 0.0 || center_frequency_hz <= 0.0 || depth_cm < 0.0 || derating_db_per_cm_mhz < 0.0 {
        return Err(ProfileError::InvalidMetadata("invalid MI parameters"));
    }
    let f_mhz = center_frequency_hz / 1.0e6;
    let p_derated_mpa = (peak_rarefactional_pa / 1.0e6)
        * 10.0f64.powf(-(derating_db_per_cm_mhz * depth_cm * f_mhz) / 20.0);
    Ok(p_derated_mpa / f_mhz.sqrt())
}

/// Positive-carrier RF reconstruction from baseband IQ:
/// `p_RF(t) = Re{ (I + iQ) * exp(+i * (2*pi*fc*t + phi0)) } = I*cos(th) - Q*sin(th)`.
pub fn modulate_positive_carrier_rf(
    i_val: f64,
    q_val: f64,
    center_frequency_hz: f64,
    demod_phase_rad: f64,
    time_s: f64,
) -> f64 {
    let theta = 2.0 * std::f64::consts::PI * center_frequency_hz * time_s + demod_phase_rad;
    i_val * theta.cos() - q_val * theta.sin()
}

/// Coherent baseband IQ demodulation matching positive-carrier convention over `rf_samples`.
pub fn demodulate_positive_carrier_iq(
    rf_samples: &[f64],
    sample_rate_hz: f64,
    center_frequency_hz: f64,
    demod_phase_rad: f64,
) -> Result<[f64; 2], ProfileError> {
    if rf_samples.is_empty() || !sample_rate_hz.is_finite() || sample_rate_hz <= 0.0 || !center_frequency_hz.is_finite() || center_frequency_hz <= 0.0 {
        return Err(ProfileError::InvalidMetadata("invalid IQ demodulation parameters"));
    }
    let mut sum_i = 0.0f64;
    let mut sum_q = 0.0f64;
    for (n, &x) in rf_samples.iter().enumerate() {
        if !x.is_finite() {
            return Err(ProfileError::NonFiniteFloat);
        }
        let t = (n as f64) / sample_rate_hz;
        let theta = 2.0 * std::f64::consts::PI * center_frequency_hz * t + demod_phase_rad;
        // x(t) * 2 * exp(-i*theta) recovers (I + i*Q) when carrier is +i*theta.
        sum_i += 2.0 * x * theta.cos();
        sum_q += -2.0 * x * theta.sin();
    }
    let scale = 1.0 / (rf_samples.len() as f64);
    Ok([sum_i * scale, sum_q * scale])
}

/// Signed Doppler axial velocity `v_axial = c * f_D / (2 * f_c)` (positive toward transducer).
pub fn doppler_axial_velocity_m_s(
    doppler_shift_hz: f64,
    center_frequency_hz: f64,
    sound_speed_m_s: f64,
) -> Result<f64, ProfileError> {
    if !doppler_shift_hz.is_finite() || !center_frequency_hz.is_finite() || !sound_speed_m_s.is_finite() {
        return Err(ProfileError::NonFiniteFloat);
    }
    if center_frequency_hz <= 0.0 || sound_speed_m_s <= 0.0 {
        return Err(ProfileError::InvalidMetadata("frequency and speed of sound must be > 0"));
    }
    Ok((sound_speed_m_s * doppler_shift_hz) / (2.0 * center_frequency_hz))
}

/// Round-trip time-of-flight delay `(||r_f - r_tx|| + ||r_f - e_k||) / c` in seconds.
pub fn element_roundtrip_delay_s(
    tx_origin_m: [f64; 3],
    element_pos_m: [f64; 3],
    focus_pos_m: [f64; 3],
    sound_speed_m_s: f64,
) -> Result<f64, ProfileError> {
    if sound_speed_m_s <= 0.0
        || tx_origin_m
            .iter()
            .chain(element_pos_m.iter())
            .chain(focus_pos_m.iter())
            .chain(std::iter::once(&sound_speed_m_s))
            .any(|v| !v.is_finite())
    {
        return Err(ProfileError::InvalidMetadata("invalid beamforming ToF geometry"));
    }
    let dist = |a: [f64; 3], b: [f64; 3]| -> f64 {
        let dx = a[0] - b[0];
        let dy = a[1] - b[1];
        let dz = a[2] - b[2];
        (dx * dx + dy * dy + dz * dz).sqrt()
    };
    Ok((dist(focus_pos_m, tx_origin_m) + dist(focus_pos_m, element_pos_m)) / sound_speed_m_s)
}
