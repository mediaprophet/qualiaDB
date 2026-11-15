//! Bounded screen-space ambient occlusion quality profiles and bilateral reconstruction.
pub use etalon_render::frame_quality::ao_quality::*;

#[cfg(test)]
#[path = "ao_quality_tests.rs"]
mod tests;
