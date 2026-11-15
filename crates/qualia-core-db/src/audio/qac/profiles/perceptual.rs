//! Q06 Mode `0x0105`: Perceptual auditory features (`4 * feature_count` B, `Schema 203`).
//!
//! Enforces strict unit/scale separation (`dB-SPL`, `sone`, `LUFS`, `mel`, `MFCC`, ERB/Bark/Gammatone),
//! explicit filterbank definitions (`centers_hz`, FIR `coefficients`, integration, `reference_pressure_pa`),
//! ITU-R BS.1770-4/5 gating profiles separate from SPL, non-infinite silence handling, and
//! raw source / legacy `Q4AU v1/v2` feature preservation (`Schema 209`).

use std::f64::consts::PI;

use crate::audio::audio_spectral_sheet::{
    parse_sidecar_header, AudioSpectralSidecarHeader, SpectralV2SubHeader,
    SPECTRAL_V2_SUBHEADER_MAGIC,
};

use super::envelope::*;
use super::ray_rir::push_aux_f32_entry;
use super::types::*;

#[path = "perceptual_helpers.rs"]
mod perceptual_helpers;
pub use perceptual_helpers::*;

pub const REFERENCE_PRESSURE_AIR_PA: f64 = 20.0e-6;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PerceptualScale {
    LinearEnergy,
    LogPower,
    DbSpl,
    Mel,
    Mfcc,
    Sone,
    Lufs,
    GammatoneEnergy,
    ErbEnergy,
    BarkEnergy,
}

impl PerceptualScale {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::LinearEnergy => "linear-energy",
            Self::LogPower => "log-power",
            Self::DbSpl => "dB-SPL",
            Self::Mel => "mel",
            Self::Mfcc => "MFCC",
            Self::Sone => "sone",
            Self::Lufs => "LUFS",
            Self::GammatoneEnergy => "gammatone-energy",
            Self::ErbEnergy => "erb-energy",
            Self::BarkEnergy => "bark-energy",
        }
    }

    pub const fn expected_units(self) -> &'static str {
        match self {
            Self::DbSpl => "dB-SPL",
            Self::Sone => "sone",
            Self::Lufs => "LUFS",
            Self::Mel => "mel",
            _ => "1",
        }
    }

    pub fn parse(s: &str) -> Result<Self, ProfileError> {
        match s {
            "linear-energy" => Ok(Self::LinearEnergy),
            "log-power" => Ok(Self::LogPower),
            "dB-SPL" => Ok(Self::DbSpl),
            "mel" => Ok(Self::Mel),
            "MFCC" => Ok(Self::Mfcc),
            "sone" => Ok(Self::Sone),
            "LUFS" => Ok(Self::Lufs),
            "gammatone-energy" => Ok(Self::GammatoneEnergy),
            "erb-energy" => Ok(Self::ErbEnergy),
            "bark-energy" => Ok(Self::BarkEnergy),
            _ => Err(ProfileError::InvalidMetadata("perceptual scale")),
        }
    }
}

// ---------------------------------------------------------------------------
// Auditory scale conversions & FIR Gammatone / BS.1770 / SPL oracles
// ---------------------------------------------------------------------------

/// Glasberg & Moore (1990) ERB bandwidth in Hz at center frequency `f_hz >= 0`.
#[inline]
pub fn erb_bandwidth_hz(f_hz: f64) -> f64 {
    24.7 * (4.37 * (f_hz / 1000.0) + 1.0)
}

/// Glasberg & Moore ERB-rate number scale (`Cams`).
#[inline]
pub fn hz_to_erb_number(f_hz: f64) -> f64 {
    21.4 * (4.37e-3 * f_hz + 1.0).log10()
}

/// Zwicker analytical Bark scale conversion.
#[inline]
pub fn hz_to_bark(f_hz: f64) -> f64 {
    13.0 * (0.00076 * f_hz).atan() + 3.5 * ((f_hz / 7500.0).powi(2)).atan()
}

/// HTK Mel conversion (`2595 * log10(1 + f / 700)`).
#[inline]
pub fn hz_to_mel_htk(f_hz: f64) -> f64 {
    2595.0 * (1.0 + f_hz / 700.0).log10()
}

#[inline]
pub fn mel_to_hz_htk(mel: f64) -> f64 {
    700.0 * (10.0f64.powf(mel / 2595.0) - 1.0)
}

