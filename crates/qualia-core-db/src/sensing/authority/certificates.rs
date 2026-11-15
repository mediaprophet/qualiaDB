//! Calibration certificate chains, half-open rational epoch validity (`[valid_from, valid_until)`),
//! single-application affine calibration rules (C02-C05), and explicit uncertainty target/unit/kind
//! verification for EMF v4 (Schema 107) and QAC v1 (Schema 202 / Schema 210).

use core::cmp::Ordering;
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};

use crate::audio::qac::signal::{ExactTime, Rate};
use crate::sensing::acquisition::Clock;
use crate::sensing::SemanticId;

/// Domain separator for BLAKE3 calibration descriptor digests (Schema 107 / Schema 202).
pub const CALIBRATION_CERT_DOMAIN: &[u8] = b"QUALIA-CALIBRATION-CERT-v1\0";
/// Domain separator for Ed25519 signatures over calibration certificates.
pub const CALIBRATION_SIG_DOMAIN: &[u8] = b"QUALIA-CALIBRATION-SIG-v1\0";
/// Maximum links in a verified calibration certificate chain (root metrology -> working sensor).
pub const MAX_CALIBRATION_CHAIN_LEN: usize = 8;
/// Maximum samples per zero-heap calibrated batch slice check.
pub const MAX_CALIBRATION_BATCH_SAMPLES: usize = 4096;

/// Target calibration schema family.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u16)]
pub enum CalibrationSchemaFamily {
    /// EMF v4 Schema 107 (`Calibration`).
    EmfSchema107 = 107,
    /// QAC v1 Schema 202 (`Calibration`).
    QacSchema202 = 202,
}

/// Signal encoding stage relation (Schema 210 key 5 and C02-C05).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SignalStageRelation {
    /// `"digital-unrelated-to-pressure"` — numeric scaling never implies physical units.
    DigitalUnrelatedToPressure,
    /// `"raw-counts"` — uncalibrated ADC/sensor counts eligible for one affine calibration.
    RawCounts,
    /// `"physical-base-units"` — already in physical SI units (`scale == 1.0, offset == 0.0`);
    /// calibration certificate is retained strictly as lineage and never re-applied.
    PhysicalBaseUnits,
}

impl SignalStageRelation {
    pub fn from_wire_str(label: &str) -> Result<Self, CalibrationError> {
        match label {
            "digital-unrelated-to-pressure" => Ok(Self::DigitalUnrelatedToPressure),
            "raw-counts" => Ok(Self::RawCounts),
            "physical-base-units" => Ok(Self::PhysicalBaseUnits),
            _ => Err(CalibrationError::InvalidStageRelation),
        }
    }

    pub const fn as_wire_str(self) -> &'static str {
        match self {
            Self::DigitalUnrelatedToPressure => "digital-unrelated-to-pressure",
            Self::RawCounts => "raw-counts",
            Self::PhysicalBaseUnits => "physical-base-units",
        }
    }
}

/// Metrological uncertainty kind (C02-C05, EMF-A05, AQ-07).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum UncertaintyKind {
    /// 1-sigma or standard measurement uncertainty in the target unit.
    StandardAbsolute,
    /// Expanded uncertainty with explicit coverage factor $k > 0$ and confidence probability in $(0, 1)$.
    ExpandedAbsolute {
        coverage_factor_k: f64,
        confidence_probability: f64,
    },
    /// Dimensionless relative uncertainty $\ge 0$.
    Relative,
}

impl UncertaintyKind {
    pub fn validate(self) -> Result<(), CalibrationError> {
        match self {
            Self::StandardAbsolute | Self::Relative => Ok(()),
            Self::ExpandedAbsolute {
                coverage_factor_k,
                confidence_probability,
            } => {
                if !coverage_factor_k.is_finite()
                    || coverage_factor_k <= 0.0
                    || !confidence_probability.is_finite()
                    || confidence_probability <= 0.0
                    || confidence_probability >= 1.0
                {
                    return Err(CalibrationError::InvalidUncertaintyKind);
                }
                Ok(())
            }
        }
    }
}

/// Qualified uncertainty declaration binding a finite nonnegative value to an explicit
/// target quantity, unit, and metrological uncertainty kind.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QualifiedUncertainty {
    pub value: f64,
    pub target_quantity: SemanticId,
    pub target_unit: SemanticId,
    pub kind: UncertaintyKind,
    pub estimator_id: SemanticId,
}

