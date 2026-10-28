//! Typed EMF, Acoustic & Physical Field Cue Profile Engine (`BS-40`).
//!
//! Enforces physical units, frequency/wavelength intervals, and calibration status—
//! ensuring uncalibrated or out-of-sensor-range bands are rejected rather than
//! fabricating biological response scalars. Strictly zero-heap Tier-1 hot path.

use crate::foundation::frame_layout::{pack_float_object, unpack_float_object};
use crate::q_hash;
use crate::NQuin;

/// Physical cue domain taxonomy.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PhysicalCueDomain {
    ElectromagneticOptical = 0,
    ElectromagneticThermalIr = 1,
    GeomagneticField = 2,
    AcousticMechanical = 3,
    HydrodynamicPressure = 4,
    VolatileChemicalPlume = 5,
}

/// Physical SI / field quantity unit kinds.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PhysicalUnitKind {
    NanometerWavelength = 0,
    HertzFrequency = 1,
    WattsPerSquareMeterIrradiance = 2,
    NanoTeslaFluxDensity = 3,
    DecibelSplAcoustic = 4,
    PascalPressure = 5,
    MicromolePerCubicMeter = 6,
}

/// Sensor and model calibration fidelity status.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CalibrationStatus {
    LaboratoryCalibrated = 0,
    FieldCalibrated = 1,
    UncalibratedTheoretical = 2,
    OutOfSensorRange = 3,
}

impl CalibrationStatus {
    #[inline(always)]
    pub const fn is_calibrated(&self) -> bool {
        matches!(self, Self::LaboratoryCalibrated | Self::FieldCalibrated)
    }
}

/// Physical cue emission record describing field stimulus at source.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PhysicalCueEmission {
    pub source_entity_did: u64,
    pub source_organ_id: u64,
    pub domain: PhysicalCueDomain,
    pub unit: PhysicalUnitKind,
    pub band_min: f64,
    pub band_max: f64,
    pub magnitude: f64,
    pub polarization_deg: Option<f32>,
    pub calibration: CalibrationStatus,
}

/// Biological receptor sensitivity profile for an organism or taxon.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrganismReceptorProfile {
    pub receiver_taxon_did: u64,
    pub receptor_organ_id: u64,
    pub domain: PhysicalCueDomain,
    pub expected_unit: PhysicalUnitKind,
    pub sensitivity_min: f64,
    pub sensitivity_max: f64,
    pub peak_sensitivity: f64,
    pub threshold_magnitude: f64,
    pub calibration: CalibrationStatus,
}

/// Evaluation error for cue-receptor coupling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CueEvaluationError {
    UncalibratedBand,
    IncompatiblePhysicalUnit {
        cue_unit: PhysicalUnitKind,
        receptor_unit: PhysicalUnitKind,
    },
    IncompatibleDomain {
        cue_domain: PhysicalCueDomain,
        receptor_domain: PhysicalCueDomain,
    },
    BufferOverflow,
    InvalidBandRange,
}

impl std::fmt::Display for CueEvaluationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UncalibratedBand => write!(f, "Cue or receptor is uncalibrated / out of range"),
            Self::IncompatiblePhysicalUnit { cue_unit, receptor_unit } => {
                write!(f, "Unit mismatch: cue={cue_unit:?}, receptor={receptor_unit:?}")
            }
            Self::IncompatibleDomain { cue_domain, receptor_domain } => {
                write!(f, "Domain mismatch: cue={cue_domain:?}, receptor={receptor_domain:?}")
            }
            Self::BufferOverflow => write!(f, "Output buffer capacity is insufficient"),
            Self::InvalidBandRange => write!(f, "Band minimum exceeds band maximum"),
        }
    }
}

impl std::error::Error for CueEvaluationError {}

pub const OP_PHYSICAL_CUE_EMISSION: u8 = 0x62;
pub const OP_RECEPTOR_CUE_RESPONSE: u8 = 0x63;