/// Synthesizes a unity-peak-gain 4th-order real Gammatone FIR filterbank impulse response
/// (`h[n]`) at `center_hz` and `sample_rate_hz`.
pub fn gammatone_fir_impulse(
    center_hz: f64,
    sample_rate_hz: f64,
    taps: usize,
) -> Result<Vec<f32>, ProfileError> {
    if !center_hz.is_finite()
        || !sample_rate_hz.is_finite()
        || center_hz <= 0.0
        || center_hz >= 0.5 * sample_rate_hz
        || taps == 0
    {
        return Err(ProfileError::InvalidMetadata("gammatone filter parameters"));
    }
    let b = 1.019 * erb_bandwidth_hz(center_hz);
    let two_pi_b = 2.0 * PI * b;
    let two_pi_f = 2.0 * PI * center_hz;
    let mut raw = Vec::with_capacity(taps);
    for n in 0..taps {
        let t = (n as f64) / sample_rate_hz;
        let env = t.powi(3) * (-two_pi_b * t).exp();
        raw.push(env * (two_pi_f * t).cos());
    }
    // Normalize gain so DC-free passband response at center_hz equals 1.0 (DTFT magnitude).
    let mag_fc = dtft_magnitude(&raw, center_hz / sample_rate_hz);
    if !mag_fc.is_finite() || mag_fc <= 0.0 {
        return Err(ProfileError::InvalidMetadata("gammatone normalization"));
    }
    Ok(raw.into_iter().map(|v| (v / mag_fc) as f32).collect())
}

/// Evaluates DTFT magnitude `|sum_n h[n] * exp(-i * 2*pi*nu*n)|` at normalized freq `nu = f/fs`.
pub fn dtft_magnitude(ir: &[f64], normalized_freq: f64) -> f64 {
    let mut re = 0.0f64;
    let mut im = 0.0f64;
    let w = 2.0 * PI * normalized_freq;
    for (n, &h) in ir.iter().enumerate() {
        let ang = w * (n as f64);
        re += h * ang.cos();
        im -= h * ang.sin();
    }
    (re * re + im * im).sqrt()
}

/// Exact dB-SPL from finite RMS acoustic pressure `rms_pa >= 0` (`p_ref > 0`).
/// Returns `Err(SampleStatus::Silence)` on exact zero RMS instead of `-inf`!
pub fn rms_pa_to_db_spl(rms_pa: f64, reference_pa: f64) -> Result<f64, SampleStatus> {
    if !rms_pa.is_finite() || rms_pa < 0.0 || !reference_pa.is_finite() || reference_pa <= 0.0 {
        return Err(SampleStatus::Unknown);
    }
    if rms_pa == 0.0 {
        return Err(SampleStatus::Silence);
    }
    Ok(20.0 * (rms_pa / reference_pa).log10())
}

/// ITU-R BS.1770-4/5 two-stage gated integrated loudness in LUFS (`-70 LKFS` absolute, `-10 LU` relative).
/// `block_mean_squares` holds `[channel]` mean-square energies per 400 ms gating block.
pub fn bs1770_integrated_lufs(
    block_mean_squares: &[&[f64]],
    channel_weights: &[f64],
) -> Result<f64, SampleStatus> {
    if block_mean_squares.is_empty() || channel_weights.is_empty() {
        return Err(SampleStatus::Silence);
    }
    let ch = channel_weights.len();
    let mut gating_powers = Vec::with_capacity(block_mean_squares.len());
    for &blk in block_mean_squares {
        if blk.len() != ch || blk.iter().any(|v| !v.is_finite() || *v < 0.0) {
            return Err(SampleStatus::Unknown);
        }
        let sum_w: f64 = blk.iter().zip(channel_weights).map(|(&z, &g)| g * z).sum();
        if sum_w <= 0.0 {
            continue;
        }
        let lj = -0.691 + 10.0 * sum_w.log10();
        if lj > -70.0 {
            gating_powers.push((lj, blk));
        }
    }
    if gating_powers.is_empty() {
        return Err(SampleStatus::Silence);
    }
    let mut mean_ch = vec![0.0f64; ch];
    for &(_, blk) in &gating_powers {
        for (dst, &z) in mean_ch.iter_mut().zip(blk.iter()) {
            *dst += z;
        }
    }
    let n_abs = gating_powers.len() as f64;
    let abs_sum: f64 = mean_ch
        .iter()
        .zip(channel_weights)
        .map(|(&m, &g)| g * (m / n_abs))
        .sum();
    if abs_sum <= 0.0 {
        return Err(SampleStatus::Silence);
    }
    let gamma_r = -0.691 + 10.0 * abs_sum.log10() - 10.0;

    let mut rel_mean = vec![0.0f64; ch];
    let mut n_rel = 0usize;
    for &(lj, blk) in &gating_powers {
        if lj > gamma_r {
            for (dst, &z) in rel_mean.iter_mut().zip(blk.iter()) {
                *dst += z;
            }
            n_rel += 1;
        }
    }
    if n_rel == 0 {
        return Err(SampleStatus::Silence);
    }
    let rel_sum: f64 = rel_mean
        .iter()
        .zip(channel_weights)
        .map(|(&m, &g)| g * (m / (n_rel as f64)))
        .sum();
    if rel_sum <= 0.0 {
        return Err(SampleStatus::Silence);
    }
    Ok(-0.691 + 10.0 * rel_sum.log10())
}

