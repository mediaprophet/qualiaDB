//! GPU/CPU node picking helpers and camera fly-to for PR-C11 navigation.

#[cfg(test)]
use crate::render::camera::orbit_forward;
use crate::render::camera::{orbit_eye_position_target, orbit_view_projection_target, CameraState};
use crate::render::pga::{sandwich_point, semantic_motor_phase2c};
use crate::render::telemetry::ObserverStandpoint;
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
    CameraState::new(yaw, pitch, zoom)
        .with_target([0.0, 0.0, 0.0])
        .clamped()
}

#[inline]
pub fn lerp_camera(a: CameraState, b: CameraState, t: f32) -> CameraState {
    CameraState {
        yaw: a.yaw + (b.yaw - a.yaw) * t,
        pitch: a.pitch + (b.pitch - a.pitch) * t,
        zoom: a.zoom + (b.zoom - a.zoom) * t,
        target: [
            a.target[0] + (b.target[0] - a.target[0]) * t,
            a.target[1] + (b.target[1] - a.target[1]) * t,
            a.target[2] + (b.target[2] - a.target[2]) * t,
        ],
        sun_dir: [
            a.sun_dir[0] + (b.sun_dir[0] - a.sun_dir[0]) * t,
            a.sun_dir[1] + (b.sun_dir[1] - a.sun_dir[1]) * t,
            a.sun_dir[2] + (b.sun_dir[2] - a.sun_dir[2]) * t,
        ],
        sun_intensity: a.sun_intensity + (b.sun_intensity - a.sun_intensity) * t,
        ambient_intensity: a.ambient_intensity + (b.ambient_intensity - a.ambient_intensity) * t,
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

/// CPU picking oracle for the GPU Tensor10D projector.
///
/// Mirrors `projector.wgsl`: semantic PGA motor (including bilateral camera pull),
/// orbit-camera view-projection, temporal rejection, clip-volume rejection and the
/// shader's alpha-scaled circular point footprint. Pixel coordinates use the same
/// top-left texel-center convention as the GPU one-pixel copy. No scratch allocation
/// is performed, so this is also suitable for the native no-GPU fallback.
pub fn cpu_pick_node_at_camera(
    tensor: &[u8],
    canvas_w: u32,
    canvas_h: u32,
    pick_x: f64,
    pick_y: f64,
    frame_time: f32,
    camera: CameraState,
    standpoint: &ObserverStandpoint,
) -> Option<u32> {
    if canvas_w == 0
        || canvas_h == 0
        || !pick_x.is_finite()
        || !pick_y.is_finite()
        || !frame_time.is_finite()
    {
        return None;
    }

    let count = crate::tensor::buffer_export::tensor_node_count(tensor).ok()?;
    if count == 0 {
        return None;
    }

    // The GPU copy clamps requests to the valid attachment extent. Sample at that
    // texel's center to match raster coverage and avoid fractional-pixel drift.
    let pixel_x = pick_x
        .floor()
        .clamp(0.0, f64::from(canvas_w.saturating_sub(1)))
        + 0.5;
    let pixel_y = pick_y
        .floor()
        .clamp(0.0, f64::from(canvas_h.saturating_sub(1)))
        + 0.5;
    let aspect = canvas_w as f32 / canvas_h as f32;
    let view_projection =
        orbit_view_projection_target(camera.yaw, camera.pitch, camera.zoom, camera.target, aspect);
    let camera_eye =
        orbit_eye_position_target(camera.yaw, camera.pitch, camera.zoom, camera.target);

    let mut best: Option<(u32, f32)> = None;
    for index in 0..count {
        let Ok(node) = read_tensor_at(tensor, index) else {
            continue;
        };
        if !node.t.is_finite()
            || !standpoint.temporal_visible(node.t)
            || !node.alpha.is_finite()
            || !node.x.is_finite()
            || !node.y.is_finite()
            || !node.z.is_finite()
        {
            continue;
        }

        // The vertex shader only rejects a node once the temporal fade reaches 0.
        let temporal_delta = (node.t - standpoint.t_slice).abs();
        let ramp_width = standpoint.t_window * 0.2;
        if !ramp_width.is_finite() || standpoint.t_window - temporal_delta <= 0.0 {
            continue;
        }

        let local = [node.x, node.y, node.z];
        let motor = semantic_motor_phase2c(
            node.v,
            node.w,
            node.q,
            node.sigma,
            frame_time,
            node.alpha,
            node.mu,
            local,
            camera_eye,
            standpoint.standpoint_class,
            standpoint.epistemic_q,
        );
        let world = sandwich_point(motor, local);
        let clip = transform_point(view_projection, world);
        if !clip.iter().all(|component| component.is_finite())
            || clip[3] <= 0.0
            || clip[2] < 0.0
            || clip[2] > clip[3]
        {
            continue;
        }

        let ndc_x = clip[0] / clip[3];
        let ndc_y = clip[1] / clip[3];
        let center_x = (f64::from(ndc_x) * 0.5 + 0.5) * f64::from(canvas_w);
        let center_y = (0.5 - f64::from(ndc_y) * 0.5) * f64::from(canvas_h);
        let point_scale = 0.012 * (0.65 + node.alpha * 0.55);
        let radius_x = f64::from(point_scale.abs()) * f64::from(canvas_w) * 0.5;
        let radius_y = f64::from(point_scale.abs()) * f64::from(canvas_h) * 0.5;
        if radius_x <= 0.0 || radius_y <= 0.0 {
            continue;
        }
        let local_x = (pixel_x - center_x) / radius_x;
        let local_y = (pixel_y - center_y) / radius_y;
        if local_x * local_x + local_y * local_y > 1.0 {
            continue;
        }

        let depth = clip[2] / clip[3];
        // The GPU uses strict Less depth testing: an exactly coplanar earlier
        // instance remains visible, while a nearer point replaces a farther one.
        if best.map_or(true, |(_, best_depth)| depth < best_depth) {
            best = Some((index as u32, depth));
        }
    }
    best.map(|(index, _)| index)
}

#[inline]
fn transform_point(matrix: [[f32; 4]; 4], point: [f32; 3]) -> [f32; 4] {
    [
        matrix[0][0] * point[0] + matrix[1][0] * point[1] + matrix[2][0] * point[2] + matrix[3][0],
        matrix[0][1] * point[0] + matrix[1][1] * point[1] + matrix[2][1] * point[2] + matrix[3][1],
        matrix[0][2] * point[0] + matrix[1][2] * point[1] + matrix[2][2] * point[2] + matrix[3][2],
        matrix[0][3] * point[0] + matrix[1][3] * point[1] + matrix[2][3] * point[2] + matrix[3][3],
    ]
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
    use crate::render::telemetry::STANDPOINT_SPECTATOR;
    use crate::tensor::buffer_export::{write_tensor_buffer, TensorBufferHeader};
    use crate::tensor::Tensor10D;

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

    fn tensor_bytes(nodes: &[Tensor10D]) -> Vec<u8> {
        let mut bytes = vec![0; TensorBufferHeader::total_bytes(nodes.len())];
        write_tensor_buffer(nodes, &mut bytes).expect("tensor export");
        bytes
    }

    fn pick_observer() -> ObserverStandpoint {
        ObserverStandpoint::new(0, 0, STANDPOINT_SPECTATOR, 1.0, 0.0, 10.0, 1, 0)
    }

    fn euclidean_node(x: f32, y: f32, z: f32) -> Tensor10D {
        Tensor10D {
            q: 0.0,
            v: 0.0,
            w: 0.0,
            x,
            y,
            z,
            t: 0.0,
            alpha: 1.0,
            mu: 0.0,
            sigma: 0.0,
        }
    }

    #[test]
    fn gpu_oracle_picks_nearest_overlapping_projected_node() {
        let camera = CameraState::default();
        let forward = orbit_forward(camera.yaw, camera.pitch);
        // Put the second point closer along the optical axis so both projected
        // discs overlap exactly while their depth values differ.
        let near = [-forward[0] * 0.5, -forward[1] * 0.5, -forward[2] * 0.5];
        let bytes = tensor_bytes(&[
            euclidean_node(0.0, 0.0, 0.0),
            euclidean_node(near[0], near[1], near[2]),
        ]);
        let picked = cpu_pick_node_at_camera(
            &bytes,
            800,
            600,
            400.0,
            300.0,
            0.0,
            camera,
            &pick_observer(),
        );
        assert_eq!(picked, Some(1), "nearer depth-tested point should win");
    }

    #[test]
    fn gpu_oracle_matches_shader_point_footprint_and_pixel_centres() {
        let bytes = tensor_bytes(&[euclidean_node(0.0, 0.0, 0.0)]);
        let pick = |x| {
            cpu_pick_node_at_camera(
                &bytes,
                800,
                600,
                x,
                300.0,
                0.0,
                CameraState::default(),
                &pick_observer(),
            )
        };
        assert_eq!(pick(405.0), Some(0), "pixel is inside the shader disc");
        assert_eq!(pick(407.0), None, "pixel lies outside the shader disc");
    }

    #[test]
    fn gpu_oracle_rejects_behind_camera_and_temporally_hidden_nodes() {
        let camera = CameraState::default();
        let eye = orbit_eye_position_target(camera.yaw, camera.pitch, camera.zoom, camera.target);
        let bytes = tensor_bytes(&[
            euclidean_node(0.0, 0.0, 8.0), // Behind the default orbit camera.
            euclidean_node(eye[0], eye[1], eye[2]), // On the near-plane eye singularity.
            Tensor10D {
                t: 20.0,
                ..euclidean_node(0.0, 0.0, 0.0)
            },
            Tensor10D {
                t: 10.0,
                ..euclidean_node(0.0, 0.0, 0.0)
            }, // Zero-fade temporal boundary.
        ]);
        assert_eq!(
            cpu_pick_node_at_camera(
                &bytes,
                800,
                600,
                400.0,
                300.0,
                0.0,
                camera,
                &pick_observer(),
            ),
            None
        );
    }

    #[test]
    fn gpu_oracle_rejects_invalid_extents_and_non_finite_pointer() {
        let bytes = tensor_bytes(&[euclidean_node(0.0, 0.0, 0.0)]);
        assert_eq!(
            cpu_pick_node_at_camera(
                &bytes,
                0,
                600,
                0.0,
                0.0,
                0.0,
                CameraState::default(),
                &pick_observer(),
            ),
            None
        );
        assert_eq!(
            cpu_pick_node_at_camera(
                &bytes,
                800,
                600,
                f64::NAN,
                300.0,
                0.0,
                CameraState::default(),
                &pick_observer(),
            ),
            None
        );
    }
}
