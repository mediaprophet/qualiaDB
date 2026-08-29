//! WASM-ontology fallbacks: physics wrappers need native or wasm-scientific.

use super::super::args;
use vibe::{Diagnostic, Span, Value};

macro_rules! stub {
    ($name:ident, $family:expr) => {
        pub fn $name(_args: &Value, span: Span) -> Result<Value, Diagnostic> {
            Err(args::need_scientific(span, $family))
        }
    };
}

stub!(projectile, "PhysicsAndODE");
stub!(wave_1d, "Physics");
stub!(heat_diffusion_1d, "Physics");
stub!(advection_diffusion_1d, "Physics");
stub!(harmonic_oscillator, "Physics");
stub!(pendulum, "Physics");
stub!(n_body, "Physics");
stub!(molecular_dynamics, "Physics");
stub!(cfd_step, "Physics");
stub!(quantum_states_1d, "Physics");
stub!(logistic_growth, "Physics");
stub!(emf_interference, "Physics");
stub!(emf_attenuation, "Physics");
stub!(doppler_shift, "Physics");
stub!(emf_field_grid_3d, "Physics");
stub!(emf_sample_at_depth, "Physics");
stub!(align, "Bioinformatics");
stub!(smiles, "OrganicChemistry");
stub!(creator_field_sample, "Physics");
stub!(creator_material_query, "Physics");
stub!(creator_evaluate_interaction, "Physics");
stub!(physics_compute, "PhysicsWorkbench");
