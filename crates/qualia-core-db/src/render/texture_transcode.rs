//! Bounded zero-heap compressed texture transcode and block decode bridge.
pub use etalon_render::texture_codec::transcode::*;

#[cfg(test)]
#[path = "texture_bc7_conformance_tests.rs"]
mod bc7_conformance_tests;
