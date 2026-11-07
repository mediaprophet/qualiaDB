//! Reference deterministic domain provider (`DeterministicMechanicsProvider`, `VS03-08`)
//! re-exported from `etalon-vibe` with compile-time sealed `NQuin48` ontology records
//! and direct-call oracle helpers.

pub use etalon_vibe::{
    oracle_aabb_volume, oracle_excavation_prism_volume, oracle_kinetic_energy,
    oracle_spring_damper_force, oracle_verlet_step, DeterministicMechanicsProvider,
};
