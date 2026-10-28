//! Microbial colony, biofilm dynamics, and multi-scale coordinate transforms.
//!
//! Enforces the BIO-MC-1 fail-closed scale-band rule to prevent unit confusion
//! when projecting nanometer/micrometer microbial coordinates into millimeter
//! scene space, alongside Monod/logistic biofilm growth and zero-heap NQuin emission.

use crate::foundation::frame_layout::pack_float_object;
use crate::query::resolver::INLINE_TAG_INTEGER;
use crate::{q_hash, NQuin};

/// Physical measurement scale-bands spanning microscopic to macroscopic realms.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ScaleBandUnit {
    Nanometer = 0,
    Micrometer = 1,
    Millimeter = 2,
    Meter = 3,
}

/// Coordinate frame anchoring local microbial space to macroscopic scene millimeter space.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct ScaleBandFrame {
    pub unit: ScaleBandUnit,
    pub origin_scene_mm: [f64; 3],
    pub um_per_local_unit: f64,
}

/// Explicit affine transformation across scale-bands.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct ScaleBandTransform {
    pub source_unit: ScaleBandUnit,
    pub target_unit: ScaleBandUnit,
    pub scale_factor: f64,
    pub translation_mm: [f64; 3],
}

/// Taxon classification for microbial agents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum MicrobialTaxonKind {
    Cyanobacteria = 0,
    Rhizobia = 1,
    Actinobacteria = 2,
    DiatomMicroalgae = 3,
}

/// Physiological and population state of a microbial colony/biofilm.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct MicrobialColonyState {
    pub colony_id: u64,
    pub taxon: MicrobialTaxonKind,
    pub cell_count: f64,
    pub cell_length_um: f32,
    pub cell_width_um: f32,
    pub division_rate_per_hour: f32,
    pub quorum_sensing_concentration_nm: f32,
    pub biofilm_eps_thickness_um: f32,
    pub substrate_nutrient_concentration: f32,
}

/// Errors occurring during microbial colony simulation or projection.
#[derive(Debug, Clone, PartialEq)]
pub enum MicrobialError {
    MissingScaleBandTransform,
    IncompatibleScaleBandUnit { expected: ScaleBandUnit, actual: ScaleBandUnit },
    InvalidCoordinate(f64),
    NegativeParameter(f32),
    BufferOverflow { needed: usize, available: usize },
}

impl std::fmt::Display for MicrobialError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingScaleBandTransform => write!(f, "BIO-MC-1: Missing explicit ScaleBandTransform for micro/nano coordinates"),
            Self::IncompatibleScaleBandUnit { expected, actual } => {
                write!(f, "Incompatible scale band unit: expected {expected:?}, got {actual:?}")
            }
            Self::InvalidCoordinate(c) => write!(f, "Invalid non-finite coordinate: {c}"),
            Self::NegativeParameter(p) => write!(f, "Negative parameter: {p}"),
            Self::BufferOverflow { needed, available } => write!(f, "Buffer overflow: needed {needed}, got {available}"),
        }
    }
}

impl std::error::Error for MicrobialError {}

pub const PRED_MICROBIAL_TAXON: u64 = q_hash("qualia:biosphere#microbialTaxon");
pub const PRED_CELL_COUNT: u64 = q_hash("qualia:biosphere#cellCount");
pub const PRED_QUORUM_SENSING: u64 = q_hash("qualia:biosphere#quorumSensingConcentration");
pub const PRED_BIOFILM_EPS_THICKNESS: u64 = q_hash("qualia:biosphere#biofilmEpsThickness");
pub const PRED_SUBSTRATE_NUTRIENT: u64 = q_hash("qualia:biosphere#substrateNutrientConcentration");
pub const METADATA_PROVENANCE_SIMULATION: u64 = 1u64 << 28;

