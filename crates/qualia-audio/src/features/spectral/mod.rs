//! Spectral descriptors (flux, HFC, rolloff, flatness, contrast, LPC, …).
//!
//! Each leaf module owns exactly one public function over a caller-supplied
//! spectrum slice, returning a scalar or writing into caller buffers
//! (zero-heap hot path). Re-exports only (AU-SPEC-DESC).

pub mod complexity;
pub mod contrast;
pub mod energy_band;
pub mod energy_band_ratio;
pub mod flatness_db;
pub mod flux;
pub mod frequency_bands;
pub mod hfc;
pub mod log_spectrum;
pub mod lpc;
pub mod panning;
pub mod power_spectrum;
pub mod rolloff;

pub use complexity::spectral_complexity;
pub use contrast::spectral_contrast;
pub use energy_band::energy_band;
pub use energy_band_ratio::energy_band_ratio;
pub use flatness_db::spectral_flatness_db;
pub use flux::spectral_flux;
pub use frequency_bands::frequency_bands;
pub use hfc::high_frequency_content;
pub use log_spectrum::log_spectrum;
pub use lpc::{lpc, MAX_LPC_ORDER};
pub use panning::panning;
pub use power_spectrum::power_spectrum;
pub use rolloff::spectral_rolloff;
