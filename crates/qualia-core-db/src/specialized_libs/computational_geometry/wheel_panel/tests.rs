use super::*;
fn recipe<'a>(axles: &'a [f32]) -> WheelPanel<'a> {
    WheelPanel {
        center_x: 0.84,
        thickness: 0.055,
        z: [-1.3, 1.3],
        bottom: 0.42,
        top: [0.85, 0.90],
        wheel_y: 0.29,
        clearance_radius: 0.39,
        axles,
        segments: 64,
    }
}
#[test]
fn closed_guard_has_raised_edges_over_each_wheel_and_finite_geometry() {
    let mut positions = vec![[0.0; 3]; 650];
    let mut triangles = vec![[0; 3]; 520];
    let receipt =
        wheel_clearance_panel_into(recipe(&[-0.77, 0.78]), &mut positions, &mut triangles).unwrap();
    assert!(receipt.triangle_count > 0);
    assert!(positions[..receipt.vertex_count]
        .iter()
        .flatten()
        .all(|v| v.is_finite()));
    for point in &positions[..receipt.vertex_count] {
        if (point[2] + 0.77).abs() < 0.03 || (point[2] - 0.78).abs() < 0.03 {
            assert!(point[1] > 0.67);
        }
    }
    assert!(receipt.min[1] <= 0.421);
    assert!(receipt.max[1] >= 0.899);
}
#[test]
fn bad_parameters_and_short_buffers_do_not_touch_outputs() {
    let mut positions = vec![[99.0; 3]; 650];
    let mut triangles = vec![[99; 3]; 520];
    let mut bad = recipe(&[0.0]);
    bad.top = [0.60; 2];
    assert!(wheel_clearance_panel_into(bad, &mut positions, &mut triangles).is_err());
    assert!(positions.iter().all(|v| *v == [99.0; 3]));
    assert!(triangles.iter().all(|v| *v == [99; 3]));
    assert!(
        wheel_clearance_panel_into(recipe(&[0.0]), &mut positions[..10], &mut triangles).is_err()
    );
    assert!(triangles.iter().all(|v| *v == [99; 3]));
}
