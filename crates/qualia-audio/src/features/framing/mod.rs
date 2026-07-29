//! Framing / overlap-add / ISTFT resynthesis (CPU reference path).
//!
//! - [`frame_count`] / [`cut_frame`] — explicit framing geometry and a
//!   caller-buffered frame cutter.
//! - [`overlap_add`] — sum pre-windowed frames back at a fixed hop (COLA).
//! - [`istft`] — inverse STFT via IFFT + weighted overlap-add (WOLA), normalised
//!   by the analysis·synthesis window envelope for exact reconstruction.
//! - [`griffin_lim`] — magnitude-only reconstruction by alternating projection.
//!
//! All hot paths are caller-buffered and zero-heap; the FFT and window primitives
//! are reused from [`crate::features::fft`] and [`crate::features::window`].

pub mod frame_cutter;
pub mod griffin_lim;
pub mod istft;
pub mod overlap_add;

pub use frame_cutter::{cut_frame, frame_count};
pub use griffin_lim::griffin_lim;
pub use istft::istft;
pub use overlap_add::overlap_add;
