//! IIR filter bank — biquad engine + RBJ cookbook designs + streaming helpers.
//!
//! - [`biquad`] — the shared second-order engine (`BiquadCoeffs` + `BiquadState`).
//! - [`lowpass`] / [`highpass`] / [`bandpass`] / [`bandreject`] / [`allpass`] —
//!   RBJ Audio-EQ-Cookbook coefficient designs feeding the engine.
//! - [`dc_removal`] — first-order DC blocker.
//! - [`moving_average`] / [`median_filter`] / [`max_filter`] — windowed
//!   time-domain filters (caller-buffered, zero-heap).
//!
//! All hot paths are caller-buffered with fixed-size stack state — no per-call
//! heap allocation.

pub mod allpass;
pub mod bandpass;
pub mod bandreject;
pub mod biquad;
pub mod dc_removal;
pub mod highpass;
pub mod lowpass;
pub mod max_filter;
pub mod median_filter;
pub mod moving_average;

pub use allpass::design_allpass;
pub use bandpass::design_bandpass;
pub use bandreject::design_bandreject;
pub use biquad::{BiquadCoeffs, BiquadState};
pub use dc_removal::DcBlocker;
pub use highpass::design_highpass;
pub use lowpass::design_lowpass;
pub use max_filter::max_filter;
pub use median_filter::{median_filter, MAX_MEDIAN_WINDOW};
pub use moving_average::moving_average;
