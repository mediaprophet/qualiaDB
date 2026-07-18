//! Peak detection / spectral peaks / correlation (caller-buffered, zero-heap
//! hot path). Each leaf module holds exactly one public function.

pub mod autocorrelation;
pub mod crosscorrelation;
pub mod max_mag_freq;
pub mod peak_detection;
pub mod spectral_peaks;

pub use autocorrelation::autocorrelation;
pub use crosscorrelation::crosscorrelation;
pub use max_mag_freq::max_magnitude_frequency;
pub use peak_detection::detect_peaks;
pub use spectral_peaks::spectral_peaks;
