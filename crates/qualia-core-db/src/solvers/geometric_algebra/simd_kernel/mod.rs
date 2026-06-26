//! 3D Geometric Algebra (Cl_3,0) SIMD kernel.
//!
//! Split into focused submodules (CLAUDE.md §11):
//! - [`simd_backend`] — the `[f32; 8]` array kernels, basis sign tables, AVX2/FMA.
//! - [`types`] — the value types (`Multivector`, `Grade`, `Rotor`, `Translator`).
//! - [`operations`] — grade-aware products and rotor/translator application.
//!
//! The public surface is re-exported verbatim so `simd_kernel::*` consumers are
//! unaffected by the split.

pub mod operations;
pub mod simd_backend;
pub mod types;

pub use operations::{
    apply_rotor, apply_translator, geometric_product, is_simd_available, outer_product,
    rotor_from_angle_axis, translator_from_displacement,
};
pub use simd_backend::{
    multivector_geometric_product, multivector_outer_product, GaKernel, GA_SIMD_KERNEL,
};
pub use types::{Grade, Multivector, Rotor, Translator};