/// BIO-MC-1: Projects local microbial coordinates into scene millimeter coordinates.
///
/// Fails closed (`Err(MicrobialError::MissingScaleBandTransform)`) whenever
/// `Micrometer` or `Nanometer` coordinates are mixed into scene space without an
/// explicit `ScaleBandTransform` (or when `transform.source_unit != pos_unit`).
pub fn project_microbial_to_scene_mm_checked(
    local_pos: [f64; 3],
    pos_unit: ScaleBandUnit,
    transform: Option<&ScaleBandTransform>,
) -> Result<[f64; 3], MicrobialError> {
    for &c in &local_pos {
        if !c.is_finite() {
            return Err(MicrobialError::InvalidCoordinate(c));
        }
    }

    match pos_unit {
        ScaleBandUnit::Micrometer | ScaleBandUnit::Nanometer => {
            let t = transform.ok_or(MicrobialError::MissingScaleBandTransform)?;
            if t.source_unit != pos_unit {
                return Err(MicrobialError::IncompatibleScaleBandUnit {
                    expected: t.source_unit,
                    actual: pos_unit,
                });
            }
            if t.target_unit != ScaleBandUnit::Millimeter {
                return Err(MicrobialError::IncompatibleScaleBandUnit {
                    expected: ScaleBandUnit::Millimeter,
                    actual: t.target_unit,
                });
            }
            Ok([
                local_pos[0] * t.scale_factor + t.translation_mm[0],
                local_pos[1] * t.scale_factor + t.translation_mm[1],
                local_pos[2] * t.scale_factor + t.translation_mm[2],
            ])
        }
        ScaleBandUnit::Millimeter => {
            if let Some(t) = transform {
                if t.source_unit != ScaleBandUnit::Millimeter {
                    return Err(MicrobialError::IncompatibleScaleBandUnit {
                        expected: t.source_unit,
                        actual: pos_unit,
                    });
                }
                Ok([
                    local_pos[0] * t.scale_factor + t.translation_mm[0],
                    local_pos[1] * t.scale_factor + t.translation_mm[1],
                    local_pos[2] * t.scale_factor + t.translation_mm[2],
                ])
            } else {
                Ok(local_pos)
            }
        }
        ScaleBandUnit::Meter => {
            if let Some(t) = transform {
                if t.source_unit != ScaleBandUnit::Meter {
                    return Err(MicrobialError::IncompatibleScaleBandUnit {
                        expected: t.source_unit,
                        actual: pos_unit,
                    });
                }
                Ok([
                    local_pos[0] * t.scale_factor + t.translation_mm[0],
                    local_pos[1] * t.scale_factor + t.translation_mm[1],
                    local_pos[2] * t.scale_factor + t.translation_mm[2],
                ])
            } else {
                Ok([local_pos[0] * 1000.0, local_pos[1] * 1000.0, local_pos[2] * 1000.0])
            }
        }
    }
}

