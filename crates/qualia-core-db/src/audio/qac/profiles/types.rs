//! Shared types, constants, coordinate transforms, exact time helpers, and errors for QAC v1 domain profiles.
//!
//! Covers canonical QAC v1 mode identifiers across packets Q03–Q07 alongside
//! preservation/auxiliary modes, little-endian record wire helpers, Hamilton
//! quaternion admission (`<= 1e-12`), tensor rotations, and checked rational instants.

use std::fmt;

use crate::q42::{field_envelope as env, field_semantics as sem};

// ---------------------------------------------------------------------------
// Canonical QAC v1 mode constants & historical packet alignment note
// ---------------------------------------------------------------------------

/// Q03: Continuum pressure perturbation and particle velocity sample (64 B).
///
/// Reconciled wire-freeze assignment: early playbook drafts numbered this `0x0100`,
/// which collided with dense PCM preservation (`0x0100`). Wire-freeze assigned
/// continuum modes to `0x0000`–`0x0001` while `0x0100`–`0x0101` remain PCM/STFT.
pub const MODE_PRESSURE_VELOCITY: u16 = 0x0000;
/// Q03: Continuum full 6-component Cauchy stress tensor and velocity state (64 B).
pub const MODE_ELASTIC_STRESS_VELOCITY: u16 = 0x0001;
/// Q05: Higher-Order Ambisonics temporal sample frame (`4 * (L + 1)^2` B, `L = 0..=3`).
pub const MODE_HOA_SPATIAL: u16 = 0x0102;
/// Q05: Head-Related / Room Impulse Response FIR taps (`taps * 4` B).
pub const MODE_HRIR_IMPULSE: u16 = 0x0103;
/// Q05: Acoustic ray / path contribution record (64 B, paired with schema 206).
pub const MODE_RAY_CONTRIBUTION: u16 = 0x0003;
/// Q06: Damped modal resonance oscillator row (32 B, disambiguated from `0x0104` opaque by schema 206 + stride 32).
pub const MODE_MODAL_RESONANCE: u16 = 0x0104;
/// Q06: Perceptual auditory filterbank features (`4 * feature_count` B, disambiguated from RF by schema 203).
pub const MODE_PERCEPTUAL_FEATURES: u16 = 0x0105;
/// Q07: Haptic mechanical sample (64 B, disambiguated from validity mask by schema 207 + stride 64).
pub const MODE_HAPTIC_SAMPLE: u16 = 0x0106;
/// Q07: Ultrasound / sonar beam summary (64 B, disambiguated from aux array by schema 208 + stride 64).
pub const MODE_ULTRASOUND_SUMMARY: u16 = 0x0107;

/// Shared / Q02 dense PCM preservation (`channels * scalar_bytes`).
pub const MODE_PCM_PRESERVATION: u16 = 0x0100;
/// Shared / Q02 dense STFT/CQT preservation (`channels * bins * scalar_bytes`).
pub const MODE_STFT_PRESERVATION: u16 = 0x0101;
/// Shared opaque original byte preservation (`stride = 1`).
pub const MODE_OPAQUE_SOURCE: u16 = 0x0104;
/// Q07 raw RF or baseband-IQ ultrasound acquisition frame (`channels * scalar_bytes`).
pub const MODE_RAW_RF_IQ: u16 = 0x0105;
/// Shared validity companion byte mask (`stride = 1`, values `0 | 1`).
pub const MODE_VALIDITY_MASK: u16 = 0x0106;
/// Shared typed multidimensional auxiliary array (`channels * scalar_bytes`).
pub const MODE_AUX_ARRAY: u16 = 0x0107;

/// Base Cartesian frame convention for QAC v1 profiles.
pub const BASE_FRAME_CONVENTION: &str = "right-handed-X-forward-Y-left-Z-up";
/// Base HRIR/RIR delay reference string.
pub const BASE_DELAY_REFERENCE: &str = "relative-to-excitation-origin";
/// Legacy unknown delay reference permitted only with Schema 209 retention.
pub const LEGACY_DELAY_REFERENCE: &str = "legacy-unknown";

