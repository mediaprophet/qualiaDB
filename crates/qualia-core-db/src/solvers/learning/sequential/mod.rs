//! Sequential / temporal models (PRML ch 13) — estimators over time-indexed data.
//!
//! - [`hmm`] — discrete Hidden Markov Model (scaled forward, Viterbi, Baum-Welch).
//!
//! Linear dynamical systems / Kalman filtering land here next (build order in
//! `stats_plan.md`).

pub mod hmm;
pub mod kalman;

pub use hmm::{baum_welch, Hmm};
pub use kalman::KalmanFilter;
