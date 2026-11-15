use super::*;
use crate::render::camera::orbit_eye_position_target;
use crate::render::skeletal::Motor;
#[test]
fn orbit_placement_preserves_mounted_eye_after_native_clamping() {
    let body = Motor::from_translation(10.0, 2.0, 4.0);
    let mount =
        mounted_camera(body, [0.34, 1.40, 0.05], [0.3, 0.05], [2.0, 0.0, 1.0], 0.04).unwrap();
    let rendered = orbit_eye_position_target(mount.yaw, mount.pitch, mount.zoom, mount.target);
    for i in 0..3 {
        assert!((rendered[i] - mount.eye[i]).abs() < 1e-6);
    }
    assert!((mount.eye[0] - 0.3336).abs() < 1e-6);
    assert!((mount.eye[1] - 0.136).abs() < 1e-6);
}
#[test]
fn body_rotation_carries_driver_mount_and_forward_direction() {
    let half = std::f64::consts::FRAC_PI_4;
    let body = Motor::from_rotation_translation([half.cos(), 0.0, half.sin(), 0.0], [0.0; 3]);
    let mount = mounted_camera(body, [0.3, 1.2, 0.0], [0.0, 0.0], [0.0; 3], 1.0).unwrap();
    assert!(mount.eye[0].abs() < 1e-6);
    assert!((mount.eye[2] + 0.3).abs() < 1e-6);
    assert!((mount.yaw - std::f32::consts::FRAC_PI_2).abs() < 1e-6);
}
#[test]
fn invalid_normalization_is_refused() {
    assert!(mounted_camera(
        Motor::from_translation(0.0, 0.0, 0.0),
        [0.0; 3],
        [0.0; 2],
        [0.0; 3],
        0.0
    )
    .is_err());
}
#[test]
fn close_lens_keeps_dashboard_depth_inside_clip_volume() {
    let default = crate::render::camera::CameraState::new(0.0, 0.0, 0.35);
    let close = default.with_near_plane(0.0014);
    let point = [0.0, 0.0, 0.34, 1.0];
    let project = |camera: crate::render::camera::CameraState| {
        let matrix = camera.view_projection(16.0 / 9.0);
        std::array::from_fn::<_, 4, _>(|row| {
            (0..4).map(|col| matrix[col][row] * point[col]).sum::<f32>()
        })
    };
    let old = project(default);
    let new = project(close);
    assert!(old[2] < 0.0);
    assert!(new[2] > 0.0 && new[2] < new[3]);
    let uniform = close.to_uniform(16.0 / 9.0, false);
    assert_eq!(uniform.view_projection, close.view_projection(16.0 / 9.0));
}
#[test]
fn default_lens_preserves_existing_projection() {
    let camera = crate::render::camera::CameraState::new(0.5, 0.2, 4.0);
    assert_eq!(
        camera.view_projection(1.5),
        crate::render::camera::orbit_view_projection_target(
            camera.yaw,
            camera.pitch,
            camera.zoom,
            camera.target,
            1.5
        )
    );
}