/// Evaluates receptor coupling response scalar with zero heap allocations.
pub fn evaluate_receptor_cue_response_zero_heap(
    cue: &PhysicalCueEmission,
    receptor: &OrganismReceptorProfile,
) -> Result<f64, CueEvaluationError> {
    if cue.unit != receptor.expected_unit {
        return Err(CueEvaluationError::IncompatiblePhysicalUnit {
            cue_unit: cue.unit,
            receptor_unit: receptor.expected_unit,
        });
    }
    if cue.domain != receptor.domain {
        return Err(CueEvaluationError::IncompatibleDomain {
            cue_domain: cue.domain,
            receptor_domain: receptor.domain,
        });
    }
    if !cue.calibration.is_calibrated() || !receptor.calibration.is_calibrated() {
        return Err(CueEvaluationError::UncalibratedBand);
    }
    if cue.band_min > cue.band_max || receptor.sensitivity_min > receptor.sensitivity_max {
        return Err(CueEvaluationError::InvalidBandRange);
    }
    if cue.magnitude <= receptor.threshold_magnitude {
        return Ok(0.0);
    }
    if cue.band_max < receptor.sensitivity_min || cue.band_min > receptor.sensitivity_max {
        return Ok(0.0);
    }

    let overlap_min = cue.band_min.max(receptor.sensitivity_min);
    let overlap_max = cue.band_max.min(receptor.sensitivity_max);
    let overlap_span = overlap_max - overlap_min;
    let cue_span = (cue.band_max - cue.band_min).max(1e-9);
    let rec_span = (receptor.sensitivity_max - receptor.sensitivity_min).max(1e-9);
    let overlap_fraction = (overlap_span / cue_span).clamp(0.0, 1.0);

    let overlap_center = 0.5 * (overlap_min + overlap_max);
    let half_width = 0.5 * rec_span;
    let dist = (overlap_center - receptor.peak_sensitivity).abs();
    let spectral_weight = if dist >= half_width { 0.0 } else { 1.0 - (dist / half_width) };

    let excess_mag = cue.magnitude - receptor.threshold_magnitude;
    let response = excess_mag * (overlap_fraction * spectral_weight);
    Ok(if response < 0.0 { 0.0 } else { response })
}

/// Emits typed 48-byte NQuins representing cue emission and receptor response.
pub fn emit_typed_cue_quins_zero_heap(
    cue: &PhysicalCueEmission,
    receptor: &OrganismReceptorProfile,
    response_scalar: f64,
    out_quins: &mut [NQuin],
) -> Result<usize, CueEvaluationError> {
    if cue.unit != receptor.expected_unit {
        return Err(CueEvaluationError::IncompatiblePhysicalUnit {
            cue_unit: cue.unit,
            receptor_unit: receptor.expected_unit,
        });
    }
    if cue.domain != receptor.domain {
        return Err(CueEvaluationError::IncompatibleDomain {
            cue_domain: cue.domain,
            receptor_domain: receptor.domain,
        });
    }
    if !cue.calibration.is_calibrated() || !receptor.calibration.is_calibrated() {
        return Err(CueEvaluationError::UncalibratedBand);
    }
    if out_quins.len() < 2 {
        return Err(CueEvaluationError::BufferOverflow);
    }

    let pol_bits = match cue.polarization_deg {
        Some(deg) => (deg.clamp(0.0, 360.0) as u16) as u64,
        None => 0xFFFF,
    };

    let e_sub = cue.source_entity_did;
    let e_pred = (q_hash("urn:qualia:cue:emission") & 0x7FFF_FFFF_FFFF_FF00) | (OP_PHYSICAL_CUE_EMISSION as u64);
    let e_obj = pack_float_object(cue.magnitude as f32);
    let e_ctx = (cue.source_organ_id & 0x00FF_FFFF_FFFF_FFFF) | (((cue.domain as u8) as u64) << 56);
    let e_meta = (((cue.unit as u8) as u64) << 48) | (((cue.calibration as u8) as u64) << 40) | (pol_bits << 24) | 1u64;
    out_quins[0] = NQuin {
        subject: e_sub, predicate: e_pred, object: e_obj, context: e_ctx, metadata: e_meta,
        parity: NQuin::calculate_parity(e_sub, e_pred, e_obj, e_ctx, e_meta),
    };

    let r_sub = receptor.receiver_taxon_did;
    let r_pred = (q_hash("urn:qualia:cue:response") & 0x7FFF_FFFF_FFFF_FF00) | (OP_RECEPTOR_CUE_RESPONSE as u64);
    let r_obj = pack_float_object(response_scalar as f32);
    let r_ctx = (receptor.receptor_organ_id & 0x00FF_FFFF_FFFF_FFFF) | (((receptor.domain as u8) as u64) << 56);
    let r_meta = (((receptor.expected_unit as u8) as u64) << 48) | (((receptor.calibration as u8) as u64) << 40) | 1u64;
    out_quins[1] = NQuin {
        subject: r_sub, predicate: r_pred, object: r_obj, context: r_ctx, metadata: r_meta,
        parity: NQuin::calculate_parity(r_sub, r_pred, r_obj, r_ctx, r_meta),
    };

    Ok(2)
}