/// Zero-heap step executing Monod kinetics, logistic carrying capacity, and quorum-sensing EPS production.
pub fn step_microbial_biofilm_zero_heap(
    colony: &mut MicrobialColonyState,
    dt_hours: f32,
    substrate_replenishment_rate: f32,
) -> Result<(), MicrobialError> {
    if dt_hours < 0.0 || !dt_hours.is_finite() {
        return Err(MicrobialError::NegativeParameter(dt_hours));
    }
    if substrate_replenishment_rate < 0.0 || !substrate_replenishment_rate.is_finite() {
        return Err(MicrobialError::NegativeParameter(substrate_replenishment_rate));
    }
    if colony.cell_count < 0.0
        || colony.substrate_nutrient_concentration < 0.0
        || colony.quorum_sensing_concentration_nm < 0.0
        || colony.biofilm_eps_thickness_um < 0.0
    {
        return Err(MicrobialError::NegativeParameter(colony.substrate_nutrient_concentration));
    }

    let s = colony.substrate_nutrient_concentration;
    let ks = 0.5f32; // Monod half-saturation constant
    let monod = if s > 0.0 { s / (ks + s) } else { 0.0 };
    let mu = colony.division_rate_per_hour * monod;

    let carrying_capacity = 1.0e11f64;
    let n = colony.cell_count;
    let dn = n * (mu as f64) * (1.0 - (n / carrying_capacity)).max(0.0) * (dt_hours as f64);
    colony.cell_count = (n + dn).max(0.0);

    let yield_coeff = 1.0e9f64; // cells per unit nutrient
    let substrate_consumed = (dn / yield_coeff) as f32;
    colony.substrate_nutrient_concentration =
        (s - substrate_consumed + substrate_replenishment_rate * dt_hours).max(0.0);

    // Quorum sensing autoinducer accumulation
    let qs_produced = (colony.cell_count * 1.0e-9) as f32 * dt_hours;
    colony.quorum_sensing_concentration_nm += qs_produced;

    // Quorum-sensing threshold triggers biofilm EPS secretion
    if colony.quorum_sensing_concentration_nm >= 10.0 {
        let eps_rate = 0.15f32; // um per hour
        colony.biofilm_eps_thickness_um += eps_rate * dt_hours;
    }

    Ok(())
}