/// Descriptor configuration for `mode 0x0105` perceptual feature containers.
#[derive(Debug, Clone)]
pub struct PerceptualWriteSpec<'a> {
    pub algorithm_uri: &'a str,
    pub algorithm_version: &'a str,
    pub scale: PerceptualScale,
    pub feature_count: u32,
    pub feature_names: &'a str,
    pub centers_hz: Option<&'a [f32]>,
    pub filter_coefficients: Option<&'a [f32]>,
    pub reference_pressure_pa: Option<f64>,
    pub channel_weights: Option<&'a [f32]>,
    pub gating_profile: Option<&'a str>,
    pub frame_rate: RationalRate,
}

/// Writes canonical QAC v1 `mode 0x0105` perceptual feature stream (`Schema 203` + `205`).
pub fn write_perceptual_features_qac(
    frames: &[&[f32]],
    spec: &PerceptualWriteSpec<'_>,
    clock: &ClockSpec,
    provenance: &ProvenanceSpec<'_>,
    validity_mask: Option<&[u8]>,
    limits: ProfileLimits,
) -> Result<Vec<u8>, ProfileError> {
    if frames.is_empty() || spec.feature_count == 0 {
        return Err(ProfileError::InvalidPayloadLength);
    }
    if spec.scale == PerceptualScale::DbSpl && spec.reference_pressure_pa.is_none() {
        return Err(ProfileError::InvalidMetadata("dB-SPL requires reference_pressure_pa"));
    }
    if spec.scale == PerceptualScale::Lufs
        && spec.gating_profile.map_or(true, |g| g.is_empty() || g == "none")
    {
        return Err(ProfileError::InvalidMetadata("LUFS requires explicit BS.1770 gating_profile"));
    }
    if let Some(m) = validity_mask {
        if m.len() != frames.len() || m.iter().any(|&b| b > 1) {
            return Err(ProfileError::InvalidValidityBits);
        }
    }
    let width = spec.feature_count as usize;
    let stride = width * 4;
    let mut main_payload = vec![0u8; frames.len() * stride];
    for (idx, row) in frames.iter().enumerate() {
        if row.len() != width {
            return Err(ProfileError::InvalidPayloadLength);
        }
        let ext_valid = validity_mask.map_or(true, |m| m[idx] != 0);
        for (c, &v) in row.iter().enumerate() {
            if !v.is_finite() {
                return Err(ProfileError::NonFiniteFloat);
            }
            require_zero_when_masked(ext_valid, v)?;
            write_f32_finite(&mut main_payload, idx * stride + c * 4, v)?;
        }
    }

    let mut meta = MetadataBuilder::new();
    meta.push_clock(1, clock)?;
    meta.push_provenance(3, provenance)?;

    // Canonical directory order on stream_id=1:
    // Entry 1: mode 0x0105 (desc 6)
    // Entry 2 (optional mask): mode 0x0106 (desc 7)
    // Aux entries (desc 8 centers, desc 9 coeffs, desc 10 weights): mode 0x0107
    let mut next_idx = 2u32;
    let mask_idx = if validity_mask.is_some() {
        let idx = next_idx;
        next_idx += 1;
        Some(idx)
    } else {
        None
    };

    let mut s203_fields = vec![
        OwnedField::crit(1, OwnedValue::Utf8(spec.algorithm_uri.to_owned())),
        OwnedField::crit(2, OwnedValue::Utf8(spec.algorithm_version.to_owned())),
        OwnedField::crit(3, OwnedValue::U32(vec![spec.feature_count])),
        OwnedField::crit(4, OwnedValue::Utf8(spec.scale.as_str().to_owned())),
    ];
    if let Some(p0) = spec.reference_pressure_pa {
        s203_fields.push(OwnedField::opt(8, OwnedValue::F64(vec![p0])));
    }
    if let Some(gp) = spec.gating_profile {
        s203_fields.push(OwnedField::opt(10, OwnedValue::Utf8(gp.to_owned())));
    }
    if !spec.feature_names.is_empty() {
        s203_fields.push(OwnedField::opt(11, OwnedValue::Utf8(spec.feature_names.to_owned())));
    }

    let mut entries = vec![OwnedEntry {
        mode: MODE_PERCEPTUAL_FEATURES,
        flags: 3,
        stream_id: 1,
        frame_id: 0,
        start_tick: 0,
        time_count: frames.len() as u64,
        spatial_key: 0,
        record_count: frames.len() as u64,
        stride: stride as u32,
        descriptor_id: 6,
        payload: main_payload,
    }];
    if let Some(mb) = validity_mask {
        entries.push(push_validity_mask_entry(
            &mut meta,
            7,
            3,
            1,
            0,
            frames.len() as u64,
            mb,
        )?);
    }

    if let Some(centers) = spec.centers_hz {
        s203_fields.push(OwnedField::opt(5, OwnedValue::EntryRefs(vec![next_idx])));
        next_idx += 1;
        let mut e = push_aux_f32_entry(&mut meta, 8, 3, "filterbank-centers-hz", "Hz", 1, AuxScalar::F32, centers)?;
        e.stream_id = 1;
        entries.push(e);
    }
    if let Some(coeffs) = spec.filter_coefficients {
        s203_fields.push(OwnedField::opt(6, OwnedValue::EntryRefs(vec![next_idx])));
        next_idx += 1;
        let mut e = push_aux_f32_entry(&mut meta, 9, 3, "filterbank-coefficients", "1", 1, AuxScalar::F32, coeffs)?;
        e.stream_id = 1;
        entries.push(e);
    }
    if let Some(cw) = spec.channel_weights {
        s203_fields.push(OwnedField::opt(9, OwnedValue::EntryRefs(vec![next_idx])));
        let mut e = push_aux_f32_entry(&mut meta, 10, 3, "loudness-channel-weights", "1", 1, AuxScalar::F32, cw)?;
        e.stream_id = 1;
        entries.push(e);
    }

    meta.push_block(OwnedBlock {
        id: 4,
        schema_type: 203,
        revision: 1,
        critical: true,
        fields: s203_fields,
    })?;
    let shape = [frames.len() as u64, u64::from(spec.feature_count)];
    meta.push_model_grid(
        5,
        &ModelGridSpec {
            model_uri: spec.algorithm_uri,
            model_version: spec.algorithm_version,
            discretization: Some("frame-feature-vector"),
            component_offsets_xyz: &[],
            component_time_offset_num: &[],
            component_time_offset_den: &[],
            timestep: Some(spec.frame_rate),
            tolerance_abs: None,
            tolerance_rel: None,
            tensor_shape: &shape,
            component_names: spec.feature_names,
        },
    )?;

    let valid_count = validity_mask.map(|m| m.iter().filter(|&&b| b == 1).count() as u64);
    let ext = [4u32, 5u32];
    meta.push_descriptor(
        6,
        &DescriptorSpec {
            clock_id: Some(1),
            frame_id: None,
            provenance_id: 3,
            medium_id: None,
            transform_id: None,
            calibration_id: None,
            quantity: "perceptual-features",
            units: spec.scale.expected_units(),
            channels: Some(1),
            layout: Some("frame-feature"),
            scalar: Some(AuxScalar::F32),
            model_uri: "urn:qac:mode:0x0105:v1",
            sample_rate: Some(spec.frame_rate),
            valid_record_count: valid_count,
            validity_mask_entry: mask_idx,
            source_entries: &[],
            extension_metadata: &ext,
        },
    )?;
    assemble_qac_v1(&meta, &entries, limits)
}
