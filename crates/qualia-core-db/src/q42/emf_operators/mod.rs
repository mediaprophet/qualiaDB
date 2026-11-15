//! `q42::emf_operators` — 3D Electromagnetic Vector Calculus, Cartesian Yee FDTD,
//! Dispersive ADE Media, Poynting Energy Balance, C02 Mode Oracles, and GPU/CPU Parity.
//!
//! Implements Packet **N01** (`emf_cpu_gpu_engineer` | Gates: `G-SCIENCE`, `G-GPU`, `G-REALTIME`)
//! under the C01 (`ownership.md`), C02 (`quantities-models.md`), and C03 (`wire-freeze.md`) contracts:
//! - Continuum 3D Maxwell/Yee/ADE/Poynting operators are owned by `etalon_sim::maxwell_emf` and
//!   re-exported here.
//! - GPU display pipeline (`EmfFieldCell`, `emf_volumetric.wgsl`) is owned by `etalon_render::emf`
//!   and GPU compute leapfrog stepping (`MAXWELL_YEE_STEP_WGSL`) by
//!   `etalon_wgsl_forge::numerical::physics::maxwell_yee`.
//! - `qualia-core-db` owns the 128-byte `Q42EMF64` Mode 0/Mode 2 wire record layouts and
//!   `BLAKE3` (`QUALIA-YEE-CHECKPOINT-v1\0`) checkpoint sealing.

pub mod cpml_restart;
pub mod dispersion_ade;
pub mod gpu_parity;
pub mod poynting_energy;
pub mod vector_calculus;
pub mod yee_fdtd;

pub use cpml_restart::*;
pub use dispersion_ade::*;
pub use etalon_sim::maxwell_emf::{
    BoundaryCondition, EmfOperatorError, Grid3dSpec, C0_M_PER_S, ELEMENTARY_CHARGE_C,
    EPS0_REF_F_PER_M, MU0_REF_H_PER_M, PLANCK_H_J_S, Z0_REF_OHM,
};
pub use gpu_parity::*;
pub use poynting_energy::*;
pub use vector_calculus::*;
pub use yee_fdtd::*;

#[cfg(test)]
mod tests;
