use super::*;

#[test]
fn pipe_model_host_exposes_branch_radii() {
    let input = args::record([
        (
            "parents",
            args::f64_list_value(vec![u32::MAX as f64, 0.0, 0.0]),
        ),
        ("tip_radius", Value::F64(1.0)),
        ("exponent", Value::F64(2.0)),
    ]);
    assert!(pipe_model_radii_host(&input, Span::new(0, 0)).is_ok());
}

#[test]
fn pipe_model_host_rejects_parent_out_of_range() {
    let input = args::record([
        ("parents", args::f64_list_value(vec![2.0])),
        ("tip_radius", Value::F64(1.0)),
        ("exponent", Value::F64(2.0)),
    ]);
    assert!(pipe_model_radii_host(&input, Span::new(0, 0)).is_err());
}

#[test]
fn miniball_host_accepts_two_points() {
    let input = args::record([(
        "points",
        args::f64_list_value(vec![0.0, 0.0, 0.0, 2.0, 0.0, 0.0]),
    )]);
    assert!(miniball_3d_host(&input, Span::new(0, 0)).is_ok());
}

#[test]
fn mesh_plane_slice_host_emits_intersection() {
    let input = args::record([
        (
            "vertices",
            args::f64_list_value(vec![-1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0]),
        ),
        ("triangles", args::f64_list_value(vec![0.0, 1.0, 2.0])),
        ("origin", args::f64_list_value(vec![0.0, 0.0, 0.0])),
        ("normal", args::f64_list_value(vec![1.0, 0.0, 0.0])),
    ]);
    assert!(mesh_plane_slice_host(&input, Span::new(0, 0)).is_ok());
}
