//! Plaintext publication candidates; no disclosure grant and no signing-provider access.
mod context;
mod entries;
mod finish;
mod owner;
pub use owner::{EntryAssignment, PlainPublisher, PublicationClass, PublisherOptions};
#[cfg(not(target_arch = "wasm32"))]
mod ewm_first_paint;
#[cfg(not(target_arch = "wasm32"))]
pub use ewm_first_paint::{
    build_first_paint_hmc, publish_first_paint, CandidateIdentityHex, FirstPaintError,
    FirstPaintSource,
};
#[cfg(test)]
mod tests;
