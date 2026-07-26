//! Render surface descriptor for Chora canvas proxy (Phase 6).
//!
//! Exposes the WebGPU/canvas2d surface configuration that qApps use to
//! initialise their renderer against the active world + temporal slice.

use serde::{Deserialize, Serialize};

/// Supported render backends for the Chora canvas proxy.
pub const BACKEND_WEBGPU: &str = "webgpu";
pub const BACKEND_CANVAS2D: &str = "canvas2d";

/// Serializable surface configuration passed to the qApp renderer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RenderSurfaceDescriptor {
    pub width: u32,
    pub height: u32,
    /// `"webgpu"` or `"canvas2d"`.
    pub backend: String,
    /// Current temporal scrub position (unix seconds).
    pub temporal_t: u64,
    /// Active canvas world identifier.
    pub active_world_id: String,
    /// Reference-frame origin latitude (degrees).
    pub origin_lat: f64,
    /// Reference-frame origin longitude (degrees).
    pub origin_lon: f64,
}

/// Build a validated render surface descriptor from navigation + canvas state.
pub fn build_surface_descriptor(
    width: u32,
    height: u32,
    backend: &str,
    temporal_t: u64,
    active_world_id: &str,
    origin_lat: f64,
    origin_lon: f64,
) -> Result<RenderSurfaceDescriptor, String> {
    if width == 0 || height == 0 {
        return Err("surface dimensions must be non-zero".into());
    }
    let backend = match backend {
        BACKEND_WEBGPU | BACKEND_CANVAS2D => backend.to_string(),
        other => {
            return Err(format!(
                "unsupported render backend '{other}'; expected webgpu or canvas2d"
            ))
        }
    };
    if active_world_id.trim().is_empty() {
        return Err("active_world_id must not be empty".into());
    }
    Ok(RenderSurfaceDescriptor {
        width,
        height,
        backend,
        temporal_t,
        active_world_id: active_world_id.to_string(),
        origin_lat,
        origin_lon,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_surface_descriptor_roundtrip() {
        let desc = build_surface_descriptor(
            1280,
            720,
            BACKEND_WEBGPU,
            1_750_000_000,
            "q42:world:demo-offline",
            -33.8688,
            151.2093,
        )
        .unwrap();
        assert_eq!(desc.width, 1280);
        assert_eq!(desc.height, 720);
        assert_eq!(desc.backend, BACKEND_WEBGPU);
        assert_eq!(desc.temporal_t, 1_750_000_000);
        assert_eq!(desc.active_world_id, "q42:world:demo-offline");
        let json = serde_json::to_string(&desc).unwrap();
        let parsed: RenderSurfaceDescriptor = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, desc);
    }

    #[test]
    fn rejects_invalid_backend() {
        assert!(build_surface_descriptor(800, 600, "opengl", 0, "world", 0.0, 0.0).is_err());
    }

    #[test]
    fn rejects_zero_dimensions() {
        assert!(build_surface_descriptor(0, 600, BACKEND_CANVAS2D, 0, "world", 0.0, 0.0).is_err());
    }
}
