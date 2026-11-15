//! Retained current QAC observations for CPU/browser adapters.
//! Admission is cold and bounded; immutable windows never rehash or allocate.
//! No audio normalization, pressure conversion, playback, or synthesized previews.
mod model;
mod retained;
mod window;
pub use model::{provenance_role_str, Error, Limits, Metadata};
pub use retained::RetainedQac;
pub use window::{PcmWindow, StftWindow};
#[cfg(test)]
mod tests;