/// Default quaternion squared-norm tolerance (`abs(|q|^2 - 1) <= 1e-12`).
pub const QUATERNION_NORM_TOLERANCE: f64 = 1e-12;
/// Ray unit-direction squared-norm tolerance (`abs(|d|^2 - 1) <= 1e-5`).
pub const DIRECTION_NORM_TOLERANCE: f64 = 1e-5;
/// Rational clock numerator ceiling (`10^15`).
pub const MAX_CLOCK_NUM: u64 = 1_000_000_000_000_000;
/// Rational clock denominator ceiling (`10^12`).
pub const MAX_CLOCK_DEN: u64 = 1_000_000_000_000;

/// Scalar representation codes for Schema 3 key 11 (`scalar`).
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuxScalar {
    F32 = 1,
    F64 = 2,
    I16 = 3,
    I24 = 4,
    I32 = 5,
    ComplexF32 = 6,
    U8 = 7,
}

impl AuxScalar {
    pub const fn byte_size(self) -> u32 {
        match self {
            Self::F32 | Self::I32 => 4,
            Self::F64 | Self::ComplexF32 => 8,
            Self::I16 => 2,
            Self::I24 => 3,
            Self::U8 => 1,
        }
    }

    pub const fn from_u32(value: u32) -> Option<Self> {
        match value {
            1 => Some(Self::F32),
            2 => Some(Self::F64),
            3 => Some(Self::I16),
            4 => Some(Self::I24),
            5 => Some(Self::I32),
            6 => Some(Self::ComplexF32),
            7 => Some(Self::U8),
            _ => None,
        }
    }
}

/// Explicit physical/model status codes replacing synthetic zeros or `-inf`/`+inf`.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SampleStatus {
    Valid = 0,
    Missing = 1,
    Unknown = 2,
    Saturated = 3,
    Silence = 4,
    Unvoiced = 5,
    ModelUnavailable = 6,
}

impl SampleStatus {
    pub const fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Valid),
            1 => Some(Self::Missing),
            2 => Some(Self::Unknown),
            3 => Some(Self::Saturated),
            4 => Some(Self::Silence),
            5 => Some(Self::Unvoiced),
            6 => Some(Self::ModelUnavailable),
            _ => None,
        }
    }
}

/// Disambiguated QAC v1 profile kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileMode {
    PressureVelocity,
    ElasticStressVelocity,
    HoaSpatial,
    HrirImpulse,
    RayContribution,
    ModalResonance,
    PerceptualFeatures,
    HapticSample,
    UltrasoundSummary,
    RawRfIq,
    OpaqueSource,
    ValidityMask,
    AuxArray,
}

/// Bounded limits controlling decoding, scratch size, and record checks before allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProfileLimits {
    pub envelope: env::Limits,
    pub semantics: sem::Limits,
    pub max_records_per_chunk: u64,
    pub max_channels: u32,
    pub max_taps: u32,
}

impl Default for ProfileLimits {
    fn default() -> Self {
        Self {
            envelope: env::Limits::default(),
            semantics: sem::Limits::default(),
            max_records_per_chunk: 1_048_576,
            max_channels: 1024,
            max_taps: 1_048_576,
        }
    }
}

