//! Fixed-size ambient-visibility uniforms and target sizing.

use super::AoUniform;
use crate::render::camera::{CAMERA_FAR_PLANE, CAMERA_TAN_HALF_FOV};

pub fn make_uniform(
    width: u32,
    height: u32,
    eye: [f32; 3],
    camera_forward: [f32; 3],
    radius: f32,
    strength: f32,
    bias: f32,
    sample_count: u32,
    enabled: bool,
) -> AoUniform {
    let aspect = width.max(1) as f32 / height.max(1) as f32;
    let (width, height) = half_extent(width, height);
    let sample_count = match sample_count {
        0..=4 => 4,
        5..=8 => 8,
        _ => 12,
    };
    let (forward, right, up) = camera_basis(camera_forward);
    AoUniform {
        viewport: [
            width as f32,
            height as f32,
            1.0 / width as f32,
            1.0 / height as f32,
        ],
        eye: [eye[0], eye[1], eye[2], sample_count as f32],
        camera_forward: [forward[0], forward[1], forward[2], 0.0],
        camera_right: [right[0], right[1], right[2], 0.0],
        camera_up: [up[0], up[1], up[2], 0.0],
        params: [
            radius.clamp(0.05, 2.0),
            strength.clamp(0.0, 1.0),
            bias.clamp(0.0, 0.2),
            if enabled { 1.0 } else { 0.0 },
        ],
        depth: [
            CAMERA_FAR_PLANE,
            CAMERA_FAR_PLANE / 65_534.0,
            CAMERA_TAN_HALF_FOV,
            aspect,
        ],
    }
}

fn camera_basis(forward: [f32; 3]) -> ([f32; 3], [f32; 3], [f32; 3]) {
    let forward = normalize_or(forward, [0.0, 0.0, -1.0]);
    let world_up = if forward[1].abs() > 0.999 {
        [0.0, 0.0, 1.0]
    } else {
        [0.0, 1.0, 0.0]
    };
    let right = normalize_or(cross(forward, world_up), [1.0, 0.0, 0.0]);
    let up = normalize_or(cross(right, forward), [0.0, 1.0, 0.0]);
    (forward, right, up)
}