/// Emits parity-valid NQuins representing microbial colony and biofilm status.
pub fn emit_microbial_quins_zero_heap(
    colony: &MicrobialColonyState,
    origin_did_hash: u64,
    is_simulation: bool,
    out_quins: &mut [NQuin],
) -> Result<usize, MicrobialError> {
    if out_quins.len() < 5 {
        return Err(MicrobialError::BufferOverflow { needed: 5, available: out_quins.len() });
    }

    let prov = if is_simulation { METADATA_PROVENANCE_SIMULATION } else { 0 };
    let metadata = (0b01u64 << 61) | prov;
    let subj = origin_did_hash ^ colony.colony_id;

    let props: [(u64, u64); 5] = [
        (PRED_MICROBIAL_TAXON, INLINE_TAG_INTEGER | (colony.taxon as u64)),
        (PRED_CELL_COUNT, pack_float_object(colony.cell_count as f32)),
        (PRED_QUORUM_SENSING, pack_float_object(colony.quorum_sensing_concentration_nm)),
        (PRED_BIOFILM_EPS_THICKNESS, pack_float_object(colony.biofilm_eps_thickness_um)),
        (PRED_SUBSTRATE_NUTRIENT, pack_float_object(colony.substrate_nutrient_concentration)),
    ];

    for (idx, (pred, obj)) in props.into_iter().enumerate() {
        let parity = NQuin::calculate_parity(subj, pred, obj, origin_did_hash, metadata);
        out_quins[idx] = NQuin { subject: subj, predicate: pred, object: obj, context: origin_did_hash, metadata, parity };
    }

    Ok(5)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::specialized_libs::computational_geometry::allocation_counter::assert_zero_alloc;

    #[test]
    fn test_missing_scale_band_transform_fails_closed() {
        let pos = [100.0, 200.0, 50.0];
        let err = project_microbial_to_scene_mm_checked(pos, ScaleBandUnit::Micrometer, None);
        assert_eq!(err, Err(MicrobialError::MissingScaleBandTransform));

        let err_nano = project_microbial_to_scene_mm_checked(pos, ScaleBandUnit::Nanometer, None);
        assert_eq!(err_nano, Err(MicrobialError::MissingScaleBandTransform));
    }

    #[test]
    fn test_incompatible_scale_band_unit_fails() {
        let pos = [100.0, 200.0, 50.0];
        let t = ScaleBandTransform {
            source_unit: ScaleBandUnit::Nanometer,
            target_unit: ScaleBandUnit::Millimeter,
            scale_factor: 1e-6,
            translation_mm: [0.0, 0.0, 0.0],
        };
        let err = project_microbial_to_scene_mm_checked(pos, ScaleBandUnit::Micrometer, Some(&t));
        assert!(matches!(err, Err(MicrobialError::IncompatibleScaleBandUnit { .. })));
    }

    #[test]
    fn test_scale_band_projection_success() {
        let pos_um = [1000.0, 2000.0, 500.0];
        let t = ScaleBandTransform {
            source_unit: ScaleBandUnit::Micrometer,
            target_unit: ScaleBandUnit::Millimeter,
            scale_factor: 0.001,
            translation_mm: [10.0, 20.0, 30.0],
        };
        let scene_mm = project_microbial_to_scene_mm_checked(pos_um, ScaleBandUnit::Micrometer, Some(&t)).unwrap();
        assert!((scene_mm[0] - 11.0).abs() < 1e-6);
        assert!((scene_mm[1] - 22.0).abs() < 1e-6);
        assert!((scene_mm[2] - 30.5).abs() < 1e-6);
    }

    #[test]
    fn test_biofilm_growth_and_quorum_sensing() {
        let mut colony = MicrobialColonyState {
            colony_id: 1,
            taxon: MicrobialTaxonKind::Cyanobacteria,
            cell_count: 1.0e6,
            cell_length_um: 4.0,
            cell_width_um: 1.0,
            division_rate_per_hour: 0.2,
            quorum_sensing_concentration_nm: 0.0,
            biofilm_eps_thickness_um: 0.5,
            substrate_nutrient_concentration: 5.0,
        };

        step_microbial_biofilm_zero_heap(&mut colony, 10.0, 0.1).unwrap();
        assert!(colony.cell_count > 1.0e6);
        assert!(colony.quorum_sensing_concentration_nm > 0.0);

        // High quorum sensing triggers EPS thickening
        colony.quorum_sensing_concentration_nm = 15.0;
        let eps_before = colony.biofilm_eps_thickness_um;
        step_microbial_biofilm_zero_heap(&mut colony, 2.0, 0.1).unwrap();
        assert!(colony.biofilm_eps_thickness_um > eps_before);
    }

    #[test]
    fn test_negative_parameter_rejected() {
        let mut colony = MicrobialColonyState {
            colony_id: 1,
            taxon: MicrobialTaxonKind::Rhizobia,
            cell_count: 1.0e5,
            cell_length_um: 2.0,
            cell_width_um: 0.8,
            division_rate_per_hour: 0.3,
            quorum_sensing_concentration_nm: 1.0,
            biofilm_eps_thickness_um: 0.1,
            substrate_nutrient_concentration: 2.0,
        };
        assert!(step_microbial_biofilm_zero_heap(&mut colony, -1.0, 0.1).is_err());
    }

    #[test]
    fn test_emit_microbial_quins_and_zero_heap() {
        let colony = MicrobialColonyState {
            colony_id: 77,
            taxon: MicrobialTaxonKind::Actinobacteria,
            cell_count: 5.0e7,
            cell_length_um: 3.5,
            cell_width_um: 0.6,
            division_rate_per_hour: 0.15,
            quorum_sensing_concentration_nm: 12.5,
            biofilm_eps_thickness_um: 1.8,
            substrate_nutrient_concentration: 3.2,
        };

        let mut out = [NQuin::default(); 5];
        assert_zero_alloc("emit_microbial_quins", || {
            let count = emit_microbial_quins_zero_heap(&colony, 9999, false, &mut out).unwrap();
            assert_eq!(count, 5);
        });

        for q in out.iter() {
            assert!(q.verify_ecc_parity());
            assert_eq!((q.metadata >> 28) & 1, 0); // FieldObservation
        }

        let mut small_out = [NQuin::default(); 2];
        let err = emit_microbial_quins_zero_heap(&colony, 9999, false, &mut small_out);
        assert!(matches!(err, Err(MicrobialError::BufferOverflow { .. })));
    }
}