impl QualifiedUncertainty {
    pub fn validate(&self) -> Result<(), CalibrationError> {
        if !self.value.is_finite() || self.value < 0.0 {
            return Err(CalibrationError::InvalidUncertaintyValue);
        }
        self.kind.validate()?;
        if self.target_quantity.value().is_none() || self.target_unit.value().is_none() {
            return Err(CalibrationError::UnlabeledUncertaintyRejected);
        }
        if self.estimator_id.value().is_none() {
            return Err(CalibrationError::InvalidIdentifier);
        }
        Ok(())
    }

    /// Enforce C02-C05 / EMF-A05 on mixed-unit records (EMF Mode 0x0106 ionizing radiation
    /// and Mode 0x0107 bioelectric dipole): one scalar uncertainty lane must explicitly name
    /// which component quantity and unit it qualifies, and that pair must belong to the
    /// record's admitted `(quantity, unit)` pairs.
    pub fn verify_against_mixed_record_components(
        &self,
        admitted_components: &[(SemanticId, SemanticId)],
    ) -> Result<(), CalibrationError> {
        self.validate()?;
        if admitted_components.is_empty() {
            return Err(CalibrationError::UnlabeledUncertaintyRejected);
        }
        let found = admitted_components.iter().any(|&(q, u)| {
            q == self.target_quantity
                && u == self.target_unit
                && q.value().is_some()
                && u.value().is_some()
        });
        if !found {
            return Err(CalibrationError::UncertaintyTargetMismatch);
        }
        Ok(())
    }
}

/// Exact clock instant using reduced signed `i128` rational arithmetic (`numerator / denominator`
/// seconds from the clock scale's epoch), bound to an explicit clock origin and restart epoch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RationalInstant {
    clock_origin: SemanticId,
    restart_epoch: u64,
    scale: u32,
    numerator: i128,
    denominator: i128,
}

impl RationalInstant {
    /// Construct a reduced rational instant from raw `(numerator, denominator)` seconds
    /// in the specified `(clock_origin, restart_epoch, scale)` domain.
    pub fn new(
        clock_origin: SemanticId,
        restart_epoch: u64,
        scale: u32,
        numerator: i128,
        denominator: i128,
    ) -> Result<Self, CalibrationError> {
        if clock_origin.value().is_none() {
            return Err(CalibrationError::InvalidIdentifier);
        }
        if scale > 2 || denominator <= 0 {
            return Err(CalibrationError::InvalidClockInstant);
        }
        let abs_num = numerator
            .checked_abs()
            .ok_or(CalibrationError::RationalOverflow)? as u128;
        let g = gcd_u128(abs_num, denominator as u128) as i128;
        Ok(Self {
            clock_origin,
            restart_epoch,
            scale,
            numerator: numerator / g,
            denominator: denominator / g,
        })
    }

