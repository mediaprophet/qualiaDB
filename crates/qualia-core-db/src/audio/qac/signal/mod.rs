//! Explicit candidate signal profiles; no default writer or implicit legacy upgrade.
mod encode;
mod model;
mod view;
pub use encode::encode;
pub use model::*;
pub use view::{admit, SignalView};
#[cfg(test)]
pub(super) mod test_support;
#[cfg(test)]
mod tests;

#[cfg(test)]
mod spectral_tests;

mod samples;
pub use samples::decode_sample;

#[cfg(test)]
mod review_tests;

/// Current-format original PCM document producer.
pub mod writer;
/// Current-format Hann STFT provider and producer.
pub mod stft;
/// Mixed-domain Schema 6/7 acquisition and signal admission bridge.
pub mod acquisition_bridge;
pub use acquisition_bridge::*;
