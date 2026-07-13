//! **Interpolation & function approximation** (Gap analysis §3.7).
//!
//! * [`lagrange`] — Lagrange and Newton divided-difference polynomial interpolation.
//! * [`spline`] — natural cubic spline (tridiagonal Thomas solve) and linear interpolation.
//! * [`least_squares`] — polynomial least-squares fit via the normal equations.
//!
//! Fail-closed ([`InterpolationError`]): empty/mismatched data, duplicate nodes, an
//! over-high fit degree, or a singular system return an error rather than a fabricated
//! curve.

pub mod lagrange;
pub mod least_squares;
pub mod spline;

pub use lagrange::{lagrange_eval, newton_coefficients, newton_eval};
pub use least_squares::{poly_eval, poly_fit};
pub use spline::{linear_interp, CubicSpline};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InterpolationError {
    /// Fewer points than the method needs, or x/y length mismatch.
    InsufficientData,
    /// Two sample nodes share an x (interpolant undefined).
    DuplicateNodes,
    /// Requested fit degree ≥ number of points, or otherwise invalid.
    InvalidDegree,
    /// The linear system was singular / rank-deficient.
    Singular,
}

impl core::fmt::Display for InterpolationError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            InterpolationError::InsufficientData => write!(f, "insufficient interpolation data"),
            InterpolationError::DuplicateNodes => write!(f, "duplicate interpolation nodes"),
            InterpolationError::InvalidDegree => write!(f, "invalid fit degree"),
            InterpolationError::Singular => write!(f, "singular linear system"),
        }
    }
}
impl std::error::Error for InterpolationError {}
