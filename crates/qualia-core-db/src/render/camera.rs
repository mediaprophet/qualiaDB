//! Orbit camera → column-major `view_projection` for the Qualia portal (CPU hot path, zero-heap).

use crate::render::telemetry::CameraUniform;

/// Interactive orbit state driven from JS (`set_camera`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CameraState {
    pub yaw: f32,
    pub pitch: f32,
    pub zoom: f32,
}

impl Default for CameraState {
    fn default() -> Self {
        Self {
            yaw: 0.0,
            pitch: 0.25,
            zoom: 3.5,
        }
    }
}

impl CameraState {
    #[inline]
    pub fn clamped(mut self) -> Self {
        const PITCH_LIMIT: f32 = 1.45;
        self.pitch = self.pitch.clamp(-PITCH_LIMIT, PITCH_LIMIT);
        self.zoom = self.zoom.clamp(0.35, 48.0);
        self
    }

    /// Build GPU uniform block (128 B) including pre-multiplied view×projection.
    pub fn to_uniform(&self, aspect: f32, tensor_mode: bool) -> CameraUniform {
        let state = self.clamped();
        let eye = orbit_eye_position(state.yaw, state.pitch, state.zoom);
        let mut padding = [0.0_f32; 12];
        padding[1] = eye[0];
        padding[2] = eye[1];
        padding[3] = eye[2];
        CameraUniform {
            view_projection: orbit_view_projection(state.yaw, state.pitch, state.zoom, aspect),
            yaw: state.yaw,
            pitch: state.pitch,
            zoom: state.zoom,
            tensor_mode: if tensor_mode { 1 } else { 0 },
            _padding: padding,
        }
    }
}

/// Orbit camera eye position — matches the `look_at` used in [`orbit_view_projection`].
#[inline]
pub fn orbit_eye_position(yaw: f32, pitch: f32, zoom: f32) -> [f32; 3] {
    let pitch = pitch.clamp(-1.45, 1.45);
    let dist = zoom.clamp(0.35, 48.0);
    let cy = yaw.cos();
    let sy = yaw.sin();
    let cp = pitch.cos();
    let sp = pitch.sin();
    [dist * cp * sy, dist * sp, dist * cp * cy]
}

/// Column-major view×projection (WGSL `mat4x4<f32>` compatible).
pub fn orbit_view_projection(yaw: f32, pitch: f32, zoom: f32, aspect: f32) -> [[f32; 4]; 4] {
    let yaw = yaw;
    let pitch = pitch.clamp(-1.45, 1.45);
    let dist = zoom.clamp(0.35, 48.0);
    let aspect = aspect.max(0.05);

    let cy = yaw.cos();
    let sy = yaw.sin();
    let cp = pitch.cos();
    let sp = pitch.sin();

    let eye_x = dist * cp * sy;
    let eye_y = dist * sp;
    let eye_z = dist * cp * cy;

    let view = look_at_rh([eye_x, eye_y, eye_z], [0.0, 0.0, 0.0], [0.0, 1.0, 0.0]);
    let proj = perspective_rh_gl(45.0_f32.to_radians(), aspect, 0.05, 200.0);
    mat4_mul(proj, view)
}

#[inline]
fn look_at_rh(eye: [f32; 3], center: [f32; 3], up: [f32; 3]) -> [[f32; 4]; 4] {
    let f = normalize(sub(center, eye));
    let s = normalize(cross(f, up));
    let u = cross(s, f);

    [
        [s[0], u[0], -f[0], 0.0],
        [s[1], u[1], -f[1], 0.0],
        [s[2], u[2], -f[2], 0.0],
        [-dot(s, eye), -dot(u, eye), dot(f, eye), 1.0],
    ]
}

#[inline]
fn perspective_rh_gl(fov_y: f32, aspect: f32, near: f32, far: f32) -> [[f32; 4]; 4] {
    let f = 1.0 / (fov_y * 0.5).tan();
    let nf = 1.0 / (near - far);
    [
        [f / aspect, 0.0, 0.0, 0.0],
        [0.0, f, 0.0, 0.0],
        [0.0, 0.0, (far + near) * nf, -1.0],
        [0.0, 0.0, 2.0 * far * near * nf, 0.0],
    ]
}

#[inline]
fn mat4_mul(a: [[f32; 4]; 4], b: [[f32; 4]; 4]) -> [[f32; 4]; 4] {
    let mut out = [[0.0; 4]; 4];
    for c in 0..4 {
        for r in 0..4 {
            out[c][r] =
                a[0][r] * b[c][0] + a[1][r] * b[c][1] + a[2][r] * b[c][2] + a[3][r] * b[c][3];
        }
    }
    out
}

#[inline]
fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

#[inline]
fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

#[inline]
fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

#[inline]
fn normalize(v: [f32; 3]) -> [f32; 3] {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if len < 1e-6 {
        return [0.0, 0.0, 1.0];
    }
    [v[0] / len, v[1] / len, v[2] / len]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn view_projection_finite_at_defaults() {
        let m = orbit_view_projection(0.0, 0.25, 3.5, 16.0 / 9.0);
        assert!(m[0][0].is_finite());
        assert!(m[3][2].is_finite());
    }

    #[test]
    fn camera_uniform_is_128_bytes() {
        let u = CameraState::default().to_uniform(1.0, true);
        assert_eq!(std::mem::size_of::<CameraUniform>(), 128);
        let bytes = bytemuck::bytes_of(&u);
        assert_eq!(bytes.len(), 128);
    }

    #[test]
    fn camera_uniform_carries_eye_position() {
        let state = CameraState {
            yaw: 0.5,
            pitch: 0.25,
            zoom: 4.0,
        };
        let u = state.to_uniform(16.0 / 9.0, true);
        let eye = orbit_eye_position(state.yaw, state.pitch, state.zoom);
        assert!((u._padding[1] - eye[0]).abs() < 1e-5);
        assert!((u._padding[2] - eye[1]).abs() < 1e-5);
        assert!((u._padding[3] - eye[2]).abs() < 1e-5);
    }
}
