//! Experiment design & evaluation (Practical Statistics ch 3) — planning and
//! analysing experiments, with uncertainty made explicit.
//!
//! - [`power`] — power analysis & required sample size.
//! - [`ab_test`] — A/B testing (two-proportion comparison with a lift CI).
//! - [`bandit`] — multi-armed bandits (ε-greedy / UCB1 / Thompson sampling).
//!
//! Reuses `statistics::distributions::normal`; no new silo.

pub mod ab_test;
pub mod bandit;
pub mod power;

pub use ab_test::{ab_test, AbResult};
pub use bandit::{Bandit, Policy};
pub use power::{power_two_sample, required_sample_size_two_proportion, required_sample_size_two_sample};
