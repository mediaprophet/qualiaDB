//! Physics & Creator Physics module (Fields, Materials, Interaction Laws, Continuant WorldLines, and Frame Morphisms).
//!
//! Provides first-class physical domain primitives for VibeScript without consuming
//! or reallocating the ten manifold axes (XYZD+T Option A).

pub mod fields;
pub mod laws;
pub mod materials;
pub mod morphism;
pub mod trajectory;

pub use fields::{AnalyticFieldProfile, FieldDeclaration, FieldQuantity, FieldUnit};
pub use laws::{
    evaluate_field_interactions, evaluate_pairwise_interactions, ContinuantState, InteractionEvent,
};
pub use materials::{
    AcousticFacet, ChemicalFacet, MaterialSignature, MechanicalFacet, OpticalFacet, ThermalFacet,
};
pub use morphism::{transform_frame, GalileanMorphism, LorentzMorphism};
pub use trajectory::{Waypoint, WorldLineTrajectory};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::Pose;

    fn make_test_pose(x: f64, y: f64) -> Pose {
        Pose {
            position: vec![x, y, 0.0],
            orientation: vec![1.0, 0.0, 0.0, 0.0],
            frame: None,
        }
    }

    #[test]
    fn test_physics_module_integration() {
        let mut sugar = ContinuantState {
            id: "sugar_01".to_string(),
            pose: make_test_pose(0.0, 0.0),
            material: MaterialSignature::sugar_cube(),
            mass_kg: 0.01,
            is_solid: true,
            is_crushed: false,
            current_temperature_k: 293.0,
        };

        let atm = FieldDeclaration::standard_atmosphere();
        let mut events = Vec::new();
        evaluate_field_interactions(&mut sugar, &[atm], &mut events);

        // Standard atmosphere (101.325 kPa > 50 kPa) crushes unreinforced sugar cube
        assert!(sugar.is_crushed);
    }
}