    /// Construct a `RationalInstant` from an `acquisition::Clock`, integer `tick`, and
    /// Schema 5 rational subtick `(subtick_num, subtick_den)` where `0 <= subtick_num < subtick_den`.
    pub fn from_clock_tick_subtick(
        clock: Clock,
        tick: i64,
        subtick_num: u64,
        subtick_den: u64,
    ) -> Result<Self, CalibrationError> {
        if subtick_den == 0 || subtick_num >= subtick_den {
            return Err(CalibrationError::InvalidClockInstant);
        }
        let base: ExactTime = clock
            .tick_time(tick)
            .map_err(|_| CalibrationError::InvalidClockInstant)?;
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
            .ok_or(CalibrationError::RationalOverflow)?;
        let sub_sec_den = i128::from(subtick_den)
            .checked_mul(i128::from(ticks_rate.numerator))
            .ok_or(CalibrationError::RationalOverflow)?;
        base_instant.add_rational_seconds(sub_sec_num, sub_sec_den)
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

    /// Add `delta_num / delta_den` seconds using exact checked `i128` rational arithmetic.
    pub fn add_rational_seconds(
        self,
        delta_num: i128,
        delta_den: i128,
    ) -> Result<Self, CalibrationError> {
        if delta_den <= 0 {
            return Err(CalibrationError::InvalidClockInstant);
        }
        let g = gcd_u128(self.denominator as u128, delta_den as u128) as i128;
        let a = delta_den / g;
        let b = self.denominator / g;
        let left = self
            .numerator
            .checked_mul(a)
            .ok_or(CalibrationError::RationalOverflow)?;
        let right = delta_num
            .checked_mul(b)
            .ok_or(CalibrationError::RationalOverflow)?;
        let sum_num = left
            .checked_add(right)
            .ok_or(CalibrationError::RationalOverflow)?;
        let lcm_den = self
            .denominator
            .checked_mul(a)
            .ok_or(CalibrationError::RationalOverflow)?;
        Self::new(
            self.clock_origin,
            self.restart_epoch,
            self.scale,
            sum_num,
            lcm_den,
        )
    }

    /// Exact comparison between two `RationalInstant`s on the same clock domain.
    pub fn checked_cmp(self, other: Self) -> Result<Ordering, CalibrationError> {
        if !self.same_clock_domain(other) {
            return Err(CalibrationError::ClockDomainMismatch);
        }
        if self.denominator == other.denominator {
            return Ok(self.numerator.cmp(&other.numerator));
        }
        let g = gcd_u128(self.denominator as u128, other.denominator as u128) as i128;
        let lhs = self
            .numerator
            .checked_mul(other.denominator / g)
            .ok_or(CalibrationError::RationalOverflow)?;
        let rhs = other
            .numerator
            .checked_mul(self.denominator / g)
            .ok_or(CalibrationError::RationalOverflow)?;
        Ok(lhs.cmp(&rhs))
    }
}

/// Half-open calibration validity window `[valid_from, valid_until)` (C02-C03).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CalibrationValidityWindow {
    valid_from: RationalInstant,
    valid_until: RationalInstant,
}

impl CalibrationValidityWindow {
    /// Construct a half-open validity interval `[valid_from, valid_until)`.
    pub fn new(
        valid_from: RationalInstant,
        valid_until: RationalInstant,
    ) -> Result<Self, CalibrationError> {
        if valid_from.checked_cmp(valid_until)? != Ordering::Less {
            return Err(CalibrationError::InvalidValidityInterval);
        }
        Ok(Self {
            valid_from,
            valid_until,
        })
    }

    pub const fn valid_from(self) -> RationalInstant {
        self.valid_from
    }
    pub const fn valid_until(self) -> RationalInstant {
        self.valid_until
    }

    /// Strictly admit `instant` iff `valid_from <= instant < valid_until` (zero tolerance).
    /// Rejects at `instant == valid_until` with `CalibrationError::CalibrationExpiredAtOrAfterUntil`.
    pub fn verify_contains(self, instant: RationalInstant) -> Result<(), CalibrationError> {
        if instant.checked_cmp(self.valid_from)? == Ordering::Less {
            return Err(CalibrationError::CalibrationNotYetValid);
        }
        if instant.checked_cmp(self.valid_until)? != Ordering::Less {
            return Err(CalibrationError::CalibrationExpiredAtOrAfterUntil);
        }
        Ok(())
    }

    /// Verify `parent.valid_from <= child.valid_from` and `child.valid_until <= parent.valid_until`.
    pub fn verify_subinterval_of(self, parent: Self) -> Result<(), CalibrationError> {
        if self.valid_from.checked_cmp(parent.valid_from)? == Ordering::Less
            || self.valid_until.checked_cmp(parent.valid_until)? == Ordering::Greater
        {
            return Err(CalibrationError::ChildWindowExceedsIssuer);
        }
        Ok(())
    }
}

/// Signed calibration certificate covering EMF Schema 107 or QAC Schema 202.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CalibrationCertificate {
    pub schema_family: CalibrationSchemaFamily,
    pub certificate_id: [u8; 32],
    pub parent_certificate_digest: Option<[u8; 32]>,
    pub instrument_id: SemanticId,
    pub channel_index: u32,
    pub input_quantity: SemanticId,
    pub output_quantity: SemanticId,
    pub output_unit: SemanticId,
    pub affine_scale: f64,
    pub affine_offset: f64,
    pub reference_level: Option<f64>,
    pub frequency_band_hz: Option<[f64; 2]>,
    pub validity_window: CalibrationValidityWindow,
    pub uncertainty: QualifiedUncertainty,
    pub issuer_public_key: [u8; 32],
    pub subject_public_key: [u8; 32],
    pub signature: [u8; 64],
}

