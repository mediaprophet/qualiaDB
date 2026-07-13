//! **Integral & discrete transforms** (Gap analysis §3.4) — Fourier, Laplace, Z.
//!
//! * [`fourier`] — discrete Fourier transform / inverse over complex samples.
//!   The forward [`dft`] is the f64-exact CPU reference; [`dft_accelerated`] is
//!   an opt-in f32 fast path that uses the WGSL forge FFT when an accelerator is
//!   present (for spectral callers that accept f32 precision).
//! * [`laplace`] — numerical Laplace transform by quadrature (general), plus a symbolic
//!   table transform over the CAS [`Expr`](crate::specialized_libs::symbolic_algebra::Expr)
//!   for the cases the current expression algebra can represent (constants, powers `tⁿ`,
//!   and their linear combinations) — fail-closed on the rest.
//! * [`ztransform`] — Z-transform of a finite sequence + the standard closed forms.
//!
//! Complex numbers are `(re, im)` tuples (`Cplx`). Fail-closed throughout.

pub mod fourier;
pub mod laplace;
pub mod ztransform;

pub use fourier::{dft, dft_accelerated, idft, Cplx};
pub use laplace::{laplace_numeric, laplace_table, LaplaceError};
pub use ztransform::{geometric_z, unit_step_z, z_transform_finite};
