//! Allocation-free decoder for uncompressed single-face RGBA8 KTX2 base levels.
pub use etalon_render::texture_codec::ktx2_rgba8::*;

#[cfg(test)]
#[path = "texture_ktx2_rgba8_tests.rs"]
mod tests;
