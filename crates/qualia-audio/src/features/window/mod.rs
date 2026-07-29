//! Window functions (periodic / DFT-even convention) and application.
//!
//! Each generator fills a caller-supplied `&mut [f32]` of the desired length;
//! [`apply_window`] multiplies a sample frame by a window in place.

pub mod apply;
pub mod blackman;
pub mod blackman_harris;
pub mod hamming;
pub mod hann;

pub use apply::apply_window;
pub use blackman::blackman_window;
pub use blackman_harris::blackman_harris_window;
pub use hamming::hamming_window;
pub use hann::hann_window;
