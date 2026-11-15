//! Single-application calibration series state and error definitions (C02-C05).

use super::{
    CalibrationCertificate, MAX_CALIBRATION_BATCH_SAMPLES, RationalInstant, SignalStageRelation,
};
use crate::sensing::SemanticId;

/// Calibrated physical value state that records its `SignalStageRelation`, active
/// stored scaling (`1.0`, `0.0` once in `PhysicalBaseUnits`), and lineage certificate digest.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CalibratedSeriesState<const N: usize> {
    stage: SignalStageRelation,
    quantity: SemanticId,
    unit: SemanticId,
    units_per_stored_sample: f64,
    stored_offset: f64,
    lineage_certificate_digest: Option<[u8; 32]>,
    samples: [f64; N],
}

impl<const N: usize> CalibratedSeriesState<N> {
    /// Construct an uncalibrated `RawCounts` series (`units_per_stored_sample = 1.0`, `stored_offset = 0.0`).
    pub fn from_raw_counts(
        raw_quantity: SemanticId,
        raw_unit: SemanticId,
        samples: [f64; N],
    ) -> Result<Self, CalibrationError> {
        if N == 0 || N > MAX_CALIBRATION_BATCH_SAMPLES {
            return Err(CalibrationError::InvalidBatchLength);
        }
        if raw_quantity.value().is_none() || raw_unit.value().is_none() {
            return Err(CalibrationError::InvalidIdentifier);
        }
        if samples.iter().any(|s| !s.is_finite()) {
            return Err(CalibrationError::NonFiniteSample);
        }
        Ok(Self {
            stage: SignalStageRelation::RawCounts,
            quantity: raw_quantity,
            unit: raw_unit,
            units_per_stored_sample: 1.0,
            stored_offset: 0.0,
            lineage_certificate_digest: None,
            samples,
        })
    }

    /// Re-open an already calibrated `PhysicalBaseUnits` series from storage (C02-C05, Schema 210).
    pub fn reopen_physical_base_units(
        physical_quantity: SemanticId,
        physical_unit: SemanticId,
        units_per_stored_sample: f64,
        stored_offset: f64,
        lineage_certificate_digest: [u8; 32],
        samples: [f64; N],
    ) -> Result<Self, CalibrationError> {
        if N == 0 || N > MAX_CALIBRATION_BATCH_SAMPLES {
            return Err(CalibrationError::InvalidBatchLength);
        }
        if physical_quantity.value().is_none()
            || physical_unit.value().is_none()
            || lineage_certificate_digest == [0u8; 32]
        {
            return Err(CalibrationError::InvalidIdentifier);
        }
        if units_per_stored_sample != 1.0 || stored_offset != 0.0 {
            return Err(CalibrationError::PhysicalBaseUnitsRequiresIdentityScaleOffset);
        }
        if samples.iter().any(|s| !s.is_finite()) {
            return Err(CalibrationError::NonFiniteSample);
        }
        Ok(Self {
            stage: SignalStageRelation::PhysicalBaseUnits,
            quantity: physical_quantity,
            unit: physical_unit,
            units_per_stored_sample: 1.0,
            stored_offset: 0.0,
            lineage_certificate_digest: Some(lineage_certificate_digest),
            samples,
        })
    }

    pub const fn stage(&self) -> SignalStageRelation {
        self.stage
    }
    pub const fn quantity(&self) -> SemanticId {
        self.quantity
    }
    pub const fn unit(&self) -> SemanticId {
        self.unit
    }
    pub const fn units_per_stored_sample(&self) -> f64 {
        self.units_per_stored_sample
    }
    pub const fn stored_offset(&self) -> f64 {
        self.stored_offset
    }
    pub const fn lineage_certificate_digest(&self) -> Option<[u8; 32]> {
        self.lineage_certificate_digest
    }
    pub const fn samples(&self) -> &[f64; N] {
        &self.samples
    }

