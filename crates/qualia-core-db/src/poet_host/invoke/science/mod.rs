//! Future seam: `qualia-science` (`domains/` + physics/chem/bio libraries).

#[cfg(not(target_arch = "wasm32"))]
mod bio;
#[cfg(not(target_arch = "wasm32"))]
mod chem;
#[cfg(not(target_arch = "wasm32"))]
mod emf;
#[cfg(not(target_arch = "wasm32"))]
mod physics;

#[cfg(not(target_arch = "wasm32"))]
pub use bio::align;
#[cfg(not(target_arch = "wasm32"))]
pub use chem::smiles;
#[cfg(not(target_arch = "wasm32"))]
pub use emf::{
    doppler_shift, emf_attenuation, emf_field_grid_3d, emf_interference, emf_sample_at_depth,
};
#[cfg(not(target_arch = "wasm32"))]
pub use physics::{
    advection_diffusion_1d, cfd_step, creator_evaluate_interaction, creator_field_sample,
    creator_material_query, harmonic_oscillator, heat_diffusion_1d, logistic_growth,
    molecular_dynamics, n_body, pendulum, projectile, quantum_states_1d, wave_1d,
};

#[cfg(target_arch = "wasm32")]
mod stubs;

#[cfg(target_arch = "wasm32")]
pub use stubs::*;