impl CalibrationCertificate {
    pub fn validate_fields(&self) -> Result<(), CalibrationError> {
        if self.certificate_id == [0u8; 32]
            || self.issuer_public_key == [0u8; 32]
            || self.subject_public_key == [0u8; 32]
            || self.instrument_id.value().is_none()
            || self.input_quantity.value().is_none()
            || self.output_quantity.value().is_none()
            || self.output_unit.value().is_none()
        {
            return Err(CalibrationError::InvalidIdentifier);
        }
        if self.parent_certificate_digest.is_some_and(|p| p == [0u8; 32]) {
            return Err(CalibrationError::InvalidIdentifier);
        }
        if !self.affine_scale.is_finite()
            || self.affine_scale <= 0.0
            || !self.affine_offset.is_finite()
        {
            return Err(CalibrationError::InvalidAffineParameters);
        }
        if self.reference_level.is_some_and(|r| !r.is_finite() || r <= 0.0) {
            return Err(CalibrationError::InvalidReferenceLevel);
        }
        if let Some([f_lo, f_hi]) = self.frequency_band_hz {
            if !f_lo.is_finite() || !f_hi.is_finite() || f_lo < 0.0 || f_lo >= f_hi {
                return Err(CalibrationError::InvalidFrequencyBand);
            }
        }
        self.uncertainty.validate()?;
        if self.uncertainty.target_quantity != self.output_quantity
            || self.uncertainty.target_unit != self.output_unit
        {
            return Err(CalibrationError::UncertaintyTargetMismatch);
        }
        Ok(())
    }

    pub fn sign(mut self, issuer_signing_key: &SigningKey) -> Result<Self, CalibrationError> {
        self.issuer_public_key = issuer_signing_key.verifying_key().to_bytes();
        self.validate_fields()?;
        let preimage = self.signing_preimage();
        self.signature = issuer_signing_key.sign(&preimage).to_bytes();
        Ok(self)
    }

    pub fn certificate_digest(&self) -> [u8; 32] {
        let mut hasher = blake3::Hasher::new();
        hasher.update(CALIBRATION_CERT_DOMAIN);
        self.hash_body(&mut hasher);
        hasher.update(&self.signature);
        *hasher.finalize().as_bytes()
    }

    pub fn verify_signature(&self) -> Result<(), CalibrationError> {
        self.validate_fields()?;
        let vk = VerifyingKey::from_bytes(&self.issuer_public_key)
            .map_err(|_| CalibrationError::InvalidSignature)?;
        let sig = Signature::from_bytes(&self.signature);
        vk.verify_strict(&self.signing_preimage(), &sig)
            .map_err(|_| CalibrationError::InvalidSignature)
    }

    pub fn verify_applicable_at(
        &self,
        instant: RationalInstant,
        frequency_hz: Option<f64>,
    ) -> Result<(), CalibrationError> {
        self.verify_signature()?;
        self.validity_window.verify_contains(instant)?;
        if let (Some([f_lo, f_hi]), Some(f)) = (self.frequency_band_hz, frequency_hz) {
            if !f.is_finite() || f < f_lo || f > f_hi {
                return Err(CalibrationError::FrequencyOutOfCalibrationBand);
            }
        }
        Ok(())
    }

    fn signing_preimage(&self) -> [u8; 32] {
        let mut hasher = blake3::Hasher::new();
        hasher.update(CALIBRATION_SIG_DOMAIN);
        self.hash_body(&mut hasher);
        *hasher.finalize().as_bytes()
    }

