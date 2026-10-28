//! Ecological and trophic coupling engine.
//!
//! Evaluates multi-species and plant-fungal-microbial interactions including
//! ectomycorrhizal, arbuscular, rhizobial, saprotrophic, pollination, and
//! transpiration couplings, emitting parity-valid NQuins with strict provenance.

use crate::foundation::frame_layout::pack_float_object;
use crate::query::resolver::INLINE_TAG_INTEGER;
use crate::{q_hash, NQuin};

/// Categorical types of ecological and trophic coupling between organisms.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TrophicCouplingKind {
    /// Bidirectional carbon-for-phosphorus exchange in ectomycorrhizal symbioses.
    EctomycorrhizalCarbonPhosphorusExchange = 0,
    /// Arbuscular mycorrhizal mineral and water uptake to host root.
    ArbuscularMycorrhizalUptake = 1,
    /// Rhizobial bacteroid nitrogen fixation within root nodules.
    RhizobialNitrogenFixation = 2,
    /// Saprotrophic fungal/bacterial breakdown of plant litter into humus.
    SaprotrophicLitterDecomposition = 3,
    /// Floral mutualism via insect or avian pollen vectoring.
    AvianInsectPollination = 4,
    /// Soil-plant-atmosphere continuum moisture extraction and transpiration.
    SoilMoistureCanopyTranspiration = 5,
}

/// Specification for an ecological or trophic interaction link.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct EcologicalCouplingSpec {
    /// Source organism or fungal/microbial agent DID hash.
    pub source_entity_did_hash: u64,
    /// Target organism, host plant, or substrate entity DID hash.
    pub target_entity_did_hash: u64,
    /// Nature of the trophic/ecological relationship.
    pub coupling_kind: TrophicCouplingKind,
    /// Base mass or energy exchange rate (mol/hr or index).
    pub flux_rate: f32,
    /// Lower soil moisture threshold for functional activity in `[0.0, 1.0]`.
    pub min_soil_moisture: f32,
    /// Upper soil moisture threshold before waterlogging/inhibition in `[0.0, 1.0]`.
    pub max_soil_moisture: f32,
    /// Optimal temperature in Celsius for maximum flux efficiency.
    pub temperature_optimum_c: f32,
    /// Provenance flag: false for FieldObservation, true for MechanisticSimulation.
    pub is_simulation: bool,
    /// Whether the link is currently structurally active.
    pub active: bool,
}

/// Errors occurring during ecological coupling evaluation.
#[derive(Debug, Clone, PartialEq)]
pub enum CouplingError {
    InvalidMoisture(f32),
    NegativeFluxRate(f32),
    BufferOverflow { needed: usize, available: usize },
    InvalidThreshold { min: f32, max: f32 },
}

impl std::fmt::Display for CouplingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidMoisture(m) => write!(f, "Invalid moisture {m}; must be in [0.0, 1.0]"),
            Self::NegativeFluxRate(flux) => write!(f, "Negative flux rate: {flux}"),
            Self::BufferOverflow { needed, available } => {
                write!(f, "Buffer overflow: needed {needed}, got {available}")
            }
            Self::InvalidThreshold { min, max } => {
                write!(f, "Invalid moisture thresholds: min {min} must be <= max {max} within [0.0, 1.0]")
            }
        }
    }
}

impl std::error::Error for CouplingError {}

pub const PRED_TROPHIC_COUPLING_KIND: u64 = q_hash("qualia:biosphere#trophicCouplingKind");
pub const PRED_TROPHIC_EXCHANGE_FLUX: u64 = q_hash("qualia:biosphere#trophicExchangeFlux");

/// Provenance metadata bit: 0 for FieldObservation, 1 for MechanisticSimulation.
pub const METADATA_PROVENANCE_SIMULATION: u64 = 1u64 << 28;

