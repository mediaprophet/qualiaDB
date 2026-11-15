//! Deterministic, allocation-free mip admission for texture streaming.
pub use etalon_render::texture_codec::streaming_plan::*;

#[cfg(test)]
fn mip_extent(base: u32, level: u32) -> u32 {
    base.checked_shr(level).unwrap_or(0).max(1)
}

#[cfg(test)]
#[path = "texture_streaming_plan_tests.rs"]
mod tests;
