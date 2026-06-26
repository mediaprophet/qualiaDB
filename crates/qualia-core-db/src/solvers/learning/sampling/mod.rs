//! Monte-Carlo sampling (PRML ch 11) — the inference engine for Bayesian methods.
//!
//! - [`mcmc`] — random-walk Metropolis-Hastings over an arbitrary log-density.
//!
//! Gibbs / Hamiltonian Monte-Carlo can specialise this later (build order in
//! `stats_plan.md`).

pub mod mcmc;

pub use mcmc::{metropolis_hastings, McmcResult};
