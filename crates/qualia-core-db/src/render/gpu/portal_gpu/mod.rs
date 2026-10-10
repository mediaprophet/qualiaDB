//! Decomposed portal GPU lifecycle, pass dispatch, and execution submodules.

pub(super) mod lighting_passes;
pub(super) mod picking;
pub(super) mod readback;
pub(super) mod render_frame;
pub(super) mod uniforms;

#[cfg(all(test, not(target_arch = "wasm32")))]
pub(super) mod tests;