/// Evaluates ecological coupling rates and emits parity-valid NQuins into a caller buffer.
///
/// Modulates flux rate against ambient soil moisture curve and encodes the provenance
/// bit (bit 28) into the metadata of every emitted quin.
pub fn evaluate_ecological_coupling_zero_heap(
    couplings: &[EcologicalCouplingSpec],
    soil_moisture: f32,
    out_quins: &mut [NQuin],
) -> Result<usize, CouplingError> {
    if soil_moisture < 0.0 || soil_moisture > 1.0 || !soil_moisture.is_finite() {
        return Err(CouplingError::InvalidMoisture(soil_moisture));
    }

    for spec in couplings.iter() {
        if spec.flux_rate < 0.0 || !spec.flux_rate.is_finite() {
            return Err(CouplingError::NegativeFluxRate(spec.flux_rate));
        }
        if spec.min_soil_moisture < 0.0
            || spec.max_soil_moisture > 1.0
            || spec.min_soil_moisture > spec.max_soil_moisture
            || !spec.min_soil_moisture.is_finite()
            || !spec.max_soil_moisture.is_finite()
        {
            return Err(CouplingError::InvalidThreshold {
                min: spec.min_soil_moisture,
                max: spec.max_soil_moisture,
            });
        }
    }

    let needed = couplings.len() * 2;
    if out_quins.len() < needed {
        return Err(CouplingError::BufferOverflow {
            needed,
            available: out_quins.len(),
        });
    }

    let mut idx = 0;
    for spec in couplings.iter() {
        let active_flux = if spec.active
            && soil_moisture >= spec.min_soil_moisture
            && soil_moisture <= spec.max_soil_moisture
        {
            let span = (spec.max_soil_moisture - spec.min_soil_moisture).max(1e-4);
            let norm = (soil_moisture - spec.min_soil_moisture) / span;
            // Bell curve activity peaked between min and max
            let factor = (4.0 * norm * (1.0 - norm)).clamp(0.0, 1.0);
            spec.flux_rate * factor
        } else {
            0.0
        };

        let prov = if spec.is_simulation {
            METADATA_PROVENANCE_SIMULATION
        } else {
            0
        };
        let metadata = (0b01u64 << 61) | prov;
        let subj = spec.source_entity_did_hash;
        let ctx = spec.target_entity_did_hash;

        // Quin 1: Coupling Kind
        let obj_kind = INLINE_TAG_INTEGER | (spec.coupling_kind as u64);
        let p1 = NQuin::calculate_parity(subj, PRED_TROPHIC_COUPLING_KIND, obj_kind, ctx, metadata);
        out_quins[idx] = NQuin {
            subject: subj,
            predicate: PRED_TROPHIC_COUPLING_KIND,
            object: obj_kind,
            context: ctx,
            metadata,
            parity: p1,
        };
        idx += 1;

        // Quin 2: Modulated Exchange Flux
        let obj_flux = pack_float_object(active_flux);
        let p2 = NQuin::calculate_parity(subj, PRED_TROPHIC_EXCHANGE_FLUX, obj_flux, ctx, metadata);
        out_quins[idx] = NQuin {
            subject: subj,
            predicate: PRED_TROPHIC_EXCHANGE_FLUX,
            object: obj_flux,
            context: ctx,
            metadata,
            parity: p2,
        };
        idx += 1;
    }

    Ok(idx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::foundation::frame_layout::unpack_float_object;
    use crate::specialized_libs::computational_geometry::allocation_counter::assert_zero_alloc;

    #[test]
    fn test_coupling_moisture_gating() {
        let specs = [EcologicalCouplingSpec {
            source_entity_did_hash: 100,
            target_entity_did_hash: 200,
            coupling_kind: TrophicCouplingKind::EctomycorrhizalCarbonPhosphorusExchange,
            flux_rate: 10.0,
            min_soil_moisture: 0.2,
            max_soil_moisture: 0.8,
            temperature_optimum_c: 22.0,
            is_simulation: true,
            active: true,
        }];

        let mut out = [NQuin::default(); 2];

        // Optimal moisture (0.5) -> peak flux
        evaluate_ecological_coupling_zero_heap(&specs, 0.5, &mut out).unwrap();
        let flux_mid = unpack_float_object(out[1].object);
        assert!((flux_mid - 10.0).abs() < 1e-4);

        // Outside moisture window -> 0 flux
        evaluate_ecological_coupling_zero_heap(&specs, 0.1, &mut out).unwrap();
        let flux_low = unpack_float_object(out[1].object);
        assert_eq!(flux_low, 0.0);
    }

    #[test]
    fn test_provenance_bit_encoding() {
        let specs = [
            EcologicalCouplingSpec {
                source_entity_did_hash: 1,
                target_entity_did_hash: 2,
                coupling_kind: TrophicCouplingKind::RhizobialNitrogenFixation,
                flux_rate: 5.0,
                min_soil_moisture: 0.1,
                max_soil_moisture: 0.9,
                temperature_optimum_c: 20.0,
                is_simulation: false, // Observation
                active: true,
            },
            EcologicalCouplingSpec {
                source_entity_did_hash: 3,
                target_entity_did_hash: 4,
                coupling_kind: TrophicCouplingKind::AvianInsectPollination,
                flux_rate: 2.0,
                min_soil_moisture: 0.0,
                max_soil_moisture: 1.0,
                temperature_optimum_c: 25.0,
                is_simulation: true, // Simulation
                active: true,
            },
        ];

        let mut out = [NQuin::default(); 4];
        evaluate_ecological_coupling_zero_heap(&specs, 0.4, &mut out).unwrap();

        // Check parity and provenance bit
        for q in &out {
            assert!(q.verify_ecc_parity());
        }
        // First pair (obs): bit 28 = 0
        assert_eq!((out[0].metadata >> 28) & 1, 0);
        assert_eq!((out[1].metadata >> 28) & 1, 0);
        // Second pair (sim): bit 28 = 1
        assert_eq!((out[2].metadata >> 28) & 1, 1);
        assert_eq!((out[3].metadata >> 28) & 1, 1);
    }

    #[test]
    fn test_negative_cases_rejected() {
        let invalid_flux = [EcologicalCouplingSpec {
            source_entity_did_hash: 1,
            target_entity_did_hash: 2,
            coupling_kind: TrophicCouplingKind::SaprotrophicLitterDecomposition,
            flux_rate: -3.0, // Invalid
            min_soil_moisture: 0.2,
            max_soil_moisture: 0.8,
            temperature_optimum_c: 20.0,
            is_simulation: true,
            active: true,
        }];
        let mut out = [NQuin::default(); 2];
        assert!(evaluate_ecological_coupling_zero_heap(&invalid_flux, 0.5, &mut out).is_err());

        let valid = [EcologicalCouplingSpec {
            source_entity_did_hash: 1,
            target_entity_did_hash: 2,
            coupling_kind: TrophicCouplingKind::SaprotrophicLitterDecomposition,
            flux_rate: 3.0,
            min_soil_moisture: 0.2,
            max_soil_moisture: 0.8,
            temperature_optimum_c: 20.0,
            is_simulation: true,
            active: true,
        }];
        // Invalid moisture
        assert!(evaluate_ecological_coupling_zero_heap(&valid, 1.5, &mut out).is_err());
        assert!(evaluate_ecological_coupling_zero_heap(&valid, -0.1, &mut out).is_err());

        // Buffer overflow
        let mut small_out = [NQuin::default(); 1];
        assert!(matches!(
            evaluate_ecological_coupling_zero_heap(&valid, 0.5, &mut small_out),
            Err(CouplingError::BufferOverflow { .. })
        ));
    }

    #[test]
    fn test_ecological_coupling_zero_heap() {
        let specs = [EcologicalCouplingSpec {
            source_entity_did_hash: 55,
            target_entity_did_hash: 66,
            coupling_kind: TrophicCouplingKind::SoilMoistureCanopyTranspiration,
            flux_rate: 15.0,
            min_soil_moisture: 0.15,
            max_soil_moisture: 0.85,
            temperature_optimum_c: 24.0,
            is_simulation: true,
            active: true,
        }];

        let mut out = [NQuin::default(); 2];
        assert_zero_alloc("evaluate_ecological_coupling", || {
            let count = evaluate_ecological_coupling_zero_heap(&specs, 0.5, &mut out).unwrap();
            assert_eq!(count, 2);
        });
        assert!(out[0].verify_ecc_parity());
        assert!(out[1].verify_ecc_parity());
    }
}
