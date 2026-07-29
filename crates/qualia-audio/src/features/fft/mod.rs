//! FFT primitives for the CPU reference path (caller-buffered, zero-heap).
//!
//! - [`fft_radix2`]: in-place power-of-two radix-2 FFT / IFFT.
//! - [`dft_direct`]: bounded direct-DFT floor for non-power-of-two `N`.
//! - [`real_fft_magnitude`]: real-input forward FFT → `N/2+1` magnitudes.
//! - [`ifft_to_real`]: inverse FFT → real time-domain samples (ISTFT helper).

pub mod inverse;
pub mod mixed_radix;
pub mod radix2;
pub mod real_fft;

pub use inverse::ifft_to_real;
pub use mixed_radix::dft_direct;
pub use radix2::fft_radix2;
pub use real_fft::real_fft_magnitude;
