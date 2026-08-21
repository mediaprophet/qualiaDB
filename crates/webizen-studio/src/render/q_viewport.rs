//! Declarative `<q-viewport>` lifecycle — types and pure logic (plan §7.3 W5).
//!
//! The viewport lifecycle is:
//! 1. **Detect backend** — call `Render.gpu_backend_info` to probe WebGPU/WebGL2.
//! 2. **Init GPU** — call `Render.gpu_init` with width/height/particle_cap.
//! 3. **Frame loop** — repeatedly call `Render.gpu_render_frame` with sim-time.
//! 4. **Resize** — call `Render.gpu_resize` when the canvas dimensions change.
//! 5. **Camera** — call `Render.gpu_set_camera` on mouse/wheel input.
//! 6. **Destroy** — call `Render.gpu_destroy` on unmount.
//!
//! This module contains only the types and pure logic (no UI dependencies).
//! The async invoke functions live in the component module, which has access
//! to the `qapp_engine::invoke_json` helper.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Backend type detected by `Render.gpu_backend_info`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ViewportBackend {
    /// Native wgpu (DirectML/Vulkan/Metal) or browser WebGPU.
    Webgpu,
    /// Browser WebGL2 fallback (naga-translated GLSL ES 300).
    Webgl2,
    /// Canvas 2D fallback (no GPU acceleration).
    Canvas2d,
    /// No backend available.
    None,
}

impl Default for ViewportBackend {
    fn default() -> Self {
        Self::None
    }
}

impl ViewportBackend {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "webgpu" => Self::Webgpu,
            "webgl2" => Self::Webgl2,
            "canvas2d" => Self::Canvas2d,
            _ => Self::None,
        }
    }

    pub fn is_gpu(&self) -> bool {
        matches!(self, Self::Webgpu | Self::Webgl2)
    }
}

/// Viewport configuration — declarative props for `<q-viewport>`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewportConfig {
    /// Canvas width in physical pixels.
    pub width: u32,
    /// Canvas height in physical pixels.
    pub height: u32,
    /// Particle capacity for the ambient field.
    #[serde(default = "default_particle_cap")]
    pub particle_cap: u32,
    /// Target frame rate (frames per second). 0 = uncapped (requestAnimationFrame).
    #[serde(default = "default_target_fps")]
    pub target_fps: u32,
    /// Initial camera yaw (radians).
    #[serde(default)]
    pub camera_yaw: f32,
    /// Initial camera pitch (radians).
    #[serde(default = "default_camera_pitch")]
    pub camera_pitch: f32,
    /// Initial camera zoom.
    #[serde(default = "default_camera_zoom")]
    pub camera_zoom: f32,
    /// Whether to auto-mount on creation.
    #[serde(default = "default_auto_mount")]
    pub auto_mount: bool,
}

fn default_particle_cap() -> u32 {
    4096
}
fn default_target_fps() -> u32 {
    60
}
fn default_camera_pitch() -> f32 {
    -0.3
}
fn default_camera_zoom() -> f32 {
    1.0
}
fn default_auto_mount() -> bool {
    true
}

impl Default for ViewportConfig {
    fn default() -> Self {
        Self {
            width: 800,
            height: 600,
            particle_cap: default_particle_cap(),
            target_fps: default_target_fps(),
            camera_yaw: 0.0,
            camera_pitch: default_camera_pitch(),
            camera_zoom: default_camera_zoom(),
            auto_mount: true,
        }
    }
}

/// Viewport runtime state — tracks the GPU handle, backend, and frame count.
#[derive(Debug, Clone, Default)]
pub struct ViewportState {
    /// GPU portal handle (from `Render.gpu_init`). None = not mounted.
    pub handle: Option<u64>,
    /// Detected backend.
    pub backend: ViewportBackend,
    /// Frame count since mount.
    pub frame_count: u64,
    /// Last sim-time passed to `Render.gpu_render_frame`.
    pub last_time: f32,
    /// Whether the viewport is currently running a frame loop.
    pub running: bool,
    /// Last error message (if any).
    pub last_error: Option<String>,
}

impl ViewportState {
    /// Whether the viewport is mounted and has a valid GPU handle.
    pub fn is_mounted(&self) -> bool {
        self.handle.is_some()
    }