/// Unpacks magnitude and organ ID from a physical cue emission Quin.
pub fn unpack_cue_emission_quin(quin: &NQuin) -> Option<(u64, f32)> {
    if (quin.predicate & 0xFF) as u8 != OP_PHYSICAL_CUE_EMISSION { return None; }
    Some((quin.context & 0x00FF_FFFF_FFFF_FFFF, unpack_float_object(quin.object)))
}

/// Unpacks response scalar and organ ID from a receptor response Quin.
pub fn unpack_receptor_response_quin(quin: &NQuin) -> Option<(u64, f32)> {
    if (quin.predicate & 0xFF) as u8 != OP_RECEPTOR_CUE_RESPONSE { return None; }
    Some((quin.context & 0x00FF_FFFF_FFFF_FFFF, unpack_float_object(quin.object)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::specialized_libs::computational_geometry::allocation_counter::assert_zero_alloc;

    fn test_receptor() -> OrganismReceptorProfile {
        OrganismReceptorProfile {
            receiver_taxon_did: 0x9001_1002_0001,
            receptor_organ_id: 1,
            domain: PhysicalCueDomain::ElectromagneticOptical,
            expected_unit: PhysicalUnitKind::WattsPerSquareMeterIrradiance,
            sensitivity_min: 400.0,
            sensitivity_max: 700.0,
            peak_sensitivity: 550.0,
            threshold_magnitude: 5.0,
            calibration: CalibrationStatus::LaboratoryCalibrated,
        }
    }

    fn test_cue() -> PhysicalCueEmission {
        PhysicalCueEmission {
            source_entity_did: 0x1001_2002_3003,
            source_organ_id: 42,
            domain: PhysicalCueDomain::ElectromagneticOptical,
            unit: PhysicalUnitKind::WattsPerSquareMeterIrradiance,
            band_min: 540.0,
            band_max: 560.0,
            magnitude: 50.0,
            polarization_deg: Some(90.0),
            calibration: CalibrationStatus::FieldCalibrated,
        }
    }

    #[test]
    fn test_calibrated_response_and_uncalibrated_rejection() {
        let cue = test_cue();
        let receptor = test_receptor();
        let resp = evaluate_receptor_cue_response_zero_heap(&cue, &receptor).unwrap();
        assert!(resp > 0.0 && (resp - 45.0).abs() < 2.0);

        let mut uncal_cue = cue;
        uncal_cue.calibration = CalibrationStatus::UncalibratedTheoretical;
        assert_eq!(evaluate_receptor_cue_response_zero_heap(&uncal_cue, &receptor), Err(CueEvaluationError::UncalibratedBand));

        let mut out_of_range_rec = receptor;
        out_of_range_rec.calibration = CalibrationStatus::OutOfSensorRange;
        assert_eq!(evaluate_receptor_cue_response_zero_heap(&cue, &out_of_range_rec), Err(CueEvaluationError::UncalibratedBand));
    }

    #[test]
    fn test_incompatible_unit_and_domain() {
        let mut cue = test_cue();
        cue.unit = PhysicalUnitKind::DecibelSplAcoustic;
        let receptor = test_receptor();
        assert_eq!(
            evaluate_receptor_cue_response_zero_heap(&cue, &receptor),
            Err(CueEvaluationError::IncompatiblePhysicalUnit {
                cue_unit: PhysicalUnitKind::DecibelSplAcoustic,
                receptor_unit: PhysicalUnitKind::WattsPerSquareMeterIrradiance,
            })
        );

        let mut acoustic_cue = test_cue();
        acoustic_cue.domain = PhysicalCueDomain::AcousticMechanical;
        acoustic_cue.unit = PhysicalUnitKind::PascalPressure;
        let mut hydro_rec = test_receptor();
        hydro_rec.domain = PhysicalCueDomain::HydrodynamicPressure;
        hydro_rec.expected_unit = PhysicalUnitKind::PascalPressure;
        assert_eq!(
            evaluate_receptor_cue_response_zero_heap(&acoustic_cue, &hydro_rec),
            Err(CueEvaluationError::IncompatibleDomain {
                cue_domain: PhysicalCueDomain::AcousticMechanical,
                receptor_domain: PhysicalCueDomain::HydrodynamicPressure,
            })
        );
    }

    #[test]
    fn test_subthreshold_and_out_of_band() {
        let mut cue = test_cue();
        cue.magnitude = 3.0;
        let receptor = test_receptor();
        assert_eq!(evaluate_receptor_cue_response_zero_heap(&cue, &receptor).unwrap(), 0.0);

        let mut oob_cue = test_cue();
        oob_cue.band_min = 800.0;
        oob_cue.band_max = 900.0;
        assert_eq!(evaluate_receptor_cue_response_zero_heap(&oob_cue, &receptor).unwrap(), 0.0);
    }

    #[test]
    fn test_emit_typed_cue_quins_and_overflow() {
        let cue = test_cue();
        let receptor = test_receptor();
        let mut quins = [NQuin::default(); 2];
        assert_eq!(emit_typed_cue_quins_zero_heap(&cue, &receptor, 42.5, &mut quins).unwrap(), 2);
        assert!(quins[0].verify_ecc_parity());
        assert!(quins[1].verify_ecc_parity());

        let (cue_org, mag) = unpack_cue_emission_quin(&quins[0]).unwrap();
        assert_eq!(cue_org, 42);
        assert!((mag - 50.0).abs() < 1e-4);

        let (rec_org, sca) = unpack_receptor_response_quin(&quins[1]).unwrap();
        assert_eq!(rec_org, 1);
        assert!((sca - 42.5).abs() < 1e-4);

        let mut small = [NQuin::default(); 1];
        assert_eq!(emit_typed_cue_quins_zero_heap(&cue, &receptor, 10.0, &mut small), Err(CueEvaluationError::BufferOverflow));
    }

    #[test]
    fn test_zero_alloc_guarantee() {
        let cue = test_cue();
        let receptor = test_receptor();
        let mut quins = [NQuin::default(); 2];

        assert_zero_alloc("eval_receptor_cue_response", || {
            let res = evaluate_receptor_cue_response_zero_heap(&cue, &receptor).unwrap();
            assert!(res > 0.0);
        });

        assert_zero_alloc("emit_typed_cue_quins", || {
            let count = emit_typed_cue_quins_zero_heap(&cue, &receptor, 35.0, &mut quins).unwrap();
            assert_eq!(count, 2);
        });
    }
}
