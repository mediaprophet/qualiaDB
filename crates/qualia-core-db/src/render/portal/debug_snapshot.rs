//! Publish live portal occupancy plus GLES HAL unpack counters.
use super::*;
use serde_json::json;

#[wasm_bindgen]
impl QualiaPortal {
    /// Observable sky / water / mesh occupancy and GLES unpack counters.
    /// Cold diagnostic path only (host receipt cadence). HAL counters on the
    /// render submit path are atomics.
    pub fn debug_scene_snapshot(&self) -> Result<JsValue, JsValue> {
        let mut value = match self.gpu.as_ref() {
            Some(gpu) => gpu.debug_scene_snapshot(),
            None => json!({
                "schema": "webizen.portal-scene-debug/v1",
                "gpu": false,
                "reason": "No adopted PortalGpu",
            }),
        };
        value["body_renderer"] = json!(self.body_renderer.as_str());
        value["body_vertex_count"] = json!(self.body_vertex_count);
        value["body_index_count"] = json!(self.body_index_count);
        value["body_frames_presented"] = json!(self.body_frames_presented);
        value["emf_display_state"] = json!(self.last_emf.status().as_str());
        value["tier"] = json!(self.tier);
        value["gles_hal"] = gles_hal_debug();
        js_sys::JSON::parse(&value.to_string())
    }
}

fn gles_hal_debug() -> serde_json::Value {
    let snap = etalon::hal::gles_debug::snapshot();
    json!({
        "copy_buffer_to_texture_pbo": snap.copy_buffer_to_texture_pbo,
        "copy_buffer_to_texture_slice": snap.copy_buffer_to_texture_slice,
        "copy_compressed_pbo": snap.copy_compressed_pbo,
        "copy_compressed_slice": snap.copy_compressed_slice,
        "bind_emulated_storage_uploads": snap.bind_emulated_storage_uploads,
        "bind_emulated_storage_skips": snap.bind_emulated_storage_skips,
        "bind_emulated_storage_texels": snap.bind_emulated_storage_texels,
        "bind_emulated_storage_bytes": snap.bind_emulated_storage_bytes,
        "bind_emulated_storage_remainder_rows": snap.bind_emulated_storage_remainder_rows,
        "last_copy": {
            "kind": snap.last_copy_kind,
            "width": snap.last_copy_width,
            "height": snap.last_copy_height,
            "unpack_row_length": snap.last_copy_unpack_row_length,
            "unpack_image_height": snap.last_copy_unpack_image_height,
            "needed_bytes": snap.last_copy_needed_bytes,
            "take_bytes": snap.last_copy_take_bytes,
        },
        "last_emulated_storage": {
            "slot": snap.last_emu_slot,
            "offset": snap.last_emu_offset,
            "size": snap.last_emu_size,
            "texels": snap.last_emu_texels,
            "width": snap.last_emu_width,
            "height": snap.last_emu_height,
            "full_rows": snap.last_emu_full_rows,
            "remainder_texels": snap.last_emu_remainder_texels,
        },
    })
}
