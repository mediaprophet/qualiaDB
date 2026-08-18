//! WASM-ontology fallbacks: physics wrappers need native or wasm-scientific.

use super::super::args;
use poet_vibe::{Diagnostic, Span, Value};

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
stub!(align, "Bioinformatics");
stub!(smiles, "OrganicChemistry");