    /// Apply affine calibration `physical = scale * raw + offset` **exactly once**
    /// on `RawCounts` stage at `observation_instant` (C02-C05).
    pub fn apply_calibration_once(
        &self,
        cert: &CalibrationCertificate,
        observation_instant: RationalInstant,
        frequency_hz: Option<f64>,
    ) -> Result<Self, CalibrationError> {
        match self.stage {
            SignalStageRelation::PhysicalBaseUnits => {
                return Err(CalibrationError::DoubleCalibrationRejected);
            }
            SignalStageRelation::DigitalUnrelatedToPressure => {
                return Err(CalibrationError::DigitalUnrelatedCannotCalibrateToPhysical);
            }
            SignalStageRelation::RawCounts => {}
        }
        if self.quantity != cert.input_quantity {
            return Err(CalibrationError::InputQuantityMismatch);
        }
        cert.verify_applicable_at(observation_instant, frequency_hz)?;

        let mut out = [0.0f64; N];
        let mut i = 0;
        while i < N {
            let v = cert.affine_scale * self.samples[i] + cert.affine_offset;
            if !v.is_finite() {
                return Err(CalibrationError::NonFiniteSample);
            }
            out[i] = v;
            i += 1;
        }
        Ok(Self {
            stage: SignalStageRelation::PhysicalBaseUnits,
            quantity: cert.output_quantity,
            unit: cert.output_unit,
            units_per_stored_sample: 1.0,
            stored_offset: 0.0,
            lineage_certificate_digest: Some(cert.certificate_digest()),
            samples: out,
        })
    }
}

/// Apply single-application I/Q calibration on a complex `(I, Q)` pair (`C02-C05` / `EMF-A04`).
pub fn calibrate_complex_iq_pair_once(
    stage: SignalStageRelation,
    iq_pair: (f64, f64),
    input_quantity: SemanticId,
    cert: &CalibrationCertificate,
    observation_instant: RationalInstant,
    carrier_hz: f64,
) -> Result<((f64, f64), SignalStageRelation, f64, [u8; 32]), CalibrationError> {
    match stage {
        SignalStageRelation::PhysicalBaseUnits => {
            return Err(CalibrationError::DoubleCalibrationRejected);
        }
        SignalStageRelation::DigitalUnrelatedToPressure => {
            return Err(CalibrationError::DigitalUnrelatedCannotCalibrateToPhysical);
        }
        SignalStageRelation::RawCounts => {}
    }
    if cert.affine_offset != 0.0 {
        return Err(CalibrationError::NonZeroScalarComplexOffsetRejected);
    }
    if input_quantity != cert.input_quantity {
        return Err(CalibrationError::InputQuantityMismatch);
    }
    if !iq_pair.0.is_finite() || !iq_pair.1.is_finite() {
        return Err(CalibrationError::NonFiniteSample);
    }
    cert.verify_applicable_at(observation_instant, Some(carrier_hz))?;
    let i_cal = iq_pair.0 * cert.affine_scale;
    let q_cal = iq_pair.1 * cert.affine_scale;
    if !i_cal.is_finite() || !q_cal.is_finite() {
        return Err(CalibrationError::NonFiniteSample);
    }
    Ok((
        (i_cal, q_cal),
        SignalStageRelation::PhysicalBaseUnits,
        1.0,
        cert.certificate_digest(),
    ))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CalibrationError {
    InvalidIdentifier,
    InvalidClockInstant,
    RationalOverflow,
    ClockDomainMismatch,
    InvalidValidityInterval,
    CalibrationNotYetValid,
    CalibrationExpiredAtOrAfterUntil,
    ChildWindowExceedsIssuer,
    InvalidAffineParameters,
    InvalidReferenceLevel,
    InvalidFrequencyBand,
    FrequencyOutOfCalibrationBand,
    InvalidUncertaintyKind,
    InvalidUncertaintyValue,
    UnlabeledUncertaintyRejected,
    UncertaintyTargetMismatch,
    InvalidSignature,
    InvalidChainLength,
    UntrustedRootAuthority,
    BrokenParentDigestLink,
    BrokenIssuerSubjectLink,
    InvalidStageRelation,
    InvalidBatchLength,
    NonFiniteSample,
    InputQuantityMismatch,
    DoubleCalibrationRejected,
    DigitalUnrelatedCannotCalibrateToPhysical,
    PhysicalBaseUnitsRequiresIdentityScaleOffset,
    NonZeroScalarComplexOffsetRejected,
}
