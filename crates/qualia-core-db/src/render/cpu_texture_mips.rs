//! Caller-buffered CPU RGBA8 mip reducer and format-specific reference filtering.
pub use etalon_render::texture_codec::cpu_mips::*;

#[cfg(test)]
#[path = "cpu_texture_mips_tests.rs"]
mod tests;
