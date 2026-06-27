//! **Special functions** (Gap analysis §3.5) — beyond the Gamma/erf/incomplete family
//! already in [`crate::solvers::statistics::distributions::special`].
//!
//! * [`orthogonal`] — Legendre, Chebyshev (T/U), Hermite, Laguerre polynomials by
//!   their three-term recurrences.
//! * [`bessel`] — Bessel `J`/`Y` and modified `I`/`K` (integer order): convergent
//!   series for the order-0 building blocks, the Wronskian for order 1, then the
//!   standard recurrences.
//! * [`airy`] — Airy `Ai`/`Bi` via their Maclaurin series.
//! * [`zeta`] — Riemann ζ(s) for real `s > 1` via Euler–Maclaurin acceleration.
//!
//! Domain-restricted functions fail closed (`Option`/`None`) rather than return a
//! fabricated value (e.g. `Y_n`/`K_n` require `x > 0`; ζ requires `s > 1`). The series
//! methods are accurate for moderate arguments; the convergence regime is documented
//! per function.

pub mod airy;
pub mod bessel;
pub mod orthogonal;
pub mod zeta;

pub use airy::{airy_ai, airy_bi};
pub use bessel::{bessel_i, bessel_j, bessel_k, bessel_y};
pub use orthogonal::{chebyshev_t, chebyshev_u, hermite, laguerre, legendre};
pub use zeta::zeta;