    /// Whether the viewport has a GPU-accelerated backend.
    pub fn is_gpu(&self) -> bool {
        self.backend.is_gpu()
    }
}

/// Extract the GPU handle from a `poet_eval` result JSON.
pub fn extract_handle(result: &Value) -> Result<u64, String> {
    // The result may have the handle in `value` (as a JSON string) or directly.
    if let Some(value_str) = result.get("value").and_then(|v| v.as_str()) {
        if let Ok(value) = serde_json::from_str::<Value>(value_str) {
            if let Some(handle) = value.get("handle").and_then(|h| h.as_u64()) {
                return Ok(handle);
            }
        }
    }
    if let Some(handle) = result.get("handle").and_then(|h| h.as_u64()) {
        return Ok(handle);
    }
    Err("no handle in gpu_init result".into())
}

/// Extract the backend string from a `poet_eval` result JSON.
pub fn extract_backend(result: &Value) -> ViewportBackend {
    if let Some(value_str) = result.get("value").and_then(|v| v.as_str()) {
        if let Ok(value) = serde_json::from_str::<Value>(value_str) {
            if let Some(backend_str) = value.get("backend").and_then(|b| b.as_str()) {
                return ViewportBackend::from_str(backend_str);
            }
        }
    }
    if let Some(backend_str) = result.get("backend").and_then(|b| b.as_str()) {
        return ViewportBackend::from_str(backend_str);
    }
    ViewportBackend::None
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn viewport_backend_from_str() {
        assert_eq!(ViewportBackend::from_str("webgpu"), ViewportBackend::Webgpu);
        assert_eq!(ViewportBackend::from_str("WebGL2"), ViewportBackend::Webgl2);
        assert_eq!(
            ViewportBackend::from_str("canvas2d"),
            ViewportBackend::Canvas2d
        );
        assert_eq!(ViewportBackend::from_str("unknown"), ViewportBackend::None);
    }

    #[test]
    fn viewport_backend_is_gpu() {
        assert!(ViewportBackend::Webgpu.is_gpu());
        assert!(ViewportBackend::Webgl2.is_gpu());
        assert!(!ViewportBackend::Canvas2d.is_gpu());
        assert!(!ViewportBackend::None.is_gpu());
    }

    #[test]
    fn viewport_config_default() {
        let config = ViewportConfig::default();
        assert_eq!(config.width, 800);
        assert_eq!(config.height, 600);
        assert_eq!(config.particle_cap, 4096);
        assert_eq!(config.target_fps, 60);
        assert!(config.auto_mount);
    }

    #[test]
    fn viewport_state_default() {
        let state = ViewportState::default();
        assert!(!state.is_mounted());
        assert!(!state.is_gpu());
        assert_eq!(state.frame_count, 0);
    }

    #[test]
    fn viewport_state_mounted() {
        let state = ViewportState {
            handle: Some(42),
            backend: ViewportBackend::Webgpu,
            ..Default::default()
        };
        assert!(state.is_mounted());
        assert!(state.is_gpu());
    }

    #[test]
    fn extract_handle_from_value() {
        let result = json!({
            "value": "{\"handle\": 123, \"width\": 800, \"height\": 600}"
        });
        assert_eq!(extract_handle(&result).unwrap(), 123);
    }

    #[test]
    fn extract_handle_direct() {
        let result = json!({ "handle": 456 });
        assert_eq!(extract_handle(&result).unwrap(), 456);
    }

    #[test]
    fn extract_handle_missing() {
        let result = json!({ "error": "something went wrong" });
        assert!(extract_handle(&result).is_err());
    }

    #[test]
    fn viewport_config_serialize_deserialize() {
        let config = ViewportConfig {
            width: 1024,
            height: 768,
            particle_cap: 8192,
            target_fps: 30,
            camera_yaw: 0.5,
            camera_pitch: -0.2,
            camera_zoom: 2.0,
            auto_mount: false,
        };
        let json_str = serde_json::to_string(&config).unwrap();
        let decoded: ViewportConfig = serde_json::from_str(&json_str).unwrap();
        assert_eq!(decoded.width, 1024);
        assert_eq!(decoded.height, 768);
        assert_eq!(decoded.particle_cap, 8192);
        assert_eq!(decoded.target_fps, 30);
        assert!(!decoded.auto_mount);
    }
}
