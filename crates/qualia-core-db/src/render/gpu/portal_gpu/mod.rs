//! Decomposed portal GPU lifecycle, pass dispatch, and execution submodules.

pub(super) mod constructors;
pub(super) mod holograph;
pub(super) mod init;
pub(super) mod lighting_passes;
pub(super) mod picking;
pub(super) mod readback;
pub(super) mod render_frame;
pub(super) mod settings;
pub(super) mod tensor_data;
pub(super) mod uniforms;

#[cfg(all(test, not(target_arch = "wasm32")))]
pub(super) mod tests;

/// Effective read-only storage buffer binding limit in bytes.
///
/// On backends without native SSBOs (WebGL2/GLES with `READ_ONLY_STORAGE_EMULATION`),
/// `max_storage_buffer_binding_size` is `0`, while `etalon-core` admits read-only storage
/// bindings up to the `RGBA32UI` 2D shadow texture capacity (`max_texture_dimension_2d^2 * 16`).
pub(super) fn effective_readonly_storage_binding_size(device: &etalon::Device) -> u64 {
    let limits = device.limits();
    let native = u64::from(limits.max_storage_buffer_binding_size);
    if native > 0 {
        return native;
    }
    let dim = u64::from(limits.max_texture_dimension_2d);
    dim.saturating_mul(dim)
        .saturating_mul(16)
        .min(i32::MAX as u64)
}
