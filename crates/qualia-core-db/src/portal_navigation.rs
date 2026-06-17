//! GPU/CPU node picking helpers and camera fly-to for PR-C11 navigation.

use crate::portal_camera::CameraState;
use crate::portal_telemetry::ObserverStandpoint;
use crate::tensor::buffer_export::read_tensor_at;

/// Background sentinel in the R32Uint picking attachment.
pub const PICK_SENTINEL: u32 = u32::MAX;

/// Frames to interpolate camera when framing a selected node.
pub const FLY_TO_FRAMES: u32 = 24;

/// Epistemic q below this threshold is treated as collapsed (matches WGSL `Q_COLLAPSED_EPS`).
pub const Q_COLLAPSED_EPS: f32 = 0.001;

/// Active camera interpolation toward a selected node.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CameraFlyTo {
    pub target: CameraState,
    pub remaining: u32,
}

impl CameraFlyTo {
    #[inline]
    pub fn is_active(&self) -> bool {
        self.remaining > 0
    }

    pub fn start_toward(target: CameraState) -> Self {
        Self {
            target: target.clamped(),
            remaining: FLY_TO_FRAMES,
        }
    }

    /// Advance one frame; returns updated camera state.
    pub fn advance(&mut self, current: CameraState) -> CameraState {
        if self.remaining == 0 {
            return current.clamped();
        }
        let t = 1.0 - (self.remaining as f32 / FLY_TO_FRAMES as f32);
        let blended = lerp_camera(current, self.target, t.clamp(0.0, 1.0));
        self.remaining = self.remaining.saturating_sub(1);
        blended
    }
}

/// Compute orbit parameters that frame a world-space node (maps to node focal point).
#[inline]
pub fn camera_frame_node(node: [f32; 3]) -> CameraState {
    let [x, y, z] = node;
    let dist = (x * x + y * y + z * z).sqrt().max(0.05);
    let yaw = x.atan2(z);
    let pitch = (y / dist).clamp(-1.0, 1.0).asin();
    let zoom = (dist * 1.75).clamp(0.35, 48.0);
    CameraState { yaw, pitch, zoom }.clamped()
}

#[inline]
pub fn lerp_camera(a: CameraState, b: CameraState, t: f32) -> CameraState {
    CameraState {
        yaw: a.yaw + (b.yaw - a.yaw) * t,
        pitch: a.pitch + (b.pitch - a.pitch) * t,
        zoom: a.zoom + (b.zoom - a.zoom) * t,
    }
    .clamped()
}

/// Canvas2D fallback pick — nearest projected node within hit radius (px).
pub fn cpu_pick_node_at(
    tensor: &[u8],
    canvas_w: f64,
    canvas_h: f64,
    pick_x: f64,
    pick_y: f64,
    yaw: f32,
    standpoint: &ObserverStandpoint,
) -> Option<u32> {
    let count = crate::tensor::buffer_export::tensor_node_count(tensor).ok()?;
    if count == 0 {
        return None;
    }

    let mut best: Option<(u32, f64)> = None;
    for i in 0..count {
        let Ok(t) = read_tensor_at(tensor, i) else {
            continue;
        };
        if !standpoint.temporal_visible(t.t) {
            continue;
        }
        let (px, py, _) = project_xyz_canvas(t.x, t.y, t.z, canvas_w, canvas_h, yaw as f64);
        let dx = px - pick_x;
        let dy = py - pick_y;
        let hit_r = 8.0 + t.alpha as f64 * 6.0;
        let d2 = dx * dx + dy * dy;
        if d2 > hit_r * hit_r {
            continue;
        }
        if best.map_or(true, |(_, bd)| d2 < bd) {
            best = Some((i as u32, d2));
        }
    }
    best.map(|(idx, _)| idx)
}

#[inline]
fn project_xyz_canvas(x: f32, y: f32, z: f32, w: f64, h: f64, yaw: f64) -> (f64, f64, f32) {
    let cx = yaw.cos() as f32;
    let sx = yaw.sin() as f32;
    let xr = x * cx + z * sx;
    let zr = -x * sx + z * cx;
    let depth = (1.0 / (1.0 + zr * 0.35)).clamp(0.2, 1.0);
    let scale = 0.42 * w.min(h) * depth as f64;
    let px = w * 0.5 + xr as f64 * scale;
    let py = h * 0.5 - y as f64 * scale;
    (px, py, depth)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camera_frame_node_produces_finite_orbit() {
        let cam = camera_frame_node([1.0, 0.5, -2.0]);
        assert!(cam.yaw.is_finite());
        assert!(cam.pitch.is_finite());
        assert!(cam.zoom.is_finite());
    }

    #[test]
    fn fly_to_converges_toward_target() {
        let target = camera_frame_node([0.0, 1.0, 2.0]);
        let mut fly = CameraFlyTo::start_toward(target);
        let mut cam = CameraState::default();
        for _ in 0..FLY_TO_FRAMES {
            cam = fly.advance(cam);
        }
        assert!((cam.yaw - target.yaw).abs() < 0.05);
    }
}