fn normalize_or(value: [f32; 3], fallback: [f32; 3]) -> [f32; 3] {
    let length = dot(value, value).sqrt();
    if length.is_finite() && length > 1e-6 {
        scale(value, length.recip())
    } else {
        fallback
    }
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn scale(value: [f32; 3], factor: f32) -> [f32; 3] {
    [value[0] * factor, value[1] * factor, value[2] * factor]
}

pub(super) fn half_extent(width: u32, height: u32) -> (u32, u32) {
    (width.div_ceil(2).max(1), height.div_ceil(2).max(1))
}

pub(super) fn target_bytes(width: u32, height: u32) -> Option<u64> {
    let (width, height) = half_extent(width, height);
    target_bytes_at_half(width, height)
}

pub(super) fn target_bytes_at_half(width: u32, height: u32) -> Option<u64> {
    u64::from(width)
        .checked_mul(u64::from(height))?
        .checked_mul(9)
}

#[cfg(test)]
mod tests {
    use super::{half_extent, make_uniform, target_bytes, CAMERA_FAR_PLANE, CAMERA_TAN_HALF_FOV};

    #[test]
    fn target_sizing_is_half_resolution_and_checked() {
        assert_eq!(half_extent(1920, 1080), (960, 540));
        assert_eq!(target_bytes(1920, 1080), Some(4_665_600));
        assert_eq!(target_bytes(1, 1), Some(9));
        assert_eq!(target_bytes(u32::MAX, u32::MAX), None);
    }

    #[test]
    fn camera_basis_and_uniform_parameters_are_bounded() {
        let uniform = make_uniform(1, 1, [0.0; 3], [0.0, 0.0, -1.0], 0.5, 0.7, 0.02, 8, true);
        assert_eq!(uniform.viewport, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(uniform.eye[3], 8.0);
        assert_eq!(uniform.camera_forward, [0.0, 0.0, -1.0, 0.0]);
        assert_eq!(uniform.camera_right, [1.0, -0.0, 0.0, 0.0]);
        assert_eq!(uniform.camera_up, [0.0, 1.0, 0.0, 0.0]);
        assert_eq!(uniform.params, [0.5, 0.7, 0.02, 1.0]);
        assert_eq!(
            uniform.depth,
            [
                CAMERA_FAR_PLANE,
                CAMERA_FAR_PLANE / 65_534.0,
                CAMERA_TAN_HALF_FOV,
                1.0,
            ]
        );
    }

    #[test]
    fn odd_viewport_keeps_projection_aspect_and_camera_basis_is_orthonormal() {
        let uniform = make_uniform(
            65,
            31,
            [2.0, 1.0, 3.0],
            [-0.3, -0.4, -0.8660254],
            0.5,
            0.5,
            0.01,
            4,
            true,
        );
        assert_eq!(uniform.viewport, [33.0, 16.0, 1.0 / 33.0, 1.0 / 16.0]);
        assert!((uniform.depth[3] - 65.0 / 31.0).abs() < 1e-6);
        let basis = [
            &uniform.camera_forward[..3],
            &uniform.camera_right[..3],
            &uniform.camera_up[..3],
        ];
        for vector in basis {
            let length = vector
                .iter()
                .map(|component| component * component)
                .sum::<f32>();
            assert!((length - 1.0).abs() < 1e-5);
        }
        for left in 0..basis.len() {
            for right in left + 1..basis.len() {
                let product = (0..3)
                    .map(|axis| basis[left][axis] * basis[right][axis])
                    .sum::<f32>();
                assert!(product.abs() < 1e-5);
            }
        }
    }

    #[test]
    fn analytic_depth_reconstruction_reprojects_to_odd_viewport_pixel_centres() {
        let (yaw, pitch, zoom, target) = (0.73_f32, -0.42_f32, 8.0_f32, [3.0, -1.0, 2.0]);
        let aspect = 65.0 / 31.0;
        let eye = crate::render::camera::orbit_eye_position_target(yaw, pitch, zoom, target);
        let forward = crate::render::camera::orbit_forward(yaw, pitch);
        let uniform = make_uniform(65, 31, eye, forward, 0.5, 0.5, 0.01, 8, true);
        let pixel = [7_i32, 9_i32];
        let view_depth = 4.25;
        let uv = [
            (pixel[0] as f32 + 0.5) / uniform.viewport[0],
            (pixel[1] as f32 + 0.5) / uniform.viewport[1],
        ];
        let ndc = [uv[0] * 2.0 - 1.0, 1.0 - uv[1] * 2.0];
        let world: [f32; 3] = std::array::from_fn(|axis| {
            eye[axis]
                + uniform.camera_forward[axis] * view_depth
                + uniform.camera_right[axis] * (ndc[0] * aspect * CAMERA_TAN_HALF_FOV * view_depth)
                + uniform.camera_up[axis] * (ndc[1] * CAMERA_TAN_HALF_FOV * view_depth)
        });
        let view_projection =
            crate::render::camera::orbit_view_projection_target(yaw, pitch, zoom, target, aspect);
        let world_h = [world[0], world[1], world[2], 1.0];
        let clip: [f32; 4] = std::array::from_fn(|row| {
            (0..4)
                .map(|column| view_projection[column][row] * world_h[column])
                .sum::<f32>()
        });
        let projected_uv = [
            (clip[0] / clip[3] + 1.0) * 0.5,
            (1.0 - clip[1] / clip[3]) * 0.5,
        ];
        assert!((projected_uv[0] - uv[0]).abs() < 2e-5);
        assert!((projected_uv[1] - uv[1]).abs() < 2e-5);
    }

    #[test]
    fn ao_samples_normalize_to_supported_quality_tiers() {
        let make = |sample_count| {
            make_uniform(
                4,
                4,
                [0.0; 3],
                [0.0, 0.0, -1.0],
                0.5,
                0.5,
                0.01,
                sample_count,
                true,
            )
            .eye[3]
        };
        assert_eq!(make(0), 4.0);
        assert_eq!(make(4), 4.0);
        assert_eq!(make(5), 8.0);
        assert_eq!(make(8), 8.0);
        assert_eq!(make(9), 12.0);
        assert_eq!(make(u32::MAX), 12.0);
    }
}
