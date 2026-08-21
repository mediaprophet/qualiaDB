//! Interaction Laws — physical transformation rules and evaluation engine.
//!
//! Evaluates interaction conditions when ambient fields, material signatures,
//! and spatial continuants meet, producing deterministic transformation events.
//!
//! Reference: `docs/plans/vibe-design/20260819_fields-materials-and-creator-physics.md` §2.3.

use crate::physics::fields::FieldDeclaration;
use crate::physics::materials::MaterialSignature;
use crate::value::Pose;

/// State of an embodied continuant in the physical simulation.
#[derive(Debug, Clone, PartialEq)]
pub struct ContinuantState {
    pub id: String,
    pub pose: Pose,
    pub material: MaterialSignature,
    pub mass_kg: f64,
    pub is_solid: bool,
    pub is_crushed: bool,
    pub current_temperature_k: f64,
}

/// Physical interaction event resulting from law evaluation.
#[derive(Debug, Clone, PartialEq)]
pub enum InteractionEvent {
    /// Object exceeded its mechanical yield strength under ambient pressure.
    YieldExceeded {
        continuant_id: String,
        ambient_pressure_kpa: f64,
        yield_threshold_kpa: f64,
        crush_ratio: f64,
    },
    /// Thermal energy caused a phase transition (e.g. melting).
    PhaseChange {
        continuant_id: String,
        temperature_k: f64,
        melt_threshold_k: f64,
        new_phase: &'static str,
    },
    /// Solute dissolving in solvent over delta time.
    DissolutionStep {
        solute_id: String,
        solvent_id: String,
        dissolved_mass_kg: f64,
        remaining_mass_kg: f64,
        mass_fraction: f64,
    },
    /// Immiscible materials meeting at an interface without blending (e.g. oil/water).
    ImmiscibleBarrier {
        body_a: String,
        body_b: String,
        interface_area_m2: f64,
    },
}

/// Evaluates ambient field interaction laws against a continuant.
pub fn evaluate_field_interactions(
    continuant: &mut ContinuantState,
    fields: &[FieldDeclaration],
    events: &mut Vec<InteractionEvent>,
) {
    for field in fields {
        let sample = field.sample_at(&continuant.pose.position);

        match field.quantity {
            crate::physics::fields::FieldQuantity::Pressure => {
                if let Some(mech) = &continuant.material.mechanical {
                    if sample > mech.yield_kpa && !continuant.is_crushed {
                        continuant.is_crushed = true;
                        let ratio = sample / mech.yield_kpa;
                        events.push(InteractionEvent::YieldExceeded {
                            continuant_id: continuant.id.clone(),
                            ambient_pressure_kpa: sample,
                            yield_threshold_kpa: mech.yield_kpa,
                            crush_ratio: ratio,
                        });
                    }
                }
            }
            crate::physics::fields::FieldQuantity::Temperature => {
                continuant.current_temperature_k = sample;
                if let Some(thermal) = &continuant.material.thermal {
                    if let Some(melt_k) = thermal.melt_point_k {
                        if sample >= melt_k && continuant.is_solid {
                            continuant.is_solid = false;
                            events.push(InteractionEvent::PhaseChange {
                                continuant_id: continuant.id.clone(),
                                temperature_k: sample,
                                melt_threshold_k: melt_k,
                                new_phase: "Liquid",
                            });
                        }
                    }
                }
            }
            crate::physics::fields::FieldQuantity::ChemicalSpecies(ref species_iri) => {
                if let Some(chem) = &continuant.material.chemical {
                    if chem.soluble_in.as_deref() == Some(species_iri) && sample > 0.0 {
                        // Rate law: dm/dt = k * m * dt (with default dt=1.0 for single eval step)
                        let dt = 1.0;
                        let dissolve_delta = (chem.dissolve_rate_per_s * continuant.mass_kg * dt)
                            .min(continuant.mass_kg);
                        continuant.mass_kg -= dissolve_delta;
                        let fraction = if continuant.mass_kg > 0.0 {
                            continuant.mass_kg / (continuant.mass_kg + dissolve_delta)
                        } else {
                            0.0
                        };

                        events.push(InteractionEvent::DissolutionStep {
                            solute_id: continuant.id.clone(),
                            solvent_id: species_iri.clone(),
                            dissolved_mass_kg: dissolve_delta,
                            remaining_mass_kg: continuant.mass_kg,
                            mass_fraction: fraction,
                        });
                    }
                }
            }
            _ => {}
        }
    }
}

