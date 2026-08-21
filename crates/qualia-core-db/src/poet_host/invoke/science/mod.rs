//! Future seam: `qualia-science` (`domains/` + physics/chem/bio libraries).

#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod bio;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod chem;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod emf;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod physics;

#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use bio::align;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use chem::smiles;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use emf::{
    doppler_shift, emf_attenuation, emf_field_grid_3d, emf_interference, emf_sample_at_depth,
};
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use physics::{
    advection_diffusion_1d, cfd_step, creator_evaluate_interaction, creator_field_sample,
    creator_material_query, harmonic_oscillator, heat_diffusion_1d, logistic_growth,
    molecular_dynamics, n_body, pendulum, projectile, quantum_states_1d, wave_1d,
};

#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
mod stubs;

#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub use stubs::*;
