//! **Vector calculus** (Calculus plan §4.3) — the differential operators (grad, div,
//! curl, Laplacian) symbolically over the CAS, and the integral side (line integrals,
//! surface flux) numerically, with the divergence/Green theorems as validation.
//!
//! * [`differential`] — symbolic `∇f`, `∇·F`, `∇×F`, `∇²f` built on the CAS
//!   [`differentiate`](crate::specialized_libs::symbolic_algebra::differentiate);
//!   provenance-bearing operators, not numeric finite differences.
//! * [`integrals`] — numeric line integral / work and surface flux by quadrature over
//!   parametric curves/surfaces given as closures.
//!
//! Fail-closed on dimension mismatch (curl needs exactly 3 components/vars).

pub mod differential;
pub mod integrals;

pub use differential::{curl, divergence, gradient, laplacian};
pub use integrals::{line_integral_scalar, line_integral_work, surface_flux};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VecCalcError {
    /// The number of field components and variables disagree, or curl was given a
    /// non-3-D field.
    DimensionMismatch,
}

impl core::fmt::Display for VecCalcError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "vector-calculus dimension mismatch")
    }
}
impl std::error::Error for VecCalcError {}