/// Unified typed error for all QAC v1 profile codecs, metadata validators, and physics guards.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfileError {
    Envelope(env::Error),
    Semantics(sem::Error),
    Overflow,
    LimitExceeded,
    InvalidMode(u16),
    InvalidStride { expected: u32, actual: u32 },
    InvalidPayloadLength,
    NonZeroPadding,
    NonFiniteFloat,
    InvalidValidityBits,
    MaskedLaneNonZero,
    InvalidQuaternion,
    InvalidUnitVector,
    InvalidOrdering,
    InvalidTimeSpec,
    ClockScaleMismatch,
    MediumMissingOrUnknown(&'static str),
    NonCollocatedProduct(&'static str),
    UnsupportedConstitutiveModel(&'static str),
    InvalidMetadata(&'static str),
    MissingRequiredSchema(u16),
    OutOfBoundsIndex(&'static str),
    ZeroDecayRateT60Undefined,
    InvalidDomainConversion(&'static str),
    ActuationAuthority(&'static str),
    MissingThermalModel(&'static str),
    LegacyFormat(&'static str),
}

impl From<env::Error> for ProfileError {
    fn from(value: env::Error) -> Self {
        Self::Envelope(value)
    }
}

impl From<sem::Error> for ProfileError {
    fn from(value: sem::Error) -> Self {
        Self::Semantics(value)
    }
}

impl fmt::Display for ProfileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Envelope(e) => write!(f, "envelope error: {e:?}"),
            Self::Semantics(e) => write!(f, "semantics error: {e:?}"),
            Self::Overflow => write!(f, "checked integer overflow"),
            Self::LimitExceeded => write!(f, "configured profile budget exceeded"),
            Self::InvalidMode(m) => write!(f, "invalid QAC payload mode 0x{m:04x}"),
            Self::InvalidStride { expected, actual } => {
                write!(f, "invalid stride: expected {expected}, got {actual}")
            }
            Self::InvalidPayloadLength => write!(f, "payload length disagrees with record count * stride"),
            Self::NonZeroPadding => write!(f, "non-zero reserved bytes or padding"),
            Self::NonFiniteFloat => write!(f, "non-finite floating-point value (NaN or Inf)"),
            Self::InvalidValidityBits => write!(f, "unknown reserved validity bits set"),
            Self::MaskedLaneNonZero => write!(f, "masked/invalid lane does not contain canonical +0.0"),
            Self::InvalidQuaternion => write!(f, "invalid or unnormalized Hamilton quaternion"),
            Self::InvalidUnitVector => write!(f, "unit direction vector norm tolerance exceeded"),
            Self::InvalidOrdering => write!(f, "entries or metadata blocks are not strictly sorted"),
            Self::InvalidTimeSpec => write!(f, "invalid rational time or clock specification"),
            Self::ClockScaleMismatch => write!(f, "cannot compare instants across distinct time scales"),
            Self::MediumMissingOrUnknown(msg) => write!(f, "medium error: {msg}"),
            Self::NonCollocatedProduct(msg) => write!(f, "non-collocated product prohibited: {msg}"),
            Self::UnsupportedConstitutiveModel(msg) => write!(f, "unsupported constitutive model: {msg}"),
            Self::InvalidMetadata(msg) => write!(f, "invalid profile metadata: {msg}"),
            Self::MissingRequiredSchema(s) => write!(f, "missing required metadata schema {s}"),
            Self::OutOfBoundsIndex(msg) => write!(f, "out-of-bounds record index: {msg}"),
            Self::ZeroDecayRateT60Undefined => write!(f, "finite T60 undefined when decay_rate == 0"),
            Self::InvalidDomainConversion(msg) => write!(f, "prohibited domain conflation: {msg}"),
            Self::ActuationAuthority(msg) => write!(f, "actuation authority/expiry failure: {msg}"),
            Self::MissingThermalModel(msg) => write!(f, "missing thermal context/model: {msg}"),
            Self::LegacyFormat(msg) => write!(f, "legacy Q4AU error: {msg}"),
        }
    }
}

impl std::error::Error for ProfileError {}

// ---------------------------------------------------------------------------
// Checked little-endian wire primitives
// ---------------------------------------------------------------------------

#[inline]
pub fn ensure_zero_bytes(bytes: &[u8]) -> Result<(), ProfileError> {
    if bytes.iter().all(|&b| b == 0) {
        Ok(())
    } else {
        Err(ProfileError::NonZeroPadding)
    }
}

#[inline]
pub fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, ProfileError> {
    let slice = bytes
        .get(offset..offset + 2)
        .ok_or(ProfileError::InvalidPayloadLength)?;
    Ok(u16::from_le_bytes([slice[0], slice[1]]))
}

#[inline]
pub fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, ProfileError> {
    let slice = bytes
        .get(offset..offset + 4)
        .ok_or(ProfileError::InvalidPayloadLength)?;
    Ok(u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

#[inline]
pub fn read_u64(bytes: &[u8], offset: usize) -> Result<u64, ProfileError> {
    let slice = bytes
        .get(offset..offset + 8)
        .ok_or(ProfileError::InvalidPayloadLength)?;
    Ok(u64::from_le_bytes([
        slice[0], slice[1], slice[2], slice[3], slice[4], slice[5], slice[6], slice[7],
    ]))
}

#[inline]
pub fn read_f32_finite(bytes: &[u8], offset: usize) -> Result<f32, ProfileError> {
    let slice = bytes
        .get(offset..offset + 4)
        .ok_or(ProfileError::InvalidPayloadLength)?;
    let val = f32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]);
    if !val.is_finite() {
        return Err(ProfileError::NonFiniteFloat);
    }
    Ok(val)
}

#[inline]
pub fn read_f32_3(bytes: &[u8], offset: usize) -> Result<[f32; 3], ProfileError> {
    Ok([
        read_f32_finite(bytes, offset)?,
        read_f32_finite(bytes, offset + 4)?,
        read_f32_finite(bytes, offset + 8)?,
    ])
}

#[inline]
pub fn read_f32_2(bytes: &[u8], offset: usize) -> Result<[f32; 2], ProfileError> {
    Ok([
        read_f32_finite(bytes, offset)?,
        read_f32_finite(bytes, offset + 4)?,
    ])
}

#[inline]
pub fn read_f32_6(bytes: &[u8], offset: usize) -> Result<[f32; 6], ProfileError> {
    Ok([
        read_f32_finite(bytes, offset)?,
        read_f32_finite(bytes, offset + 4)?,
        read_f32_finite(bytes, offset + 8)?,
        read_f32_finite(bytes, offset + 12)?,
        read_f32_finite(bytes, offset + 16)?,
        read_f32_finite(bytes, offset + 20)?,
    ])
}

#[inline]
pub fn write_u16(out: &mut [u8], offset: usize, val: u16) {
    out[offset..offset + 2].copy_from_slice(&val.to_le_bytes());
}

#[inline]
pub fn write_u32(out: &mut [u8], offset: usize, val: u32) {
    out[offset..offset + 4].copy_from_slice(&val.to_le_bytes());
}

#[inline]
pub fn write_u64(out: &mut [u8], offset: usize, val: u64) {
    out[offset..offset + 8].copy_from_slice(&val.to_le_bytes());
}

#[inline]
pub fn write_f32_finite(out: &mut [u8], offset: usize, val: f32) -> Result<(), ProfileError> {
    if !val.is_finite() {
        return Err(ProfileError::NonFiniteFloat);
    }
    out[offset..offset + 4].copy_from_slice(&val.to_le_bytes());
    Ok(())
}

#[inline]
pub fn write_f32_slice(out: &mut [u8], offset: usize, vals: &[f32]) -> Result<(), ProfileError> {
    for (idx, &v) in vals.iter().enumerate() {
        write_f32_finite(out, offset + idx * 4, v)?;
    }
    Ok(())
}

/// Enforces canonical `+0.0` bits (`0x0000_0000`) when a numeric lane is invalid/masked.
#[inline]
pub fn require_zero_when_masked(valid: bool, value: f32) -> Result<(), ProfileError> {
    if !valid && value.to_bits() != 0 {
        return Err(ProfileError::MaskedLaneNonZero);
    }
    Ok(())
}

#[inline]
pub fn require_zero_vec3_when_masked(valid: bool, v: [f32; 3]) -> Result<(), ProfileError> {
    for c in v {
        require_zero_when_masked(valid, c)?;
    }
    Ok(())
}

#[inline]
pub fn require_finite_f64(val: f64) -> Result<f64, ProfileError> {
    if val.is_finite() {
        Ok(val)
    } else {
        Err(ProfileError::NonFiniteFloat)
    }
}

#[inline]
pub fn has_control_chars(s: &str) -> bool {
    s.chars()
        .any(|c| matches!(c as u32, 0x00..=0x1f | 0x7f..=0x9f))
}

#[inline]
pub fn validate_identifier_str(s: &str, name: &'static str) -> Result<(), ProfileError> {
    if s.is_empty() || has_control_chars(s) {
        return Err(ProfileError::InvalidMetadata(name));
    }
    Ok(())
}

pub use super::geometry_time::*;