/// Evaluates pairwise proximity interactions between two continuants (solvation, immiscibility).
pub fn evaluate_pairwise_interactions(
    body_a: &mut ContinuantState,
    body_b: &mut ContinuantState,
    dt: f64,
    events: &mut Vec<InteractionEvent>,
) {
    // Proximity / contact distance check
    let dx =
        body_a.pose.position.first().unwrap_or(&0.0) - body_b.pose.position.first().unwrap_or(&0.0);
    let dy = body_a.pose.position.get(1).unwrap_or(&0.0) - body_b.pose.get_coord(1);
    let dz = body_a.pose.position.get(2).unwrap_or(&0.0) - body_b.pose.get_coord(2);
    let dist_sq = dx * dx + dy * dy + dz * dz;

    // Contact threshold (e.g. radius < 0.5m)
    if dist_sq > 0.25 {
        return;
    }

    // Check Immiscibility Barrier (e.g. Oil & Water)
    if let Some(chem_a) = &body_a.material.chemical {
        if chem_a.immiscible_with.contains(&body_b.material.id) {
            events.push(InteractionEvent::ImmiscibleBarrier {
                body_a: body_a.id.clone(),
                body_b: body_b.id.clone(),
                interface_area_m2: 0.01,
            });
            return;
        }
    }

    // Check Solvation / Dissolution (e.g. Sugar in Water)
    if let Some(chem_a) = &body_a.material.chemical {
        if chem_a.soluble_in.as_deref() == Some(&body_b.material.id) && body_a.mass_kg > 0.0 {
            let dm = (chem_a.dissolve_rate_per_s * body_a.mass_kg * dt).min(body_a.mass_kg);
            body_a.mass_kg -= dm;
            let frac = body_a.mass_kg / (body_a.mass_kg + dm);

            events.push(InteractionEvent::DissolutionStep {
                solute_id: body_a.id.clone(),
                solvent_id: body_b.id.clone(),
                dissolved_mass_kg: dm,
                remaining_mass_kg: body_a.mass_kg,
                mass_fraction: frac,
            });
        }
    }
}

trait PoseExt {
    fn get_coord(&self, idx: usize) -> f64;
}

impl PoseExt for Pose {
    fn get_coord(&self, idx: usize) -> f64 {
        self.position.get(idx).copied().unwrap_or(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{FieldRepresentation, FieldSupport};
    use crate::physics::fields::{AnalyticFieldProfile, FieldQuantity, FieldUnit};

    fn make_test_pose(x: f64, y: f64, z: f64) -> Pose {
        Pose {
            position: vec![x, y, z],
            orientation: vec![1.0, 0.0, 0.0, 0.0],
            frame: None,
        }
    }

    #[test]
    fn test_yield_law_under_deep_pressure() {
        let mut sugar = ContinuantState {
            id: "sugar_01".to_string(),
            pose: make_test_pose(0.0, 0.0, 0.0),
            material: MaterialSignature::sugar_cube(),
            mass_kg: 0.01,
            is_solid: true,
            is_crushed: false,
            current_temperature_k: 293.0,
        };

        // Extreme pressure field (100 kPa > 50 kPa yield threshold)
        let deep_pressure = FieldDeclaration {
            id: "field:deep_pressure".to_string(),
            quantity: FieldQuantity::Pressure,
            unit: FieldUnit::KiloPascal,
            support: FieldSupport::Region,
            representation: FieldRepresentation::Analytic,
            profile: AnalyticFieldProfile::Uniform(100.0),
        };

        let mut events = Vec::new();
        evaluate_field_interactions(&mut sugar, &[deep_pressure], &mut events);

        assert!(sugar.is_crushed);
        assert_eq!(events.len(), 1);
        match &events[0] {
            InteractionEvent::YieldExceeded { crush_ratio, .. } => {
                assert!((crush_ratio - 2.0).abs() < 1e-4);
            }
            _ => panic!("expected YieldExceeded event"),
        }
    }

    #[test]
    fn test_sugar_dissolution_in_water() {
        let mut sugar = ContinuantState {
            id: "sugar_01".to_string(),
            pose: make_test_pose(0.0, 0.0, 0.0),
            material: MaterialSignature::sugar_cube(),
            mass_kg: 0.01,
            is_solid: true,
            is_crushed: false,
            current_temperature_k: 293.0,
        };

        let mut water = ContinuantState {
            id: "water_glass".to_string(),
            pose: make_test_pose(0.05, 0.0, 0.0), // Close contact
            material: MaterialSignature::liquid_water(),
            mass_kg: 0.25,
            is_solid: false,
            is_crushed: false,
            current_temperature_k: 293.0,
        };

        let mut events = Vec::new();
        evaluate_pairwise_interactions(&mut sugar, &mut water, 0.5, &mut events);

        assert_eq!(events.len(), 1);
        match &events[0] {
            InteractionEvent::DissolutionStep {
                dissolved_mass_kg,
                remaining_mass_kg,
                ..
            } => {
                // Rate = 0.5/s, dt = 0.5s => dm = 0.5 * 0.01 * 0.5 = 0.0025 kg
                assert!((dissolved_mass_kg - 0.0025).abs() < 1e-6);
                assert!((remaining_mass_kg - 0.0075).abs() < 1e-6);
            }
            _ => panic!("expected DissolutionStep event"),
        }
    }

    #[test]
    fn test_oil_water_immiscibility() {
        let mut oil = ContinuantState {
            id: "oil_drop".to_string(),
            pose: make_test_pose(0.0, 0.0, 0.0),
            material: MaterialSignature::mineral_oil(),
            mass_kg: 0.02,
            is_solid: false,
            is_crushed: false,
            current_temperature_k: 293.0,
        };

        let mut water = ContinuantState {
            id: "water_glass".to_string(),
            pose: make_test_pose(0.05, 0.0, 0.0),
            material: MaterialSignature::liquid_water(),
            mass_kg: 0.25,
            is_solid: false,
            is_crushed: false,
            current_temperature_k: 293.0,
        };

        let mut events = Vec::new();
        evaluate_pairwise_interactions(&mut oil, &mut water, 1.0, &mut events);

        assert_eq!(events.len(), 1);
        match &events[0] {
            InteractionEvent::ImmiscibleBarrier { .. } => {}
            _ => panic!("expected ImmiscibleBarrier event"),
        }
    }
}
