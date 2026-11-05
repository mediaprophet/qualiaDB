//! Allocation-free KTX2 container metadata and level-range parser (native + WASM).
pub use etalon_render::texture_codec::ktx2::*;

#[cfg(test)]
#[path = "texture_ktx2_tests.rs"]
mod tests;