    fn hash_body(&self, hasher: &mut blake3::Hasher) {
        hasher.update(&(self.schema_family as u16).to_le_bytes());
        hasher.update(&self.certificate_id);
        match self.parent_certificate_digest {
            Some(parent) => {
                hasher.update(&[1]);
                hasher.update(&parent);
            }
            None => {
                hasher.update(&[0; 33]);
            }
        }
        hasher.update(&self.instrument_id.value().unwrap_or(0).to_le_bytes());
        hasher.update(&self.channel_index.to_le_bytes());
        hasher.update(&self.input_quantity.value().unwrap_or(0).to_le_bytes());
        hasher.update(&self.output_quantity.value().unwrap_or(0).to_le_bytes());
        hasher.update(&self.output_unit.value().unwrap_or(0).to_le_bytes());
        hasher.update(&self.affine_scale.to_bits().to_le_bytes());
        hasher.update(&self.affine_offset.to_bits().to_le_bytes());
        match self.reference_level {
            Some(r) => {
                hasher.update(&[1]);
                hasher.update(&r.to_bits().to_le_bytes());
            }
            None => {
                hasher.update(&[0; 9]);
            }
        }
        match self.frequency_band_hz {
            Some([lo, hi]) => {
                hasher.update(&[1]);
                hasher.update(&lo.to_bits().to_le_bytes());
                hasher.update(&hi.to_bits().to_le_bytes());
            }
            None => {
                hasher.update(&[0; 17]);
            }
        }
        hash_instant(hasher, self.validity_window.valid_from);
        hash_instant(hasher, self.validity_window.valid_until);
        hasher.update(&self.uncertainty.value.to_bits().to_le_bytes());
        hasher.update(&self.uncertainty.target_quantity.value().unwrap_or(0).to_le_bytes());
        hasher.update(&self.uncertainty.target_unit.value().unwrap_or(0).to_le_bytes());
        hasher.update(&self.uncertainty.estimator_id.value().unwrap_or(0).to_le_bytes());
        match self.uncertainty.kind {
            UncertaintyKind::StandardAbsolute => {
                hasher.update(&[1]);
            }
            UncertaintyKind::ExpandedAbsolute {
                coverage_factor_k,
                confidence_probability,
            } => {
                hasher.update(&[2]);
                hasher.update(&coverage_factor_k.to_bits().to_le_bytes());
                hasher.update(&confidence_probability.to_bits().to_le_bytes());
            }
            UncertaintyKind::Relative => {
                hasher.update(&[3]);
            }
        }
        hasher.update(&self.issuer_public_key);
        hasher.update(&self.subject_public_key);
    }
}

/// Verify an ordered certificate chain `[root, ..., leaf]` (`1 <= chain.len() <= MAX_CALIBRATION_CHAIN_LEN`)
/// anchored at `trusted_root_public_key` and valid at `observation_instant`.
pub fn verify_calibration_certificate_chain(
    chain: &[CalibrationCertificate],
    trusted_root_public_key: &[u8; 32],
    observation_instant: RationalInstant,
    frequency_hz: Option<f64>,
) -> Result<[u8; 32], CalibrationError> {
    if chain.is_empty() || chain.len() > MAX_CALIBRATION_CHAIN_LEN {
        return Err(CalibrationError::InvalidChainLength);
    }
    let root = &chain[0];
    if &root.issuer_public_key != trusted_root_public_key || root.parent_certificate_digest.is_some()
    {
        return Err(CalibrationError::UntrustedRootAuthority);
    }
    root.verify_signature()?;
    root.validity_window.verify_contains(observation_instant)?;

    let mut prev_digest = root.certificate_digest();
    let mut prev_subject_key = root.subject_public_key;
    let mut prev_window = root.validity_window;

    for cert in &chain[1..] {
        cert.verify_signature()?;
        if cert.parent_certificate_digest != Some(prev_digest) {
            return Err(CalibrationError::BrokenParentDigestLink);
        }
        if cert.issuer_public_key != prev_subject_key {
            return Err(CalibrationError::BrokenIssuerSubjectLink);
        }
        cert.validity_window.verify_subinterval_of(prev_window)?;
        cert.validity_window.verify_contains(observation_instant)?;
        prev_digest = cert.certificate_digest();
        prev_subject_key = cert.subject_public_key;
        prev_window = cert.validity_window;
    }

    chain[chain.len() - 1].verify_applicable_at(observation_instant, frequency_hz)?;
    Ok(prev_digest)
}

#[path = "certificates_calibration.rs"]
mod certificates_calibration;
pub use certificates_calibration::*;


fn hash_instant(hasher: &mut blake3::Hasher, instant: RationalInstant) {
    hasher.update(&instant.clock_origin.value().unwrap_or(0).to_le_bytes());
    hasher.update(&instant.restart_epoch.to_le_bytes());
    hasher.update(&instant.scale.to_le_bytes());
    hasher.update(&instant.numerator.to_le_bytes());
    hasher.update(&instant.denominator.to_le_bytes());
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
