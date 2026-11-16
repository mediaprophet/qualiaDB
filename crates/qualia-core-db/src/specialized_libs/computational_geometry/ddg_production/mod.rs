//! P09 — Operator conventions and true algebraic residuals.
//!
//! Wraps [`super::ddg_operators`] without changing `DdgReport.residual`
//! (iterate change, `maxᵢ |x⁺ᵢ − xᵢ|`). The true residual `||b − A x||` is a
//! separate, norm-declared field named `algebraic_residual`. Dense local
//! solves are size- and conditioning-gated. No coarsening engine is added.
//!
//! Sibling of `ddg_operators.rs`. Callers reuse cotangent/mass assembly and
//! Jacobi/heat solvers through the admitted APIs in this module.

mod algebraic_residual;
mod conventions;

pub use algebraic_residual::*;
pub use conventions::*;

#[cfg(test)]
mod tests;
